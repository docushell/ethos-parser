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

//! The limitations a PDF profile declares (`docs/01-CONTRACT.md` §7).
//!
//! `engine-core` derives one limitation per `false` capability, which covers everything a
//! caller can read off the capability set. This module owns the rest: the gaps that exist
//! because of **this backend, this vendored data, and this format** — none of which
//! `engine-core` is allowed to know about (`docs/04-ARCHITECTURE.md` §1).
//!
//! # These replace M2's `not_detected` and M3's `not_decoded`
//!
//! Both were stand-ins, and both said so in their own doc comments. Two ad-hoc lists beside a
//! capability block is two vocabularies a consumer has to learn and reconcile, and the second
//! one is always the one nobody reads. There is now one list, one shape, and one place to look:
//! `assurance.limitations`.
//!
//! # A limitation code is a promise
//!
//! Callers match on these strings, so they are kebab-case, stable, and renamed only as a schema
//! change. The prose attached to each is stable enough to assert on for the same reason: a
//! declaration nobody can match against is barely better than no declaration.

use engine_core::Limitation;

use crate::thresholds::NOT_DETECTED;

/// Classification samples a bounded number of pages and stops.
pub const CLASSIFY_SAMPLE_BOUND: &str = "classify-sample-bound";

/// The backend refuses xref entries that are not exactly 20 bytes.
pub const BACKEND_XREF_STRICT_20_BYTE: &str = "backend-xref-strict-20-byte";

/// The Adobe predefined CJK CMaps are not vendored, so a document naming one is refused.
pub const PREDEFINED_CMAPS_NOT_VENDORED: &str = "predefined-cmaps-not-vendored";

/// Text drawn inside a form XObject is not descended into.
pub const FORM_XOBJECT_TEXT_NOT_DESCENDED: &str = "form-xobject-text-not-descended";

/// A font supplied no usable widths, so advances are absent rather than guessed.
pub const FONT_WIDTHS_ABSENT: &str = "font-widths-absent";

/// This document's cross-reference table was padded from 19-byte entries to the specified 20.
///
/// Declared on every artifact produced from a repaired open. New at v0.1 — see
/// `crate::xref` for the repair and `docs/01-CONTRACT.md` §12 for the decision.
pub const XREF_ENTRY_PADDED: &str = "xref-entry-padded";

/// A font's encoding could not map every code, so the affected runs were dropped, not guessed.
///
/// New at v0.1. The alternative every other reader takes is a substitution character, which puts
/// text in the evidence that the document does not contain.
pub const BROKEN_FONT_ENCODING: &str = "broken-font-encoding";

/// The code declaring that a reason in the classification vocabulary is never emitted.
///
/// Derived from the reason's own wire spelling rather than hand-written, so the declaration and
/// the vocabulary cannot drift into disagreeing about what a reason is called.
pub fn undetected_reason_code(reason: &str) -> String {
    format!("{reason}-reason-not-detected")
}

/// Limitations true of **any** run under a PDF profile, whatever the document holds.
///
/// Always declared, on both artifacts. A backend that refuses one document in twenty-six is not
/// a property of the twenty-five it accepted, and a reader of a successful artifact still needs
/// to know which documents this engine would have turned away.
pub fn backend_limitations() -> Vec<Limitation> {
    vec![
        Limitation::profile(
            BACKEND_XREF_STRICT_20_BYTE,
            "This backend requires the exactly-20-byte cross-reference entries PDF 32000-1 \
             §7.5.4 mandates and refuses 19-byte ones (`0000000015 00000 n\\n`, missing the \
             trailing space). **v0.1 decided repair-or-refuse in favour of a bounded repair**: \
             where the table is the last structure in the file, `startxref` names it, no `/Prev` \
             chain exists, and every in-use offset precedes the table, the entries are padded to \
             the specified width and the document is parsed normally — and the artifact declares \
             `xref-entry-padded`. Any document failing one of those preconditions still exits 2 \
             with a named malformed-structure error and produces no artifact at all; it is never \
             partially read. The repair is recorded in the profile as `xref_repair`, so an \
             artifact from a repairing build is correctly non-comparable with one from a build \
             that refuses. Other malformations are not repaired: this is one bounded class, not \
             PDFium-style general recovery.",
        ),
        Limitation::profile(
            PREDEFINED_CMAPS_NOT_VENDORED,
            "The Adobe predefined CJK CMaps are not vendored. A document whose font names one \
             is refused with a named error rather than decoded approximately, because a \
             substituted character is a character the document does not contain. If those files \
             land, `cmap_data_version` changes and artifacts from before and after become \
             correctly non-comparable.",
        ),
    ]
}

