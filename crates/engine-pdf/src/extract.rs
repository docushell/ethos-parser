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
    PageState, PageStateEntry, Profile, Sha256Hex, TextFinding, QUANTUM_PER_POINT,
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
    // v1-S6. Counted across pages, declared once, never repaired and never silently skipped.
    let mut inline_images: u32 = 0;
    let mut unresolved_xobjects: u32 = 0;
    let mut findings_seen: std::collections::BTreeMap<&'static str, u32> =
        std::collections::BTreeMap::new();

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
        // v1-S6. The frame an off-page finding is measured against, in the same coordinate system
        // the runs end up in. Computed once per page rather than per run.
        let visible = geom.visible_in_display_space();
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

        // v1-S6. The page's `/XObject` names, so `Do` can be resolved to an object number. The
        // interpreter still never holds a `Document` — it gets names and ids, and extraction
        // sorts `/Image` from `/Form` where the document is already in scope.
        let xobjects = crate::images::page_xobjects(doc.inner(), page_dict);
        let mut interp = Interpreter::new(&fonts).with_xobjects(xobjects);
        interp.run(&decoded.operations)?;
        inline_images = inline_images.saturating_add(interp.inline_images);
        unresolved_xobjects = unresolved_xobjects.saturating_add(interp.unresolved_xobjects);

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
                // v1-S6. Observations about the run, never a reason to withhold it. Both are
                // computed from evidence the page supplies: the text rendering mode the content
                // stream set, and the visible box the page declares.
                findings: run_findings(shown.render_mode, origin_x, origin_y, &visible),
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

        // v1-S5. **Reading order, and the only order there is.**
        //
        // Runs are in content-stream order at this point. The rule reads the page's whitespace
        // and says what order a human reads it in; `reorder_page` then makes the run list *be*
        // that order — array position, span id and ordinal all together, so nothing downstream
        // has to consult a second index to know what comes first. A parallel `reading_order`
        // field beside a stream-ordered array would be two answers to one question, which is the
        // defect this slice is here to avoid rather than introduce.
        //
        // **After detection, deliberately.** Both detectors read origins, not sequence, so
        // neither cares — but a table's box is what makes its runs one atom, and it does not
        // exist until detection has accepted one. Running the rule first would let a cut fall
        // through a grid before anything knew it was a grid.
        if profile.capabilities.multi_column_reading_order {
            let geometry: Vec<crate::reading_order::RunGeometry> = runs
                .iter()
                .map(|r| crate::reading_order::RunGeometry {
                    x: r.locator.origin_x,
                    y: r.locator.origin_y,
                    advance: r.locator.advance,
                })
                .collect();
            let boxes: Vec<crate::tables::QuantRect> = tables.iter().map(|t| t.rect).collect();
            reorder_page(
                &mut runs,
                &mut tables,
                &crate::reading_order::order(&geometry, &boxes),
            );
        }

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

        // v1-S6. One node per `Do`, in the order the page painted them. A `/Form` yields
        // nothing here — this profile does not descend into form XObjects, which stays declared
        // as `form-xobject-text-not-descended` — and neither does an XObject with no readable
        // `/Subtype`: emitting a node for an unlabelled stream would put a picture on the wire
        // the document never called one.
        let mut images = Vec::new();
        if profile.capabilities.images {
            for placement in &interp.images {
                let Some(attributes) =
                    crate::images::image_attributes(doc.inner(), placement.object)
                else {
                    continue;
                };
                let corners = placement.corners.map(|(x, y)| geom.to_top_left(x, y));
                images.push(crate::nodes::ImageRecord {
                    id: alloc.next(IdKind::Image)?,
                    locator: engine_core::PdfImageLocator {
                        page: page_number,
                        object: placement.object.0,
                        generation: u32::from(placement.object.1),
                        rect: crate::images::painted_rect(corners),
                    },
                    attributes,
                });
            }
        }

        for run in &runs {
            for f in &run.findings {
                *findings_seen.entry(f.as_code()).or_insert(0) += 1;
            }
        }

        pages.push(PageExtract {
            tables,
            objects,
            images,
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

    // v1-S6. Findings are counted where they were observed and declared once, so a consumer
    // reading only the assurance block learns they exist. **Every counted run is still in the
    // artifact** — this is a summary of what is there, never a record of what was removed.
    for (code, count) in &findings_seen {
        limitations.push(lim::text_finding(code, *count));
    }
    if inline_images > 0 {
        limitations.push(lim::inline_images_not_emitted(inline_images));
    }
    if unresolved_xobjects > 0 {
        limitations.push(lim::xobject_name_unresolved(unresolved_xobjects));
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

/// Put one page's runs into reading order, and put its identity there with them (v1-S5).
///
/// `order[i]` is the stream index of the run that belongs at position `i`.
///
/// # One order, not an order and an index
///
/// Three things move together here, and the point is that they cannot come apart:
///
/// 1. **The array.** `runs` ends up in reading order, so a consumer that iterates it reads the
///    document. There is no second field saying "…but actually read them like this".
/// 2. **The span ids.** The allocator handed out `s1…sN` down the content stream; they are
///    re-laid over the permuted list so `s1` is the first run a human should read. Since the
///    allocator's span counter is independent of the table and object counters, re-laying the
///    same contiguous ids in the new positions is **identical** to having allocated them after
///    the reorder — which is what the slice asks for, without moving table and annotation ids
///    that have nothing to do with reading order. `ordinal` is array position, assigned in
///    `crate::represent`, so it follows for free and stays monotone.
/// 3. **`DetectedCell::run_indices`.** These address the page's run list, and the list just
///    moved, so they are remapped. Left alone they would silently point at whatever run now
///    occupies the old slot — a cell claiming text it does not contain, which is the one failure
///    here that no artifact would show.
///
/// A table's runs are contiguous in the new order and keep their relative sequence (they are one
/// atom), so a cell's remapped indices stay ascending and still concatenate to the `text` the
/// detector built. `cell_text_survives_the_reordering` is the proof over real fixtures.
fn reorder_page(
    runs: &mut Vec<TextRun>,
    tables: &mut [crate::tables::DetectedTable],
    order: &[usize],
) {
    // The single-column case, which is most pages: the rule found no gutter and returned the
    // identity. Returning early is not just an optimization — it is the assertion that such a
    // page is byte-identical to what v0 emitted, because nothing at all happened to it.
    if order.iter().enumerate().all(|(i, &old)| i == old) {
        return;
    }

    let ids: Vec<engine_core::NodeId> = runs.iter().map(|r| r.id.clone()).collect();
    let mut slot: Vec<Option<TextRun>> = runs.drain(..).map(Some).collect();
    let mut position = vec![0usize; slot.len()];

    for (new, &old) in order.iter().enumerate() {
        position[old] = new;
        let mut run = slot[old]
            .take()
            .expect("`order` is a permutation, so no index is visited twice");
        run.id = ids[new].clone();
        runs.push(run);
    }

    for table in tables {
        for cell in &mut table.cells {
            for i in &mut cell.run_indices {
                *i = position[*i];
            }
            cell.run_indices.sort_unstable();
        }
    }
}

/// What was observed about one run, beyond its text (v1-S6).
///
/// Both findings come from evidence the page itself supplies — the rendering mode its content
/// stream set, and the box it declares as visible. Neither is an appearance judgement, and
/// neither removes anything: the run this describes is in the artifact with its text and its
/// origin intact, which is the whole of checklist O21.
fn run_findings(
    render_mode: i64,
    origin_x: i64,
    origin_y: i64,
    visible: &PageBox,
) -> Vec<TextFinding> {
    let mut out = Vec::new();
    // Modes 3 and 7 fill nothing and stroke nothing (32000-1 Table 106). Mode 7 also adds the
    // glyphs to the clip path, which is a different purpose and the same visible result: no ink.
    // Mode 4 through 6 DO paint and are deliberately not flagged — a rule that called every
    // clipping mode invisible would report ordinary text as hidden.
    if render_mode == 3 || render_mode == 7 {
        out.push(TextFinding::InvisibleRenderMode);
    }
    // The origin, because the origin is the run's address and the one coordinate this engine
    // treats as identity. An ink box would be a better test and most runs do not have one.
    let q = f64::from(QUANTUM_PER_POINT);
    let (x, y) = (origin_x as f64 / q, origin_y as f64 / q);
    if x < visible.x0 || x > visible.x1 || y < visible.y0 || y > visible.y1 {
        out.push(TextFinding::OffPage);
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn quantize_err(_: engine_core::QuantizeError) -> EngineError {
    EngineError::Malformed {
        what: "coordinate".into(),
        detail: "a coordinate is non-finite or outside the canonical integer range".into(),
    }
}

/// A page box, normalised so the low corner really is the low corner.
///
/// PDF permits either diagonal — `[612 792 0 0]` describes the same page as `[0 0 612 792]`
/// (32000-1 §7.9.5) — so every box is normalised on the way in and nothing downstream has to
/// wonder which corner it holds.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PageBox {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl PageBox {
    fn from_corners(a: f64, b: f64, c: f64, d: f64) -> Self {
        Self {
            x0: a.min(c),
            y0: b.min(d),
            x1: a.max(c),
            y1: b.max(d),
        }
    }

    fn width(self) -> f64 {
        self.x1 - self.x0
    }

    fn height(self) -> f64 {
        self.y1 - self.y0
    }

    /// The part of `self` that `other` also covers, or `None` when they do not overlap.
    ///
    /// A `/CropBox` reaching outside the `/MediaBox` is clipped to it, per 32000-1 §14.11.2:
    /// the visible page is the intersection, not whichever box is larger.
    fn intersect(self, other: Self) -> Option<Self> {
        let r = Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        };
        (r.x1 > r.x0 && r.y1 > r.y0).then_some(r)
    }
}

/// Page boxes and rotation, and the transform into the declared coordinate system.
pub(crate) struct PageGeometry {
    /// The page's `/MediaBox`, normalised. **Its origin is load-bearing** — see
    /// [`PageGeometry::to_top_left`].
    media: PageBox,
    /// The box a viewer actually shows: `/CropBox` clipped to the media box, or the media box
    /// where the page declares no crop box (v1-S6).
    ///
    /// Read for one reason: an off-page finding has to be measured against the box content is
    /// *visible* in, and that is this one. Measuring against `/MediaBox` on a page that crops
    /// would report ordinary trimmed content as off-page — a fabricated finding, which is worse
    /// than no finding at all.
    visible: PageBox,
    rotation: i64,
    display_width: f64,
    display_height: f64,
}

impl PageGeometry {
    pub(crate) fn resolve(
        doc: &Document,
        page_dict: &lopdf::Dictionary,
    ) -> Result<Self, EngineError> {
        let media =
            Self::box_from(doc, page_dict, b"MediaBox").ok_or(EngineError::MissingPart {
                part: "/MediaBox".into(),
            })?;

        // v1-S6. The visible box: `/CropBox` clipped to the media box, or the media box where no
        // crop box is declared. A crop box that does not overlap the media box at all describes
        // nothing visible, and rather than emit an empty page geometry the media box stands —
        // the document contradicted itself and the larger, always-present box is the safer of the
        // two answers.
        let visible = Self::box_from(doc, page_dict, b"CropBox")
            .and_then(|c| c.intersect(media))
            .unwrap_or(media);

        // `/Rotate` is inheritable (32000-1 Table 30), and its value may be an indirect
        // reference or a real. Reading it with `as_i64()` off the page dictionary alone made a
        // `/Rotate 90` on the `/Pages` node — or `/Rotate 90 0 R` — silently mean zero, and a
        // page rotated by a wrong amount puts every coordinate in the wrong place.
        let rotation = Self::inherited(doc, page_dict, b"Rotate")
            .and_then(
                |o| match crate::fonts::resolve_object(doc.inner(), Some(&o)) {
                    Some(lopdf::Object::Integer(v)) => Some(v),
                    Some(lopdf::Object::Real(v)) => Some(f64::from(v) as i64),
                    _ => None,
                },
            )
            .unwrap_or(0)
            .rem_euclid(360);
        if !matches!(rotation, 0 | 90 | 180 | 270) {
            return Err(EngineError::Malformed {
                what: "/Rotate".into(),
                detail: format!("{rotation} is not a multiple of 90"),
            });
        }

        // A quarter turn swaps the visible dimensions. Taken from the VISIBLE box, because that
        // is the page a reader sees and the box every coordinate here is expressed against.
        let (display_width, display_height) = if rotation % 180 == 0 {
            (visible.width(), visible.height())
        } else {
            (visible.height(), visible.width())
        };

        Ok(Self {
            media,
            visible,
            rotation,
            display_width,
            display_height,
        })
    }

    /// A page box by name, normalised, resolving indirect references and one `/Parent` hop.
    fn box_from(doc: &Document, page_dict: &lopdf::Dictionary, key: &[u8]) -> Option<PageBox> {
        let array = crate::fonts::resolve_array(
            doc.inner(),
            Self::inherited(doc, page_dict, key).as_ref(),
        )?;
        if array.len() != 4 {
            return None;
        }
        let n = |i: usize| -> Option<f64> {
            match crate::fonts::resolve_object(doc.inner(), array.get(i))? {
                lopdf::Object::Integer(v) => Some(v as f64),
                lopdf::Object::Real(v) => Some(f64::from(v)),
                _ => None,
            }
        };
        let (a, b, c, d) = (n(0)?, n(1)?, n(2)?, n(3)?);
        if !(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite()) {
            return None;
        }
        let r = PageBox::from_corners(a, b, c, d);
        (r.width() > 0.0 && r.height() > 0.0).then_some(r)
    }

    /// A key from the page dictionary, or from an ancestor that declares it.
    ///
    /// Bounded at [`INHERITANCE_MAX_DEPTH`] hops. An unbounded walk over a `/Parent` chain a
    /// document controls is a hang a document can cause, and a cycle is a document this reader
    /// must survive rather than spin on.
    fn inherited(
        doc: &Document,
        page_dict: &lopdf::Dictionary,
        key: &[u8],
    ) -> Option<lopdf::Object> {
        if let Ok(v) = page_dict.get(key) {
            return Some(v.clone());
        }
        let mut node = crate::fonts::resolve_dict(doc.inner(), page_dict.get(b"Parent").ok())?;
        for _ in 0..INHERITANCE_MAX_DEPTH {
            if let Ok(v) = node.get(key) {
                return Some(v.clone());
            }
            node = crate::fonts::resolve_dict(doc.inner(), node.get(b"Parent").ok())?;
        }
        None
    }

    /// Map a user-space point into the declared top-left system, applying `/Rotate`.
    ///
    /// Derived by asking where each corner lands under a clockwise quarter turn, rather than by
    /// pattern-matching a formula: for `/Rotate 90` the bottom-left corner becomes the top-left,
    /// which fixes the mapping uniquely.
    ///
    /// # The box origin is subtracted, and that was a repair (v1-S6)
    ///
    /// Through v1-S5 this used only the box's *width and height* and threw its origin away, so a
    /// page whose `/MediaBox` is `[0 20 612 812]` — a legal and not unusual box — had every
    /// coordinate in the artifact shifted by 20 points, with the top of the page landing at
    /// `y = -20`. Nothing caught it because **not one document in either corpus declares a box
    /// whose origin is other than `(0, 0)`**, measured across all 67 PDFs available to this
    /// repository; on such a page `x0 = y0 = 0` and the subtraction below is the identity, which
    /// is why this change moves no existing golden.
    ///
    /// It had to be fixed before v1-S6 could ship an off-page finding at all: a bounds test
    /// against a frame the content is systematically offset from reports ordinary text at the top
    /// of a page as off-page, and a **fabricated** security finding is worse than no finding.
    fn to_top_left(&self, x: f64, y: f64) -> (f64, f64) {
        let b = self.media;
        match self.rotation {
            90 => (y - b.y0, x - b.x0),
            180 => (b.x1 - x, y - b.y0),
            270 => (b.y1 - y, b.x1 - x),
            // 0, and anything else is refused before reaching here.
            _ => (x - b.x0, b.y1 - y),
        }
    }

    /// Map a point in the declared top-left system back into PDF user space (v1-S6).
    ///
    /// The exact inverse of [`Self::to_top_left`], and it exists for one caller: the annotated
    /// overlay, which holds rectangles in the artifact's coordinate system and has to write them
    /// into a PDF, where annotations are in user space. Deriving the inverse here rather than at
    /// the call site keeps the two transforms in one place, where a test can check that composing
    /// them is the identity.
    pub(crate) fn to_user_space(&self, x: f64, y: f64) -> (f64, f64) {
        let b = self.media;
        match self.rotation {
            90 => (y + b.x0, x + b.y0),
            180 => (b.x1 - x, y + b.y0),
            270 => (b.x1 - y, b.y1 - x),
            _ => (x + b.x0, b.y1 - y),
        }
    }

    /// The visible page box, mapped into the declared top-left system (v1-S6).
    ///
    /// Both corners go through [`Self::to_top_left`] and are then normalised, because a rotation
    /// can exchange which corner is which — deriving the frame from the same transform the
    /// content goes through is what makes the comparison meaningful.
    fn visible_in_display_space(&self) -> PageBox {
        let (ax, ay) = self.to_top_left(self.visible.x0, self.visible.y0);
        let (bx, by) = self.to_top_left(self.visible.x1, self.visible.y1);
        PageBox::from_corners(ax, ay, bx, by)
    }
}

/// How far a `/Parent` chain is walked for an inheritable page attribute.
///
/// Same reasoning and the same shape as the structure walk's depth bound: a chain the document
/// controls must not decide how long this process runs, and a cycle must be survived rather than
/// spun on.
const INHERITANCE_MAX_DEPTH: usize = 32;

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(w: f64, h: f64, rot: i64) -> PageGeometry {
        let (dw, dh) = if rot % 180 == 0 { (w, h) } else { (h, w) };
        let media = PageBox::from_corners(0.0, 0.0, w, h);
        PageGeometry {
            media,
            visible: media,
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

    /// The rule id the default profile names, and the one it no longer does (v1-S5).
    ///
    /// v0 named `single-column-v1`, meaning content-stream order with nothing reordered. The new
    /// rule got a **new id** rather than a bump of that one, because the old string still has a
    /// true meaning and a profile that turns the capability off still uses it. Pinning both here
    /// is what stops a later slice from quietly redefining either.
    #[test]
    fn the_reading_order_rule_is_the_gutter_rule_and_not_the_v0_id() {
        assert_eq!(
            Profile::default().reading_order_rule,
            engine_core::READING_ORDER_RULE_V1
        );
        assert_eq!(
            engine_core::READING_ORDER_RULE_V1,
            "gutter-columns-v1",
            "the id is data on every artifact; changing the string is an identity event"
        );
        assert_ne!(
            engine_core::READING_ORDER_RULE_V1,
            engine_core::READING_ORDER_RULE_V0,
            "two rules that order runs differently must not share an id"
        );
        assert_eq!(engine_core::READING_ORDER_RULE_V0, "single-column-v1");
    }
}
