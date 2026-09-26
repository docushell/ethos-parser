// Copyright 2026 The ethos-parser maintainers
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

//! The annotated overlay: what was detected, drawn on the document (v1-S6, checklist O10).
//!
//! A copy of the source PDF with annotations added over the geometry this engine found — table
//! boxes, image placements, and the runs it flagged. It is a **viewing aid for the artifact**, and
//! that is the whole of its claim.
//!
//! # It shows absence, not only presence
//!
//! O10's exit criterion is stricter than "draw the boxes": *the overlay distinguishes present
//! geometry from typed absence*. An overlay that draws only what it has looks like a fully-parsed
//! document — the reader sees six neat rectangles and concludes six things were found and nothing
//! was missed. So every page also carries a note counting the nodes on it that have **no**
//! rectangle to draw, and a page where nothing was found at all says so rather than coming back
//! looking untouched.
//!
//! # It is not a verdict, and it is not a redaction
//!
//! Nothing here scores, grades or decides. A flagged run is drawn *because it stays in the
//! artifact* — the overlay is how you look at evidence, never how you remove it. This is not
//! `--sanitize`: the source's own bytes are copied through, no content stream is edited, and no
//! text is deleted, blacked out or moved.
//!
//! # Determinism, and the one trap
//!
//! Byte identity holds **per fresh document**. `lopdf`'s writer mutates the document it saves —
//! `write_cross_reference_stream` increments `max_id` and rewrites trailer keys — so saving the
//! *same* instance twice produces two different files. [`build`] therefore parses the source
//! bytes and saves once, every time, and never caches a `Document` between writes. Nothing else
//! about the write is volatile: objects live in a `BTreeMap`, no `/ID` is generated, and lopdf's
//! only clock is behind a feature this build does not enable.

use std::collections::BTreeMap;

use ethos_parser_core::{EngineError, Profile, QRect};

use crate::document::Document;
use crate::extract::{ExtractArtifact, PageGeometry};

/// Artifact type for an overlay. **DRAFT**, like the others.
pub const OVERLAY_ARTIFACT_TYPE: &str = "ethos.parser.overlay.v0";

/// What an overlay annotation marks, and how it is drawn.
///
/// Colours are chosen only to be distinguishable, and they carry **no severity**. A flagged run is
/// not "worse" than a table; nothing in this engine grades anything, and an overlay that used red
/// for "bad" would be smuggling a verdict in through the palette.
struct Mark {
    subtype: &'static str,
    colour: [f64; 3],
    label: &'static str,
}

const TABLE: Mark = Mark {
    subtype: "Square",
    colour: [0.15, 0.35, 0.75],
    label: "table",
};
const IMAGE: Mark = Mark {
    subtype: "Square",
    colour: [0.10, 0.55, 0.35],
    label: "image",
};
const FLAGGED_RUN: Mark = Mark {
    subtype: "Square",
    colour: [0.85, 0.45, 0.05],
    label: "flagged text run",
};