/// Limitations specific to classification.
///
/// Absorbs M2's `not_detected` list. The honesty it encoded is unchanged and now travels in the
/// same shape as everything else: a vocabulary that lists `garbled` while no detector can ever
/// produce it invites a caller to read an empty reason list as evidence the document is not
/// garbled — absence of a reason mistaken for evidence of its absence.
pub fn classify_limitations(sample_pages: u32) -> Vec<Limitation> {
    let mut out = backend_limitations();

    out.push(Limitation::profile(
        CLASSIFY_SAMPLE_BOUND,
        format!(
            "Classification samples at most {sample_pages} page(s) and stops; cost must not \
             scale with total page count. Pages past the bound are reported `not_attempted` — \
             they were never observed, which is not the same as observed and empty. A caller \
             needing every page must raise the sample count, which changes `profile_sha256`."
        ),
    ));

    for (reason, why) in NOT_DETECTED {
        out.push(Limitation::profile(
            &undetected_reason_code(reason),
            format!(
                "The `{reason}` reason code is in this profile's vocabulary and is never \
                 emitted by it. {why} An empty reason list is therefore not evidence that \
                 `{reason}` does not apply to this document."
            ),
        ));
    }

    out
}

/// Limitations specific to extraction.
///
/// Absorbs M3's `not_decoded` list. `font-widths-absent` is document-scoped and appended by the
/// extractor when it actually occurs; the rest are true of every run.
pub fn extract_limitations() -> Vec<Limitation> {
    let mut out = backend_limitations();

    out.push(Limitation::profile(
        FORM_XOBJECT_TEXT_NOT_DESCENDED,
        "Text drawn inside a form XObject (via `Do`) is not descended into. The operator is \
         acknowledged rather than skipped silently, but a document relying on form XObjects for \
         its text will under-report runs. Declared here so a short run list is read as a \
         declared gap rather than as a sparse page.",
    ));

    out
}

/// The document-scoped limitation for a font that supplied no usable widths.
pub fn font_widths_absent(detail: &str) -> Limitation {
    Limitation::document(
        FONT_WIDTHS_ABSENT,
        format!(
            "A font on this document supplies no usable width information, so affected runs \
             report an ABSENT advance rather than a guessed one — and no ink box is measured \
             for them, since a box with no width would have to be invented. The origin is \
             unaffected: it comes from the content stream. Detail: {detail}"
        ),
    )
}

/// The document-scoped limitation for an open that needed the cross-reference repair.
///
/// Document-scoped rather than profile-scoped, deliberately: the *policy* is on the profile
/// (`xref_repair`) and is declared on every artifact, while this says something about **this
/// document** — that it was malformed and what was done about it. A reader must be able to tell
/// "the repair was available" from "the repair fired here".
pub fn xref_entry_padded(entries_padded: u32) -> Limitation {
    Limitation::document(
        XREF_ENTRY_PADDED,
        crate::xref::repaired_detail(entries_padded),
    )
}

