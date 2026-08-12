// Copyright 2026 The ethos-engine maintainers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Extraction: the evidence itself (`docs/05-MILESTONES.md` M3).
//!
//! Takes the **same [`Document`] handle** [`crate::classify`] takes. Nothing here reopens a file:
//! two loads can disagree, and a classifier that saw a different object graph from the extractor
//! is a silent divergence with no diagnostic.
//!
//! # Coordinates
//!
//! PDF user space has its origin at the bottom-left with y increasing upward. The artifact
//! declares top-left with y increasing downward ([`engine_core::CoordinateSystem::V0`]), and page
//! `/Rotate` is applied so geometry matches the declaration rather than the raw stream. The
//! transform is this engine's job and the declaration is how a reader knows it happened.

use engine_core::{
    quantize, ArtifactIdentity, Assurance, DerivationClass, EngineError, IdAllocator, IdKind,
    PageState, PageStateEntry, Profile, Sha256Hex, QUANTUM_PER_POINT,
};
use serde::{Deserialize, Serialize};

use crate::classify::SourceRef;
use crate::content::Interpreter;
use crate::document::Document;
use crate::fonts::{load_page_fonts, WidthSource};
use crate::limitations as lim;
use crate::nodes::{PageExtract, PdfLocator, SynthesisReason, SynthesizedChar, TextRun};

/// Artifact type for an extract. **DRAFT** — see `docs/draft-schemas/`.
pub const EXTRACT_ARTIFACT_TYPE: &str = "ethos.engine.extract.v0";

/// Shape version of the extract artifact. **DRAFT**.
pub const EXTRACT_SCHEMA_VERSION: &str = "0.1.0";

/// The extract artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractArtifact {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
    pub identity: ArtifactIdentity,
    /// The bytes this extract describes.
    pub source: SourceRef,
    /// Version id of the reading-order rule that ordered the runs.
    ///
    /// Mirrors `profile.reading_order_rule`, on the artifact so a reader need not fetch the
    /// profile to know which rule produced this order.
    pub reading_order_rule: String,
    /// Total pages.
    pub page_count: u32,
    /// Per-page results for the **processed** pages, in page order.
    ///
    /// A page missing from this list was not read; [`Assurance::page_states`] says which of the
    /// reasons applied. Its absence is never evidence that the page holds no text.
    pub pages: Vec<PageExtract>,
    /// Declared capabilities, limitations, per-page state, coverage, and terminal state.
    ///
    /// **The L1 gate** (`docs/01-CONTRACT.md` §7). Absorbs what M3 emitted as `not_decoded`:
    /// absent font widths and undescended form XObjects are now limitations in
    /// `assurance.limitations`, alongside the capability-derived ones — including the explicit
    /// multi-column reading-order limitation that `synthetic/two-columns` exists to pin.
    pub assurance: Assurance,
}

impl ExtractArtifact {
    /// Canonical bytes, via `engine-core`'s c14n.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact cannot be canonicalized — unreachable through
    /// the public API, since every field is an integer, string, bool, or enum.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        let value = serde_json::to_value(self).map_err(|e| EngineError::Malformed {
            what: "extract".into(),
            detail: e.to_string(),
        })?;
        engine_core::c14n_bytes(&value).map_err(|e| EngineError::Malformed {
            what: "extract".into(),
            detail: e.to_string(),
        })
    }

    /// Every run across every page, in reading order.
    pub fn runs(&self) -> impl Iterator<Item = &TextRun> {
        self.pages.iter().flat_map(|p| p.runs.iter())
    }

    /// Whether this artifact may be presented as a complete reading of its source.
    ///
    /// The question a consumer must ask before treating the run list as the whole document.
    /// False whenever any authorized page did not reach `Processed`.
    pub fn is_complete(&self) -> bool {
        self.assurance.is_complete()
    }
}