/// Build an overlay PDF from a document and the extract taken from it.
///
/// `doc` must be the same document `extract` describes; the binding is asserted on the digest
/// rather than assumed, because an overlay drawn over a *different* file would put this engine's
/// findings on a page that never produced them.
///
/// # Errors
///
/// - [`EngineError::Malformed`] if the extract does not describe this document, or if the source
///   bytes will not re-parse.
/// - [`EngineError::Encrypted`] if the empty user password opened the document: the copy would
///   carry neither its encryption nor its permissions.
/// - [`EngineError::Unsupported`] with `what` = `overlay` if a dictionary carries `/ByteRange`: a
///   digital signature would no longer cover the bytes it signed; or if an object or the trailer
///   holds a real outside `f32`'s range, which `lopdf`'s writer prints as `inf`.
pub fn build_overlay(
    doc: &Document,
    extract: &ExtractArtifact,
    profile: &Profile,
) -> Result<Vec<u8>, EngineError> {
    if extract.source.sha256.as_str() != doc.source_sha256().as_str() {
        return Err(EngineError::Malformed {
            what: "overlay".into(),
            detail: format!(
                "this extract describes {} and the document is {} — an overlay drawn over a \
                 different file would put findings on a page that never produced them",
                extract.source.sha256.as_str(),
                doc.source_sha256().as_str()
            ),
        });
    }
    crate::tagging::refuse_rewrite(doc, "overlay")?;

    // **A fresh copy per call, never the handle itself and never a cached one.** lopdf's writer
    // MUTATES the document it saves — `write_cross_reference_stream` increments `max_id` and
    // rewrites trailer keys — so saving one instance twice produces two different files. Cloning
    // here is what makes two independent builds byte-identical, and it is also what keeps the
    // open handle every other stage is borrowing untouched.
    let mut out = doc.inner().clone();

    let pages: BTreeMap<u32, lopdf::ObjectId> = out.get_pages();
    let mut added: BTreeMap<u32, Vec<lopdf::Object>> = BTreeMap::new();

    for page in &extract.pages {
        let Some(&page_id) = pages.get(&page.index) else {
            continue;
        };
        let geom = {
            let dict = out
                .get_dictionary(page_id)
                .map_err(|e| EngineError::Malformed {
                    what: "overlay page".into(),
                    detail: e.to_string(),
                })?
                .clone();
            PageGeometry::resolve(doc, &dict)?
        };

        let mut annots: Vec<lopdf::Object> = Vec::new();
        // How many nodes on this page have no rectangle for the overlay to draw. Counted rather
        // than passed over — see the module header.
        let mut without_geometry: u32 = 0;

        // A tagged table's grid is the document's own tags and it carries no
        // geometry (v2-S24), so it cannot be drawn — which is exactly what this
        // count exists to report. It was iterating `page.tables` alone, so the note
        // said a page with two tagged tables had no tables and nothing undrawable.
        without_geometry += u32::try_from(page.tagged_tables.len()).unwrap_or(u32::MAX);

        for table in &page.tables {
            annots.push(square(
                &geom,
                table.rect.as_qrect_checked()?,
                &TABLE,
                &table.id,
            ));
        }

        for image in &page.images {
            match image.locator.rect.painted() {
                Some(r) => annots.push(square(&geom, r, &IMAGE, &image.id)),
                // A rotated or unexpressible placement. The image was found; its area cannot be
                // drawn as a rectangle, and drawing its bounding box would claim page area the
                // picture does not cover.
                None => without_geometry += 1,
            }
        }

        for run in &page.runs {
            if run.findings.is_empty() {
                continue;
            }
            match run.geometry.measured() {
                Some(r) => annots.push(square(&geom, r, &FLAGGED_RUN, &run.id)),
                // The overwhelmingly common case: most fonts supply no metrics, so most runs have
                // no measured ink box. A flagged run with nowhere to draw it is exactly the
                // absence O10 asks the overlay not to hide.
                None => without_geometry += 1,
            }
        }

        annots.push(page_note(&geom, page, without_geometry));
        added.insert(page.index, annots);
    }

    // Annotations are appended to whatever the page already had. **The document's own annotations
    // are not touched**: a form's widgets and a reviewer's comments are content, and an overlay
    // that replaced them would be editing the document rather than annotating it.
    for (index, mut new_annots) in added {
        let Some(&page_id) = pages.get(&index) else {
            continue;
        };
        let mut existing: Vec<lopdf::Object> = out
            .get_dictionary(page_id)
            .ok()
            .and_then(|d| d.get(b"Annots").ok().cloned())
            .and_then(|o| match o {
                lopdf::Object::Array(a) => Some(a),
                lopdf::Object::Reference(r) => out.get_object(r).ok()?.as_array().ok().cloned(),
                _ => None,
            })
            .unwrap_or_default();
        existing.append(&mut new_annots);
        if let Ok(dict) = out.get_dictionary_mut(page_id) {
            dict.set("Annots", lopdf::Object::Array(existing));
        }
    }

    stamp_identity(&mut out, doc, extract, profile);

    let mut bytes = Vec::new();
    out.save_to(&mut bytes)
        .map_err(|e| EngineError::Malformed {
            what: "overlay".into(),
            detail: e.to_string(),
        })?;
    Ok(bytes)
}