/// The document-scoped limitation for a font whose encoding could not map every code.
///
/// The runs that could not be decoded are **absent from the artifact**, and their count is here.
/// Never a substitution character: `U+FFFD` in an evidence artifact is a character the document
/// does not contain, and downstream nothing can tell it from one that does.
pub fn broken_font_encoding(runs_dropped: u32, detail: &str) -> Limitation {
    Limitation::document(
        BROKEN_FONT_ENCODING,
        format!(
            "{runs_dropped} text run(s) contained character codes THIS PROFILE could not map — \
             neither the font's `/ToUnicode` CMap nor its simple encoding produced a character for \
             them. Those runs are OMITTED from this artifact rather than decoded approximately: no \
             `U+FFFD`, no best-guess glyph, and nothing dropped silently. Text that IS decodable on \
             the same page is present and unaffected, with exact origins. A consumer must read this \
             document's text as incomplete, and must not infer from a run's absence that the page \
             is blank there.\n\n\
             **This does not say the document is at fault, and until v1-S6.1 it did.** The earlier \
             wording opened `A font on this document has an incomplete or damaged encoding`, which \
             was a false statement about conformant files: the reader was splitting simple fonts' \
             single-byte codes into pairs, so codes that were never in the document arrived here \
             unmappable, and 8 417 runs were dropped from one 28-page corpus document under that \
             sentence. A declaration that misattributes is worse than none, because a reader acts \
             on it — someone would have gone to fix a document with nothing wrong with it. What \
             this code reports now is what can actually be established: a code arrived, and this \
             profile had no character for it. Whether the cause is a damaged font, an encoding this \
             profile does not vendor, or a defect in this reader is NOT decided here. Detail: \
             {detail}"
        ),
    )
}

/// The document-scoped limitation for pages where the alignment rule refused a candidate grid.
///
/// **The near-miss disclosure** (`docs/09-V1-MILESTONES.md` S2, decision 7). Columns that almost
/// line up must not become a table — but a reader who sees only `tables: []` cannot tell that
/// case from a page with nothing grid-shaped on it at all. This says which pages the alignment
/// rule looked hard at, and which precondition each one failed.
///
/// **Not a confidence field.** It reports that a refusal happened and names the rule that
/// refused. It never grades how close the candidate came, because a "0.8 table" is exactly the
/// number `docs/01-CONTRACT.md` §9 forbids.
pub fn unruled_candidate_refused(refusals: &[(u32, crate::unruled::Refusal)]) -> Limitation {
    let mut detail = String::from(
        "On some pages the alignment rule built a candidate grid from the text's own positions \
         and REFUSED it, so no table was emitted there. This is the difference between `no grid \
         was implied here` and `a grid was implied and judged incoherent`, and only the second \
         one is reported below. Nothing was repaired, relaxed or partially emitted: a candidate \
         either satisfies every precondition of `unruled-align-v1` or it produces no table. \
         Refused by page:",
    );
    for (page, r) in refusals {
        detail.push_str(&format!("\n  - page {page}: {}", r.detail()));
    }
    Limitation::document(engine_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED, detail)
}