/// Extract text runs from an already-open document.
///
/// # Errors
///
/// - [`EngineError::Unsupported`] — an operator outside PDF 32000-1 Table A.1, or a character
///   code this profile cannot decode. **Fails closed**: a skipped operator can move or delete
///   text, and a substituted character is a character the document does not contain.
/// - [`EngineError::Malformed`] — operands of the wrong shape, or an unreadable page structure.
/// - [`EngineError::MissingPart`] — a font resource a `Tf` refers to is absent.
pub fn extract(doc: &Document, profile: &Profile) -> Result<ExtractArtifact, EngineError> {
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "profile".into(),
            detail: e.to_string(),
        })?;

    let mut alloc = IdAllocator::new(profile_sha256.clone());
    let mut pages = Vec::with_capacity(doc.pages().len());
    let mut limitations = lim::extract_limitations();
    let mut page_states: Vec<PageStateEntry> = Vec::with_capacity(doc.pages().len());

    let budget = profile.page_budget;
    let page_count = doc.page_count();

    for &(page_number, page_id) in doc.pages() {
        // The budget is checked before any work on the page, not after. A page counted as
        // quarantined must genuinely not have been read — otherwise the coverage summary
        // describes a run that did not happen.
        if !budget.admits(page_number) {
            page_states.push(PageStateEntry {
                index: page_number,
                state: PageState::Quarantined(engine_core::codes::RESOURCE_LIMIT_PAGES.to_string()),
            });
            continue;
        }

        let page_dict =
            doc.inner()
                .get_dictionary(page_id)
                .map_err(|e| EngineError::Malformed {
                    what: "page dictionary".into(),
                    detail: e.to_string(),
                })?;

        let geom = PageGeometry::resolve(doc, page_dict)?;
        let fonts = load_page_fonts(doc.inner(), page_dict)?;

        for font in fonts.values() {
            if let WidthSource::Absent { reason } = &font.widths {
                let entry = lim::font_widths_absent(reason);
                if !limitations.contains(&entry) {
                    limitations.push(entry);
                }
            }
        }

        let content = doc.inner().get_page_content(page_id);
        let decoded =
            lopdf::content::Content::decode(&content).map_err(|e| EngineError::Malformed {
                what: "content stream".into(),
                detail: format!("page {page_number}: {e}"),
            })?;

        let mut interp = Interpreter::new(&fonts);
        interp.run(&decoded.operations)?;

        let mut runs = Vec::with_capacity(interp.shown.len());
        for shown in &interp.shown {
            if shown.text.is_empty() {
                continue;
            }
            let font = fonts.get(&shown.font_id);

            let (ox_pt, oy_pt) = geom.to_top_left(shown.origin.0, shown.origin.1);
            let origin_x = quantize(ox_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            let origin_y = quantize(oy_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            let advance = shown
                .advance
                .map(|a| quantize(a, QUANTUM_PER_POINT))
                .transpose()
                .map_err(quantize_err)?;

            let geometry = match (font, shown.advance) {
                (Some(f), Some(w)) => f.ink_box(ox_pt, oy_pt, w, shown.font_size),
                // No advance means no width, so there is no box to measure — and a box guessed
                // from the font size is exactly what this project refuses.
                _ => engine_core::GeometryPresence::Absent(
                    engine_core::GeometryAbsence::NotReportedByReader,
                ),
            };

            let synthesized: Vec<SynthesizedChar> = shown
                .synthesized_indices
                .iter()
                .map(|i| SynthesizedChar {
                    char_index: *i,
                    reason: SynthesisReason::TjGap,
                })
                .collect();

            let scalar_code_mismatch = shown.text.chars().count() != shown.codes.len();

            runs.push(TextRun {
                id: alloc.next(IdKind::Span)?,
                text: shown.text.clone(),
                char_codes: shown.codes.clone(),
                scalar_code_mismatch,
                synthesized,
                font_id: shown.font_id.clone(),
                font_size: quantize(shown.font_size, QUANTUM_PER_POINT).map_err(quantize_err)?,
                locator: PdfLocator {
                    page: page_number,
                    origin_x,
                    origin_y,
                    advance,
                },
                geometry,
                mcid: shown.mcid,
                // Text and origins are read from the document's own encoding.
                derivation: DerivationClass::Extracted,
            });
        }

        pages.push(PageExtract {
            index: page_number,
            width: quantize(geom.display_width, QUANTUM_PER_POINT).map_err(quantize_err)?,
            height: quantize(geom.display_height, QUANTUM_PER_POINT).map_err(quantize_err)?,
            rotation: geom.rotation,
            runs,
        });
        // Reached only after the page's runs are in the artifact, so `Processed` cannot be
        // claimed for a page whose interpretation failed — that path returns `Err` above and
        // produces no artifact at all.
        page_states.push(PageStateEntry {
            index: page_number,
            state: PageState::Processed,
        });
    }

    if let Some(b) = budget.max_pages_to_process() {
        if b < page_count {
            limitations.push(lim::resource_limit_pages(b, page_count));
        }
    }

    Ok(ExtractArtifact {
        identity: ArtifactIdentity {
            artifact_type: EXTRACT_ARTIFACT_TYPE.to_string(),
            schema_version: EXTRACT_SCHEMA_VERSION.to_string(),
            parser_version: profile.parser_version.clone(),
            profile_sha256,
        },
        source: SourceRef {
            media_type: "application/pdf".to_string(),
            sha256: Sha256Hex::parse(doc.source_sha256().as_str())?,
        },
        reading_order_rule: profile.reading_order_rule.clone(),
        page_count,
        pages,
        assurance: Assurance::new(profile.capabilities, page_count, page_states, limitations)?,
    })
}

fn quantize_err(_: engine_core::QuantizeError) -> EngineError {
    EngineError::Malformed {
        what: "coordinate".into(),
        detail: "a coordinate is non-finite or outside the canonical integer range".into(),
    }
}

/// Page box and rotation, and the transform into the declared coordinate system.
struct PageGeometry {
    media_width: f64,
    media_height: f64,
    rotation: i64,
    display_width: f64,
    display_height: f64,
}

impl PageGeometry {
    fn resolve(doc: &Document, page_dict: &lopdf::Dictionary) -> Result<Self, EngineError> {
        // /MediaBox may be inherited; lopdf resolves inheritance for us where it can.
        let media = crate::fonts::resolve_array(doc.inner(), page_dict.get(b"MediaBox").ok())
            .or_else(|| {
                crate::fonts::resolve_dict(doc.inner(), page_dict.get(b"Parent").ok())
                    .and_then(|p| crate::fonts::resolve_array(doc.inner(), p.get(b"MediaBox").ok()))
            })
            .ok_or(EngineError::MissingPart {
                part: "/MediaBox".into(),
            })?;

        if media.len() != 4 {
            return Err(EngineError::Malformed {
                what: "/MediaBox".into(),
                detail: format!("expected four numbers, found {}", media.len()),
            });
        }
        let n = |i: usize| -> f64 {
            match &media[i] {
                lopdf::Object::Integer(v) => *v as f64,
                lopdf::Object::Real(v) => f64::from(*v),
                _ => 0.0,
            }
        };
        let (x0, y0, x1, y1) = (n(0), n(1), n(2), n(3));
        let media_width = (x1 - x0).abs();
        let media_height = (y1 - y0).abs();

        let rotation = page_dict
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0)
            .rem_euclid(360);
        if !matches!(rotation, 0 | 90 | 180 | 270) {
            return Err(EngineError::Malformed {
                what: "/Rotate".into(),
                detail: format!("{rotation} is not a multiple of 90"),
            });
        }

        // A quarter turn swaps the visible dimensions.
        let (display_width, display_height) = if rotation % 180 == 0 {
            (media_width, media_height)
        } else {
            (media_height, media_width)
        };

        Ok(Self {
            media_width,
            media_height,
            rotation,
            display_width,
            display_height,
        })
    }

    /// Map a user-space point into the declared top-left system, applying `/Rotate`.
    ///
    /// Derived by asking where each corner lands under a clockwise quarter turn, rather than by
    /// pattern-matching a formula: for `/Rotate 90` the bottom-left corner becomes the top-left,
    /// which fixes the mapping uniquely.
    fn to_top_left(&self, x: f64, y: f64) -> (f64, f64) {
        match self.rotation {
            90 => (y, x),
            180 => (self.media_width - x, y),
            270 => (self.media_height - y, self.media_width - x),
            // 0, and anything else is refused before reaching here.
            _ => (x, self.media_height - y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(w: f64, h: f64, rot: i64) -> PageGeometry {
        let (dw, dh) = if rot % 180 == 0 { (w, h) } else { (h, w) };
        PageGeometry {
            media_width: w,
            media_height: h,
            rotation: rot,
            display_width: dw,
            display_height: dh,
        }
    }

    #[test]
    fn unrotated_pages_flip_the_y_axis_only() {
        let g = geom(300.0, 144.0, 0);
        // Bottom-left in user space is the origin in a top-left system.
        assert_eq!(g.to_top_left(0.0, 144.0), (0.0, 0.0));
        // Top-left in user space is the bottom-left of the display.
        assert_eq!(g.to_top_left(0.0, 0.0), (0.0, 144.0));
        assert_eq!(g.to_top_left(72.0, 72.0), (72.0, 72.0));
    }

    /// The corner test that pins the 90° mapping.
    #[test]
    fn a_quarter_turn_sends_the_bottom_left_corner_to_the_top_left() {
        // MediaBox [0 0 144 300] with /Rotate 90 — the rotation-90 fixture's shape.
        let g = geom(144.0, 300.0, 90);
        assert_eq!(
            g.display_width, 300.0,
            "a quarter turn swaps the dimensions"
        );
        assert_eq!(g.display_height, 144.0);

        assert_eq!(
            g.to_top_left(0.0, 0.0),
            (0.0, 0.0),
            "bottom-left -> top-left"
        );
        assert_eq!(
            g.to_top_left(144.0, 0.0),
            (0.0, 144.0),
            "bottom-right -> bottom-left"
        );
        assert_eq!(
            g.to_top_left(0.0, 300.0),
            (300.0, 0.0),
            "top-left -> top-right"
        );
    }

    #[test]
    fn every_rotation_keeps_points_inside_the_display_box() {
        for rot in [0i64, 90, 180, 270] {
            let g = geom(144.0, 300.0, rot);
            for (x, y) in [
                (0.0, 0.0),
                (144.0, 0.0),
                (0.0, 300.0),
                (144.0, 300.0),
                (72.0, 150.0),
            ] {
                let (dx, dy) = g.to_top_left(x, y);
                assert!(
                    (0.0..=g.display_width).contains(&dx),
                    "rot {rot}: x {dx} outside 0..{}",
                    g.display_width
                );
                assert!(
                    (0.0..=g.display_height).contains(&dy),
                    "rot {rot}: y {dy} outside 0..{}",
                    g.display_height
                );
            }
        }
    }

    #[test]
    fn the_reading_order_rule_is_stream_order_at_v0() {
        // Single-column: runs come out in the order the content stream shows them, with no
        // reordering transform applied. The rule id lives on the profile so a future rule gets a
        // new id rather than replacing this one silently.
        assert_eq!(Profile::default().reading_order_rule, "single-column-v1");
    }
}
