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
pub const EXTRACT_SCHEMA_VERSION: &str = "0.3.0";

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

/// Resolve one run's structural address (v1-S3).
///
/// The precedence is the point, so it is stated rather than left to fall out of the `if`s:
///
/// 1. **Artifact wins.** The page itself said this content is furniture and outside the structure
///    tree (§14.8.2.2). That is the document's own statement about its own content, and it
///    outranks anything a join could conclude. The run stays in the artifact — flagged, never
///    dropped.
/// 2. **A tree citation binds.** Only on exact `(page, mcid)` equality. Nothing fuzzy, nothing
///    nearest-match: an mcid means one thing on one page, and a looser join would file text under
///    a heading that does not claim it.
/// 3. **Otherwise the bare id**, exactly as v0 emitted it. This is a *smaller* claim than a bound
///    role path, and the difference is preserved rather than smoothed over.
/// 4. **Otherwise nothing**, because the page marked nothing here.
fn bind_structure(
    tree: Option<&crate::structure::StructureTree>,
    page: lopdf::ObjectId,
    mcid: Option<i64>,
    artifact: bool,
) -> Option<engine_core::StructuralLocator> {
    use engine_core::{PdfArtifactLocator, StructuralLocator};

    if artifact {
        return Some(StructuralLocator::PdfArtifact(PdfArtifactLocator { mcid }));
    }
    let mcid = mcid?;
    match tree.and_then(|t| t.locator_for(page, mcid)) {
        Some(found) => Some(StructuralLocator::PdfTagged(found.clone())),
        None => Some(StructuralLocator::PdfMcid(mcid)),
    }
}