/// Pages where the **ruled** rule built a candidate lattice and refused it (v1-S7b).
///
/// The companion to [`unruled_candidate_refused`], and it exists for the same reason: without it,
/// a page where rectangles implied a grid that their own ink did not draw is indistinguishable
/// from a page that painted nothing. Both say `tables: []`.
pub fn ruled_candidate_refused(refusals: &[(u32, crate::tables::RuledRefusal)]) -> Limitation {
    let mut detail = format!(
        "On some pages the rectangles the document painted implied a grid and the ruled rule \
         REFUSED it, so no table was emitted there. This is the difference between `nothing \
         grid-shaped was drawn here` and `a grid was implied and judged incoherent`, and only the \
         second one is reported below. Nothing was repaired or partially emitted: a candidate \
         either satisfies every precondition of `{}` or it produces no table.",
        engine_core::TABLE_DETECTION_V3
    );
    // **Grouped by precondition, so the reasoning is stated once.** The ruled rule refuses 481 of
    // `nist-sp-800-53r5`'s 492 pages; repeating a five-line explanation per page would put a
    // quarter of a megabyte of identical prose inside a hashed artifact. Every page is still
    // named with its own numbers, so nothing is truncated — only the prose is de-duplicated.
    let mut kinds: std::collections::BTreeMap<&'static str, (String, Vec<String>)> =
        std::collections::BTreeMap::new();
    for (page, r) in refusals {
        let e = kinds
            .entry(r.kind())
            .or_insert_with(|| (r.explanation(), Vec::new()));
        e.1.push(format!("page {page} ({})", r.detail()));
    }
    for (kind, (explanation, pages)) in kinds {
        detail.push_str(&format!(
            "\n\n  {kind}: {explanation}\n  Refused on {} page(s): {}",
            pages.len(),
            pages.join(", ")
        ));
    }
    Limitation::document(engine_core::codes::RULED_TABLE_CANDIDATE_REFUSED, detail)
}

/// Pages where the **stroke-ruled** rule built a candidate band and refused it (v1-S8).
///
/// The third companion to [`unruled_candidate_refused`] and [`ruled_candidate_refused`], grouped
/// by precondition for the same reason the ruled one is: this rule builds a candidate on any page
/// that draws two rows of lines ending at common x positions, so a per-page copy of a five-line
/// explanation would put more prose about refusals into the artifact than there is text on the
/// pages. Every page is still named with its own numbers; only the reasoning is de-duplicated.
pub fn stroke_ruled_candidate_refused(
    refusals: &[(u32, crate::stroke_ruled::Refusal)],
) -> Limitation {
    let mut detail = String::from(
        "On some pages the ruling lines the document stroked implied a grid and the stroke-ruled \
         rule REFUSED it, so no table was emitted there. This is the difference between `nothing \
         grid-shaped was ruled here` and `a grid was implied and judged incoherent`, and only the \
         second one is reported below. Nothing was repaired or partially emitted: a candidate \
         either satisfies every precondition of `stroke-ruled-v1` or it produces no table.",
    );
    let mut kinds: std::collections::BTreeMap<&'static str, (String, Vec<String>)> =
        std::collections::BTreeMap::new();
    for (page, r) in refusals {
        let e = kinds
            .entry(r.kind())
            .or_insert_with(|| (r.explanation(), Vec::new()));
        e.1.push(format!("page {page} ({})", r.detail()));
    }
    for (kind, (explanation, pages)) in kinds {
        detail.push_str(&format!(
            "\n\n  {kind}: {explanation}\n  Refused on {} page(s): {}",
            pages.len(),
            pages.join(", ")
        ));
    }
    Limitation::document(
        engine_core::codes::STROKE_RULED_TABLE_CANDIDATE_REFUSED,
        detail,
    )
}

/// The document-scoped limitation for a file that carries no tagged-structure tree.
///
/// **The honest answer for an untagged document**, and the reason
/// `capabilities.structural_locators: true` is a claim about *looking* rather than about finding.
/// This profile read the catalog, found no `/StructTreeRoot`, and invented nothing — no role
/// deduced from a font size, no table inferred from a `"Table 3:"` prefix (parity checklist P14).
pub fn untagged_structure_tree_absent() -> Limitation {
    Limitation::document(
        engine_core::codes::UNTAGGED_STRUCTURE_TREE_ABSENT,
        "This document's catalog declares no `/StructTreeRoot`, so it carries no tagged-structure \
         tree and NO ROLE PATH EXISTS to report. Marked-content ids are still captured verbatim \
         where the content stream supplies them, and they are still not structural addresses — an \
         id with no tree to resolve it against names nothing. Nothing is inferred to fill the gap: \
         a heading guessed from a type size, or a table from a caption's wording, would be \
         indistinguishable on the wire from structure the author actually wrote, which is worse \
         than reporting none.",
    )
}