/// One rectangle, in PDF user space, as a `/Square` annotation.
///
/// # Integer points, deliberately
///
/// lopdf writes reals through `f32`'s `Display`, so `1/3` becomes `0.33333334` and the exact
/// spelling is a property of a float formatter rather than of this engine. Since
/// `docs/01-CONTRACT.md` keeps floats off the wire everywhere else, the overlay rounds **outward**
/// to whole points: the drawn box always contains the measured one, and never claims a smaller
/// area than was found.
fn square(
    geom: &PageGeometry,
    rect: QRect,
    mark: &Mark,
    id: &ethos_parser_core::NodeId,
) -> lopdf::Object {
    let q = f64::from(ethos_parser_core::QUANTUM_PER_POINT);
    let (ax, ay) = geom.to_user_space(rect.x0() as f64 / q, rect.y0() as f64 / q);
    let (bx, by) = geom.to_user_space(rect.x1() as f64 / q, rect.y1() as f64 / q);
    let (x0, x1) = (ax.min(bx).floor(), ax.max(bx).ceil());
    let (y0, y1) = (ay.min(by).floor(), ay.max(by).ceil());

    let mut d = lopdf::Dictionary::new();
    d.set("Type", lopdf::Object::Name(b"Annot".to_vec()));
    d.set(
        "Subtype",
        lopdf::Object::Name(mark.subtype.as_bytes().to_vec()),
    );
    d.set("Rect", int_rect(x0, y0, x1, y1));
    d.set(
        "C",
        lopdf::Object::Array(
            mark.colour
                .iter()
                .map(|c| lopdf::Object::Real(*c as f32))
                .collect(),
        ),
    );
    // Print flag off, so the overlay is a screen annotation rather than something that lands on
    // paper as though the document had contained it.
    d.set("F", lopdf::Object::Integer(0));
    d.set(
        "Contents",
        lopdf::Object::string_literal(format!("ethos-parser: {} {}", mark.label, id.as_str())),
    );
    lopdf::Object::Dictionary(d)
}

/// The per-page note: what was found here, and what could not be drawn.
///
/// **Its rectangle is the page's own top-left corner and means nothing about location.** A note
/// has to sit somewhere, and there is no honest place to put a marker for content whose position
/// is precisely what is unknown — so it says so in its own text rather than being anchored
/// somewhere that could be mistaken for a claim.
fn page_note(geom: &PageGeometry, page: &crate::nodes::PageExtract, without: u32) -> lopdf::Object {
    let (cx, cy) = geom.to_user_space(0.0, 0.0);
    let mut d = lopdf::Dictionary::new();
    d.set("Type", lopdf::Object::Name(b"Annot".to_vec()));
    d.set("Subtype", lopdf::Object::Name(b"Text".to_vec()));
    d.set(
        "Rect",
        int_rect(
            cx.floor(),
            (cy - 20.0).floor(),
            (cx + 20.0).ceil(),
            cy.ceil(),
        ),
    );
    d.set("F", lopdf::Object::Integer(0));
    d.set("Name", lopdf::Object::Name(b"Note".to_vec()));
    let flagged = page.runs.iter().filter(|r| !r.findings.is_empty()).count();
    d.set(
        "Contents",
        lopdf::Object::string_literal(format!(
            "ethos-parser overlay, page {}: {} run(s), {} table(s), {} image(s), {} flagged \
             run(s). {} marked item(s) on this page have NO rectangle this overlay can draw — a \
             run whose font supplies no ink metrics, an image placed by a matrix that is not \
             axis-aligned, or a tagged table, whose grid is the document's own tags and which \
             carries no geometry at all. They were found and they are in the artifact; what is \
             missing is a box, not the evidence. This note's own position is the page corner and is not a claim \
             about where anything is.",
            page.index,
            page.runs.len(),
            page.tables.len() + page.tagged_tables.len(),
            page.images.len(),
            flagged,
            without
        )),
    );
    lopdf::Object::Dictionary(d)
}