/// Flip an annotation rectangle into the artifact's declared coordinate system (v1-S4).
///
/// The same transform a glyph origin goes through, for the same reason: geometry that lived in a
/// different coordinate system from the text around it would be uncheckable by construction.
/// A rectangle that will not quantize becomes `Malformed` rather than a guess — and never a
/// page-sized box.
fn to_top_left_rect(
    geom: &PageGeometry,
    rect: engine_core::AnnotationRect,
) -> engine_core::AnnotationRect {
    use engine_core::AnnotationRect;

    let Some(r) = rect.declared() else {
        return rect;
    };
    let q = f64::from(QUANTUM_PER_POINT);
    let (ax, ay) = geom.to_top_left(r.x0() as f64 / q, r.y0() as f64 / q);
    let (bx, by) = geom.to_top_left(r.x1() as f64 / q, r.y1() as f64 / q);
    let to_q = |v: f64| quantize(v, QUANTUM_PER_POINT).ok();
    match (to_q(ax), to_q(ay), to_q(bx), to_q(by)) {
        (Some(x0), Some(y0), Some(x1), Some(y1)) => {
            match engine_core::QRect::new(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)) {
                Ok(r) => AnnotationRect::Declared(r),
                Err(_) => AnnotationRect::Malformed,
            }
        }
        _ => AnnotationRect::Malformed,
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
    // A repaired open is never silent: every artifact derived from one says so.
    if let Some(padded) = doc.xref_entries_padded() {
        limitations.push(lim::xref_entry_padded(padded));
    }
    let mut page_states: Vec<PageStateEntry> = Vec::with_capacity(doc.pages().len());
    // Accumulated across pages: how much text is missing from this artifact because a font's
    // encoding could not map it, and the first failure's reason for the declaration's detail.
    let mut encoding_dropped_runs: u32 = 0;
    let mut encoding_detail = String::new();
    // v1-S2. Pages where the alignment rule built a candidate lattice and refused it, with the
    // precondition that failed. Collected rather than declared per page so the artifact carries
    // one limitation naming every such page instead of one per page.
    let mut unruled_refusals: Vec<(u32, crate::unruled::Refusal)> = Vec::new();

    // v1-S3. Read the document's own structure tree ONCE, off the same handle every other stage
    // borrows (`docs/04-ARCHITECTURE.md` §2.1). `None` means the catalog declares no
    // `/StructTreeRoot` — an untagged document, which is an answer rather than a failure.
    let structure = crate::structure::read(doc.inner())?;
    // Counted while binding, declared afterwards, and only when non-zero.
    let mut mcids_unbound: u32 = 0;
    let mut unclaimed_tree_items: u32 = 0;
    let mut props_by_name: u32 = 0;
    let mut tagged_without_geometric: Vec<u32> = Vec::new();
    // v1-S4. Widgets whose `/Parent` chain did not resolve. Counted, declared, never repaired.
    let mut unresolved_field_parents: u32 = 0;

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

        // v0.1: a font that cannot map a code drops its run rather than failing the document.
        // Accumulated across pages so the artifact declares one honest total.
        encoding_dropped_runs = encoding_dropped_runs.saturating_add(interp.dropped_runs);
        if interp.dropped_runs > 0 {
            if let Some(first) = interp.undecodable.first() {
                if encoding_detail.is_empty() {
                    encoding_detail = format!("page {page_number}: {first}");
                }
            }
        }

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
                // v1-S3. The join against the document's structure tree, or the honest lesser
                // answer when the tree does not reach this run. `bind_structure` never invents:
                // an unbound id stays an unbound id.
                structural: bind_structure(structure.as_ref(), page_id, shown.mcid, shown.artifact),
                // Text and origins are read from the document's own encoding.
                derivation: DerivationClass::Extracted,
            });
        }

        // Which of the tree's citations this page's runs actually answered. A cited pair that no
        // run claims is a real hole — the tree says there is content there and the content stream
        // did not mark any — and it is counted rather than filled with a fabricated run.
        if let Some(tree) = structure.as_ref() {
            for &(pg, mcid) in tree.keys() {
                if pg == page_id && !runs.iter().any(|r| r.mcid == Some(mcid)) {
                    unclaimed_tree_items += 1;
                }
            }
        }
        mcids_unbound += runs
            .iter()
            .filter(|r| {
                matches!(
                    r.structural,
                    Some(engine_core::StructuralLocator::PdfMcid(_))
                )
            })
            .count() as u32;
        props_by_name = props_by_name.saturating_add(interp.props_by_name);

        // v1-S1: ruled tables, from the rectangles this page actually painted. Rects arrive in
        // user space and go through the SAME transform and quantum as a glyph origin — a table
        // whose geometry lived in a different coordinate system from the text inside it would be
        // uncheckable by construction.
        let mut table_rects = Vec::with_capacity(interp.rects.len());
        for r in &interp.rects {
            let (ax, ay) = geom.to_top_left(r.x0, r.y0);
            let (bx, by) = geom.to_top_left(r.x1, r.y1);
            table_rects.push(crate::tables::quantize_rect(ax, ay, bx, by)?);
        }
        let origins: Vec<crate::tables::RunOrigin<'_>> = runs
            .iter()
            .map(|r| crate::tables::RunOrigin {
                x: r.locator.origin_x,
                y: r.locator.origin_y,
                text: r.text.as_str(),
            })
            .collect();
        // v1-S2: ruled first, then the alignment rule on whatever text no ruled table claims.
        let detected = crate::tables::detect(page_number, &table_rects, &origins, &mut alloc)?;
        let mut tables = detected.tables;

        // v1-S3: the document's own tags, compared against what the detectors found. The two
        // derivations meet here and nowhere else — the tree walk never saw a box, and neither
        // detector ever saw a structure type.
        //
        // Paired by position: the nth `/Table` the tree describes on this page against the nth
        // table found on it. Anything cleverer would be matching two grids by geometry, and the
        // tagged half has no geometry to match with.
        if let Some(tree) = structure.as_ref() {
            let tagged_here: Vec<&crate::structure::TaggedTable> = tree
                .tables
                .iter()
                .filter(|t| t.page == Some(page_id))
                .collect();
            for (i, tagged) in tagged_here.iter().enumerate() {
                match tables.get_mut(i) {
                    Some(found) => {
                        let positions: Vec<engine_core::TableCellPosition> =
                            found.cells.iter().map(|c| c.position.clone()).collect();
                        found.tagged_check =
                            Some(tagged.check_against(found.rows, found.columns, &positions));
                    }
                    // The tree says there is a table here and no detector found one. **No table
                    // is invented to match the tags**: a grid emitted on the strength of `/TD`
                    // elements alone would have cells this engine placed, and a consumer could
                    // not tell them from cells a detector reconstructed from the page.
                    None => tagged_without_geometric.push(page_number),
                }
            }
        }
        // A refused candidate is recorded once per page it happened on. Without this, a near-miss
        // page and a page with no grid-shaped text at all would both say `tables: []`, and only
        // one of them means "the alignment rule looked at something and decided against it".
        if let Some(r) = detected.refusal {
            unruled_refusals.push((page_number, r));
        }
        drop(origins);

        // v1-S4. Annotations and form fields, from the page's own `/Annots`. Walked here rather
        // than from `/AcroForm` downward because a field's node needs a page and a field
        // dictionary does not name one — its widget does, by being on that page.
        //
        // **The interpreter above never saw these.** Their text comes from dictionaries; nothing
        // in the content stream draws it, and nothing here feeds it back into `runs`.
        let mut objects = Vec::new();
        if profile.capabilities.form_fields || profile.capabilities.annotations {
            let found = crate::forms::read_page_objects(doc.inner(), page_dict);
            unresolved_field_parents =
                unresolved_field_parents.saturating_add(found.unresolved_parents);

            for object in found.objects {
                let is_field = matches!(object.detail, crate::forms::PageObjectDetail::Field(_));
                // A profile with one capability off still reads the other. The two are separate
                // claims, so turning one off must not silently take the other with it.
                if is_field && !profile.capabilities.form_fields {
                    continue;
                }
                if !is_field && !profile.capabilities.annotations {
                    continue;
                }

                objects.push(crate::nodes::PageObjectRecord {
                    id: alloc.next(if is_field {
                        IdKind::FormField
                    } else {
                        IdKind::Annotation
                    })?,
                    locator: engine_core::PdfObjectLocator {
                        page: page_number,
                        object: object.id.0,
                        generation: u32::from(object.id.1),
                        // The `/Rect` arrives in user space and goes through the SAME transform
                        // and quantum as a glyph origin. A rectangle in a different coordinate
                        // system from the text around it would be uncheckable by construction.
                        rect: to_top_left_rect(&geom, object.rect),
                    },
                    text: object.text,
                    attributes: match object.detail {
                        crate::forms::PageObjectDetail::Field(a) => {
                            engine_core::NodeAttributes::FormField(a)
                        }
                        crate::forms::PageObjectDetail::Annotation(a) => {
                            engine_core::NodeAttributes::Annotation(a)
                        }
                    },
                    // v1-S4 decision 7: the structure tree may cite a widget by object
                    // reference. S3 walked `/OBJR` and bound nothing; this is where that
                    // binding would land. No role is invented when the tree is silent.
                    structural: structure
                        .as_ref()
                        .and_then(|t| t.locator_for_object(object.id))
                        .map(|l| engine_core::StructuralLocator::PdfTagged(l.clone())),
                });
            }
        }

        pages.push(PageExtract {
            tables,
            objects,
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

    // v1-S2. Only when a candidate was actually built and refused — a page whose text implied
    // nothing grid-shaped produced no candidate and gets no declaration, because declaring a
    // refusal that did not happen is as misleading as omitting one that did.
    if !unruled_refusals.is_empty() {
        limitations.push(lim::unruled_candidate_refused(&unruled_refusals));
    }

    // v1-S3. Four facts about the structure tree, each declared only where it is true. A
    // capability that says "this profile looks" is worth having only if the artifact also says
    // what the looking found, and "found nothing" has more than one cause.
    match structure.as_ref() {
        None => limitations.push(lim::untagged_structure_tree_absent()),
        Some(tree) => {
            if mcids_unbound > 0 {
                limitations.push(lim::structure_mcid_unbound(mcids_unbound));
            }
            if unclaimed_tree_items > 0 {
                limitations.push(lim::structure_item_without_content(unclaimed_tree_items));
            }
            let _ = tree;
        }
    }
    if props_by_name > 0 {
        limitations.push(lim::mcid_property_list_by_name(props_by_name));
    }
    if !tagged_without_geometric.is_empty() {
        limitations.push(lim::tagged_table_without_geometric_table(
            &tagged_without_geometric,
        ));
    }

    // v1-S4. An XFA packet is DETECTED and declared, never parsed (checklist L15). Any static
    // AcroForm fields beside it are still read, which is why this is a limitation rather than a
    // refusal — but a sparse field set on such a document must not read as "this form is blank".
    if (profile.capabilities.form_fields || profile.capabilities.annotations)
        && crate::forms::has_xfa(doc.inner())
    {
        limitations.push(lim::xfa_forms_not_extracted());
    }
    if unresolved_field_parents > 0 {
        limitations.push(lim::form_field_parent_unresolved(unresolved_field_parents));
    }

    // **Encoding holes: declare, or refuse outright.**
    //
    // Some text decoded and some did not — say so, and say how much is missing. But a document
    // that showed text and decoded *none* of it has no usable text layer, and an artifact
    // carrying zero runs would be indistinguishable from a genuinely blank page. That is the
    // one case where refusing is the honest answer (`docs/01-CONTRACT.md` §8).
    if encoding_dropped_runs > 0 {
        let any_text = pages.iter().any(|p| !p.runs.is_empty());
        if !any_text {
            return Err(EngineError::Unsupported {
                what: "text encoding".into(),
                detail: format!(
                    "this document's text layer is unusable: {encoding_dropped_runs} run(s) were \
                     shown and none could be decoded, because no font supplied a `/ToUnicode` \
                     CMap or an encoding this profile can map. No artifact is emitted — an \
                     artifact with zero runs would be indistinguishable from a blank page, and \
                     substituting `U+FFFD` would put characters in the evidence that the \
                     document does not contain. First failure: {encoding_detail}"
                ),
            });
        }
        limitations.push(lim::broken_font_encoding(
            encoding_dropped_runs,
            &encoding_detail,
        ));
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