/// The document-scoped limitation for marked content the structure tree never claims.
///
/// Distinct from an untagged document: here there **is** a tree and it does not reach this
/// content. The runs keep their bare marked-content ids and gain no role path.
pub fn structure_mcid_unbound(runs: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::STRUCTURE_MCID_UNBOUND,
        format!(
            "{runs} text run(s) carry a marked-content id that NO structure element in this \
             document's tree claims. This document is tagged; its tree simply does not reach that \
             content. Those runs are present and complete, with exact origins, and carry their \
             marked-content id alone rather than a role path — the id is the join key, and a join \
             that found nothing produces no address. No nearest-match was attempted: binding a run \
             to a role path the tree did not give it would file text under a heading that does not \
             claim it."
        ),
    )
}

/// The document-scoped limitation for tree citations no run answered.
///
/// The mirror of [`structure_mcid_unbound`]. Never filled in: a node standing in for
/// cited-but-absent content would be text this engine authored.
pub fn structure_item_without_content(items: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::STRUCTURE_ITEM_WITHOUT_CONTENT,
        format!(
            "This document's structure tree cites {items} marked-content item(s) that NO run on \
             the named page carried. The tree says there is content there and the content stream \
             did not mark any. Counted rather than filled: an empty node standing in for cited \
             content would be a node this engine authored, and a consumer could not tell it from \
             one the document produced. A reader must not conclude from this that those pages are \
             blank — only that the tree and the content stream disagree about what is marked."
        ),
    )
}

/// The document-scoped limitation for `BDC` property lists supplied by name.
///
/// An **unread** id and an **absent** id are different facts, and only the second means "this
/// content is outside the structure tree".
pub fn mcid_property_list_by_name(sequences: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::MCID_PROPERTY_LIST_BY_NAME,
        format!(
            "{sequences} marked-content sequence(s) supplied their property list as a NAME \
             indirecting through the page's `/Properties` resource (PDF 32000-1 §14.6.2) rather \
             than inline. This profile does not resolve that indirection, so any `/MCID` in those \
             property lists went unread and their content can look unmarked when it is not. \
             Declared rather than silently treated as `no id`: an id this reader did not resolve \
             is not the same as an id the document did not write."
        ),
    )
}

/// The document-scoped limitation for a tagged table no detector found.
///
/// **No table is invented to match the tags.** A grid emitted on the strength of `/TD` elements
/// alone would have cells this engine placed rather than cells reconstructed from the page, and a
/// consumer could not tell the two apart. The tree's claim is reported instead, so the gap is
/// visible without being filled (`docs/09-V1-MILESTONES.md` S3, decision 7).
pub fn tagged_table_without_geometric_table(pages: &[u32]) -> Limitation {
    let list: Vec<String> = pages.iter().map(u32::to_string).collect();
    Limitation::document(
        engine_core::codes::TAGGED_TABLE_WITHOUT_GEOMETRIC_TABLE,
        format!(
            "This document's structure tree describes a `/Table` on page(s) {} that NO table \
             detector found — the page paints no grid of rectangles there and its text implies no \
             coherent alignment lattice. The tree's claim is recorded here and NO table is \
             emitted for it: cells placed from `/TD` elements alone would be cells this engine \
             positioned, indistinguishable on the wire from cells reconstructed off the page \
             itself. The text is present and complete either way, with exact origins, and each \
             run carries the role path the tree gave it — so the table's content is addressable \
             even though its grid is not.",
            list.join(", ")
        ),
    )
}