fn int_rect(x0: f64, y0: f64, x1: f64, y1: f64) -> lopdf::Object {
    lopdf::Object::Array(
        [x0, y0, x1, y1]
            .iter()
            .map(|v| lopdf::Object::Integer(*v as i64))
            .collect(),
    )
}

/// Record what this overlay is and what it was drawn from, inside the file itself.
///
/// An overlay that did not say which document, which profile and which build produced it would be
/// a PDF with some rectangles on it — indistinguishable from a reviewer's markup, and impossible
/// to check against the artifact it came from.
fn stamp_identity(
    out: &mut lopdf::Document,
    doc: &Document,
    extract: &ExtractArtifact,
    profile: &Profile,
) {
    let mut id = lopdf::Dictionary::new();
    id.set(
        "ArtifactType",
        lopdf::Object::string_literal(OVERLAY_ARTIFACT_TYPE),
    );
    id.set(
        "SourceSha256",
        lopdf::Object::string_literal(doc.source_sha256().as_str()),
    );
    id.set(
        "ProfileSha256",
        lopdf::Object::string_literal(extract.identity.profile_sha256.as_str()),
    );
    id.set(
        "ParserVersion",
        lopdf::Object::string_literal(profile.parser_version.clone()),
    );
    id.set(
        "ObservationRule",
        lopdf::Object::string_literal(profile.observation_rule.clone()),
    );
    let Ok(catalog) = out.catalog_mut() else {
        return;
    };
    catalog.set("EthosParserOverlay", lopdf::Object::Dictionary(id));
}

#[cfg(test)]
mod tests {
    /// **A tagged table is counted, both as a table and as undrawable.**
    ///
    /// Since v2-S24 a tagged table is a first-class record carrying absent
    /// geometry, so it is exactly the case this note exists to disclose: found,
    /// in the artifact, and impossible to draw. The loops read `page.tables`
    /// alone, so on `irs-f1040sd-2025` page 1 the note read "0 table(s) … 0
    /// marked item(s) have NO rectangle" about a page holding two of them. A
    /// reader doing what the module header asks — using the note to tell a
    /// missing box from a missed node — was told the page had neither.
    #[test]
    fn the_page_note_counts_tagged_tables_as_undrawable() {
        let source = include_str!("overlay.rs");
        assert!(
            source.contains("without_geometry += u32::try_from(page.tagged_tables.len())"),
            "tagged tables must reach the undrawable count"
        );
        assert!(
            source.contains("page.tables.len() + page.tagged_tables.len()"),
            "the note's table count must cover both table populations"
        );
        assert!(
            source.contains("or a tagged table, whose grid is the document's own tags"),
            "the note's prose must name the third cause it now counts"
        );
    }

    /// The overlay marks, and never edits.
    ///
    /// A source scan, because the difference between an overlay and a redactor is exactly which
    /// operations appear in this file. `--sanitize` is out of scope for this project entirely, and
    /// the way it arrives by accident is somebody reaching for a content-stream edit here.
    #[test]
    fn the_overlay_never_edits_the_document_it_draws_on() {
        let src = include_str!("overlay.rs");
        let end = src.find("#[cfg(test)]").unwrap_or(src.len());
        let code: String = src[..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code.contains("save_to"),
            "the scan did not reach the writer, so its claims below prove nothing"
        );
        for banned in [
            "set_content",
            "remove_annot",
            "delete_object",
            "Contents\", lopdf::Object::Stream",
            "sanitize",
            "redact",
        ] {
            assert!(
                !code.contains(banned),
                "`{banned}` reached the overlay. This artifact ANNOTATES a document; it does not \
                 edit one. Removing or rewriting content here would make a scrubbed document and \
                 a clean one produce the same file, which is the defect the findings exist to \
                 prevent"
            );
        }
    }
}