/// The document-scoped limitation for an XFA packet this profile does not parse (v1-S4).
///
/// **Detected, declared, never parsed** (checklist L15). XFA is an XML form description living
/// beside — or instead of — the static AcroForm, and reading it is a different format inside a
/// PDF wrapper.
pub fn xfa_forms_not_extracted() -> Limitation {
    Limitation::document(
        engine_core::codes::XFA_FORMS_NOT_EXTRACTED,
        "This document's `/AcroForm` declares an `/XFA` packet: an XML form description this \
         profile does NOT parse. Static AcroForm fields alongside it are still read and emitted, \
         but a dynamic XFA form keeps its real field set and values in that packet — so a sparse \
         or empty set of field nodes on this document must NOT be read as `this form is blank`. \
         Parsing XFA is reading a second format inside a PDF wrapper, with its own escaping, its \
         own versions and its own failure modes; doing it badly would produce field values that \
         look exactly like ones read from the file's own dictionaries.",
    )
}

/// The document-scoped limitation for a widget whose parent field would not resolve (v1-S4).
///
/// **The repair that is not performed.** LiteParse repairs orphaned widgets in memory and always
/// flattens; this engine emits the widget with what it declares about itself and says the link was
/// broken. A repair nobody recorded hands the reader a document other than the one they have.
pub fn form_field_parent_unresolved(widgets: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::FORM_FIELD_PARENT_UNRESOLVED,
        format!(
            "{widgets} widget annotation(s) name a `/Parent` field that could not be resolved — \
             a missing object, or a `/Parent` chain that loops. Those widgets ARE emitted, \
             carrying whatever they declare about themselves; what is missing is the part of \
             their fully-qualified name, type or value that only an ancestor held. NOTHING WAS \
             REPAIRED: no parent was inferred from position, no `/Kids` array was rewritten, and \
             the source bytes are untouched. A field name here may therefore be shorter than the \
             form intends, or absent — and that is visible rather than papered over."
        ),
    )
}

/// The document-scoped limitation for a run stopped by the configured page budget.
pub fn resource_limit_pages(budget: u32, page_count: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::RESOURCE_LIMIT_PAGES,
        format!(
            "A configured page budget of {budget} stopped this run before the document's \
             {page_count} pages were exhausted. Pages past the budget are `quarantined`: they \
             exist, they were never read, and nothing may be concluded about their contents. \
             The artifact's terminal state is `partial` for exactly this reason and must not be \
             presented as a complete reading of the source."
        ),
    )
}

/// A text finding, counted and declared (v1-S6).
///
/// **Document-scoped, because it is a fact about this document** rather than about the profile.
/// The runs it counts are all still in the artifact, with their text, their origins and their
/// place in reading order — this is a summary so that a consumer reading the assurance block
/// learns the observation exists without diffing node lists. It is not a record of anything
/// removed, because nothing was removed.
pub fn text_finding(code: &str, count: u32) -> Limitation {
    let detail = match code {
        engine_core::codes::INVISIBLE_RENDER_MODE_TEXT => format!(
            "{count} text run(s) were drawn in an INVISIBLE rendering mode — `Tr 3` or `Tr 7`, \
             which fill nothing and stroke nothing (PDF 32000-1 Table 106). Every one of them is \
             in this artifact, with its text and its origin, and NONE was removed: text that is \
             invisible to a human reader is still text a machine reads, and deleting it would \
             make a document that hides an instruction indistinguishable from one that says \
             nothing. This profile does not decide what it is looking at. The same mode carries a \
             scanner's OCR layer under a page image, which is ordinary and useful, and a prompt \
             hidden behind a picture, which is not — and nothing in the content stream tells the \
             two apart. A consumer with context this engine does not have decides; the engine \
             reports."
        ),
        engine_core::codes::OFF_PAGE_TEXT => format!(
            "{count} text run(s) have an origin OUTSIDE the page's visible box — outside \
             `/CropBox` where the page declares one, outside `/MediaBox` otherwise, measured \
             after `/Rotate`. They are in the file and not on the page. Every one is in this \
             artifact and none was removed, for the reason invisible text is not removed: content \
             a reader cannot see is still content a machine reads. Ordinary causes exist — \
             printer's marks, a trimmed bleed area, an editing remnant — and this profile does \
             not distinguish them from deliberate concealment, because the geometry does not."
        ),
        other => format!("{count} run(s) carry the `{other}` observation."),
    };
    Limitation::document(code, detail)
}

/// Inline images were drawn and are not nodes (v1-S6).
pub fn inline_images_not_emitted(count: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::INLINE_IMAGES_NOT_EMITTED,
        format!(
            "{count} inline image(s) (`BI` … `ID` … `EI`) were drawn on this document and are NOT \
             emitted as image nodes. An inline image carries its samples in the content stream \
             itself, so it has no object number to address it by and no independent stream to \
             digest — the two things an image node IS. Counted rather than skipped, because \
             without this an artifact showing no image nodes could not be told apart from a \
             document that draws no images. The pixels were never read and no filter was run."
        ),
    )
}

/// A `Do` named an XObject this profile could not resolve (v1-S6).
pub fn xobject_name_unresolved(count: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::XOBJECT_NAME_UNRESOLVED,
        format!(
            "{count} `Do` operator(s) named an XObject this profile could not resolve — a name \
             absent from the page's `/Resources /XObject`, a resource embedded directly rather \
             than by reference, or an operand that is not a name. The page drew something; this \
             reader cannot say what, so it emits no node for it and counts it here instead. \
             **Not a refusal**: the operator is known, the document is malformed only in this \
             bounded way, and rejecting the whole file over it would turn documents that read \
             today into failures. Not a silent skip either — that is what this count is for."
        ),
    )
}

/// A composite font's code width came from its `/ToUnicode` codespace (v1-S6.1).
///
/// **The declared interim v1-S6.1 chose over silence.** PDF 32000-1 §9.7.5 gives a Type0 font's
/// code width to the CMap named by its `/Encoding`, and nothing in this profile parses those. The
/// width is taken from the font's `/ToUnicode` codespace instead — which agrees with `Identity-H`,
/// what real documents overwhelmingly use, and is unverified for anything else.
///
/// Declared rather than left to be found, because being found is exactly what went wrong the first
/// time: the simple-font half of this same decision was wrong for six slices and no artifact said
/// anything about how a code width was arrived at.
pub fn composite_font_codes_from_tounicode(fonts: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::COMPOSITE_FONT_CODES_FROM_TOUNICODE,
        format!(
            "{fonts} composite (`/Type0`) font(s) on this document had their character-code WIDTH \
             taken from the `/ToUnicode` CMap's codespace, because this profile does not parse the \
             `/Encoding` CMap that PDF 32000-1 §9.7.5 makes authoritative for it. For `Identity-H` \
             — which is what real documents overwhelmingly use — the two agree, and the codes are \
             right. For a predefined CJK CMap or a mixed-width embedded one they may not, and this \
             profile cannot tell which case it is in. Simple fonts are unaffected: their codes are \
             one byte by the specification and are split that way regardless of what any CMap \
             says. Nothing here is guessed at silently — that is what this declaration is for."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::LimitationScope;

    /// Every code this module emits, in one place, so a rename is a visible event.
    const PDF_CODES: [&str; 9] = [
        CLASSIFY_SAMPLE_BOUND,
        BACKEND_XREF_STRICT_20_BYTE,
        PREDEFINED_CMAPS_NOT_VENDORED,
        FORM_XOBJECT_TEXT_NOT_DESCENDED,
        FONT_WIDTHS_ABSENT,
        // Two this module declares that the list did not carry. `every_code_is_stable_kebab_case`
        // said *every code* over five of the seven `pub const` spellings in this file, and the
        // two it omitted are wire spellings a caller matches on exactly like the other five.
        XREF_ENTRY_PADDED,
        BROKEN_FONT_ENCODING,
        // Derived rather than declared: `undetected_reason_code` builds these from a reason name,
        // and `the_undetected_reason_codes_are_pinned` below pins both spellings.
        "garbled-reason-not-detected",
        "multi-column-reason-not-detected",
    ];

    #[test]
    fn every_code_is_stable_kebab_case() {
        // **Derived first, so the name can say *every*.** `PDF_CODES` is a hand-list, and a
        // hand-list is exactly as complete as whoever last edited it remembered to be: it
        // carried five of this module's seven `pub const` spellings, and `xref-entry-padded`
        // and `broken-font-encoding` were checked by nothing. Reading them back out of the
        // source is what makes the eighth const someone adds a decision rather than an omission.
        let this_file = include_str!("limitations.rs");
        let declared: Vec<&str> = this_file
            .lines()
            .filter_map(|l| l.strip_prefix("pub const "))
            .filter_map(|l| l.split_once(": &str = \""))
            .filter_map(|(_, v)| v.split_once('"'))
            .map(|(v, _)| v)
            .collect();
        assert_eq!(
            declared.len(),
            7,
            "this module declares {} `pub const` code(s): {declared:?}. Seven is the number at \
             v2-S13.1; a new one belongs in `PDF_CODES` too.",
            declared.len()
        );
        for code in &declared {
            assert!(
                PDF_CODES.contains(code),
                "`{code}` is declared here and absent from `PDF_CODES`, so nothing checks its \
                 spelling — which is the whole job of this test"
            );
        }

        for code in PDF_CODES {
            assert!(
                !code.is_empty()
                    && code
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "`{code}` is not a stable kebab-case code callers can match on"
            );
        }
    }

    /// The derived undetected-reason codes are pinned, because they are wire spellings.
    #[test]
    fn the_undetected_reason_codes_are_pinned() {
        assert_eq!(
            undetected_reason_code("garbled"),
            "garbled-reason-not-detected"
        );
        assert_eq!(
            undetected_reason_code("multi-column"),
            "multi-column-reason-not-detected"
        );
    }

    /// `not_detected` absorbed, entry for entry — nothing M2 declared was dropped in the move.
    #[test]
    fn every_not_detected_entry_became_a_limitation() {
        let declared = classify_limitations(8);
        for (reason, why) in NOT_DETECTED {
            let code = undetected_reason_code(reason);
            let l = declared
                .iter()
                .find(|l| l.code == code)
                .unwrap_or_else(|| panic!("`{reason}` lost its declaration in the M4 migration"));
            assert!(
                l.detail.contains(why),
                "the reason M2 gave for `{reason}` must survive verbatim, not be paraphrased"
            );
            assert_eq!(l.scope, LimitationScope::Profile);
        }
    }

    #[test]
    fn every_limitation_carries_a_real_reason() {
        let mut all = classify_limitations(8);
        all.extend(extract_limitations());
        all.push(font_widths_absent("no /Widths array"));
        all.push(resource_limit_pages(1, 2));
        for l in &all {
            assert!(
                l.detail.len() > 60,
                "`{}` needs a real reason, not a placeholder: {}",
                l.code,
                l.detail
            );
        }
    }

    /// The xref limitation is declared even on a document that opened cleanly.
    #[test]
    fn the_backend_refusal_is_declared_on_every_artifact() {
        for set in [classify_limitations(8), extract_limitations()] {
            let l = set
                .iter()
                .find(|l| l.code == BACKEND_XREF_STRICT_20_BYTE)
                .expect("the backend's refusal rate is a property of every run under it");
            assert_eq!(l.scope, LimitationScope::Profile);
            assert!(
                l.detail.contains("exits 2"),
                "a caller must be told the failure mode, not just that one exists"
            );
        }
    }

    #[test]
    fn the_sample_bound_limitation_names_the_configured_count() {
        let l = classify_limitations(3)
            .into_iter()
            .find(|l| l.code == CLASSIFY_SAMPLE_BOUND)
            .expect("declared");
        assert!(l.detail.contains("at most 3 page(s)"), "{}", l.detail);
    }
}
