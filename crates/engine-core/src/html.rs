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

//! Safe HTML: the second projection, under the same four laws (v1.1-S4).
//!
//! # Why this is a slice and not a stylesheet
//!
//! `ethos.html.v1` would not be worth a second artifact if it were `ethos.markdown.v1` with angle
//! brackets. It is worth one for exactly one reason: **GFM cannot say `rowspan`, and HTML can.**
//!
//! v1.1-S2 had to expand every merged cell into the slots it covered and count what that cost —
//! [`crate::markdown::GFM_SPAN_SLOTS_UNREPRESENTABLE`], because a table that reads as
//! `| North | merged span |  |` has silently become a different table. Here the origin cell comes
//! out as one `<td colspan="2">` and the slot it covers produces **nothing**, which is what the
//! document actually drew.
//!
//! It also never invents a header. GFM's delimiter row makes row 0 a header on every renderer
//! there is, whatever the document said; HTML has no such requirement, so **every cell is a
//! `<td>`** and there is no `<th>` in this file at all. No detector reads `/TH`, so a `<th>`
//! here would be this exporter deciding what the document meant.
//!
//! # Which erasures this artifact declares, and which two it does not
//!
//! The test is not "is the code named `gfm-*`" — it is **does this projection commit that
//! erasure**. Only two of the six are GFM's alone:
//!
//! | code | here | why |
//! | --- | --- | --- |
//! | `gfm-row-zero-separator-v1` | **dropped** | GFM's delimiter row asserts a header on every renderer; HTML emits only `<td>` and asserts none |
//! | `gfm-span-slots-unrepresentable-v1` | **recomputed** | GFM pays a slot for every merge; `<td rowspan>` pays nothing — unless the grid could not hold the span, and then it is counted |
//! | `gfm-cell-not-placed-v1` | **kept** | a cell outside the declared grid has nowhere to print in either projection |
//! | `gfm-cell-run-claimed-twice-v1` | **kept** | a run two cells claim is kept by the first in both, or one node's characters would be counted twice |
//! | `gfm-table-not-projected-v1` | **kept** | a table with zero rows or columns has no grid to draw in either |
//! | `gfm-list-item-run-joins-v1` | **kept** | two sibling `/LI`s have identical role paths, so nothing in the representation separates "the rest of this item" from "the next item", and both projections must guess |
//!
//! The three kept table codes describe faults in the **record**, not limits of GFM, so dropping
//! them here would be an artifact quietly committing an erasure its sibling discloses. The list
//! join is the same: HTML guesses the way Markdown does, because two artifacts of one document
//! that disagreed about how many list items it has would both be wrong to cite.
//!
//! The `gfm-` prefix on all six is therefore **historical rather than descriptive** — the codes
//! carry the name of the slice that first met them. Renaming them would change what
//! `ethos.markdown.v1` says under a `markdown_rule` this slice deliberately does not move, and a
//! rule id that stayed put while its output changed is the one dishonesty a version id exists to
//! prevent.
//!
//! # The four laws are the same four laws
//!
//! 1. **Never one without the other.** [`HtmlArtifact`] holds `html` and `anchor_map` together,
//!    and there is no `--html-only` on the CLI.
//! 2. **The map total-tiles the bytes.** The same [`crate::AnchorMap::new`] runs, over the HTML.
//! 3. **Two segment kinds.** Every tag, attribute, and newline this file writes is `syntax`. Only
//!    node text is `source`.
//! 4. **Coverage is a census.** The same [`crate::markdown::census`] closes it, so the two
//!    artifacts cannot disagree about how many CHARACTERS a document contains, nor about which
//!    are dropped and why. Their `structural_erasures` do differ, and that difference is the
//!    point — see the table below.
//!
//! # What is reused rather than rewritten
//!
//! Everything that decides *what* to emit: [`crate::markdown::normalize`], `heading_level`,
//! `list_role`, `dropped_code`, `hyphen_tail` and `plan_tables`. A second copy of the hyphen
//! predicate would be a second rule that could drift from the first, and only one of them would
//! have the corpus test that found `nonescr`.
//!
//! **Two things here are more than spelling, and both are consequences of using the span rather
//! than expanding it.** [`ListState`] carries nesting state a Markdown line never needs, and the
//! span-resolution pass in [`plan_tables`] has to clamp a merge that collides with another
//! origin — GFM expands every merge, so it never had to resolve the collision at all.
//!
//! # A fragment, deliberately
//!
//! There is no `<html>`, `<head>` or `<body>` wrapper, and no `<!doctype>`. Those are bytes no
//! document drew, they are the consumer's framing rather than this engine's, and a projection
//! whose whole point is that every byte is accounted for should not open with four of them it was
//! not asked for. Embedding the fragment is one concatenation; unwrapping a document is a parse.

use serde::{Deserialize, Serialize};

use crate::markdown::{
    census, dropped_code, heading_level, hyphen_tail, list_role, normalize, plan_tables, AnchorMap,
    Coverage, Emit, SlotRole, TablePlan, GFM_LIST_ITEM_RUN_JOINS, GFM_ROW_ZERO_SEPARATOR,
    GFM_SPAN_SLOTS_UNREPRESENTABLE, HYPHENATION_REJOIN_DROPPED,
};
use crate::{sha256_hex_bytes, ArtifactIdentity, DocumentRepresentation, EngineError, Sha256Hex};

/// The artifact type this module emits.
pub const HTML_ARTIFACT_TYPE: &str = "ethos.html.v1";

/// Semantic version of the artifact *shape*, independent of the parser build.
pub const HTML_SCHEMA_VERSION: &str = "1.0.0";

/// The projection rule v1.1-S4 ships.
///
/// A versioned id for the reason every rule here has one: it decides what comes out. It is
/// **separate from `markdown_rule`** and moves independently — a change to how a `<td>` is spelled
/// is not a change to how a GFM row is, and an artifact whose profile could not tell the two apart
/// would claim a comparability it lacks.
///
/// # Why this is `-v2` when `-v1` was never released
///
/// `html-blocks-v1` existed for one commit and got two tables wrong: a span colliding with another
/// cell's origin widened the row past the grid, and a list whose tree skipped a depth came out
/// unbalanced. The repair changed what a document projects to.
///
/// Nothing was ever published under `-v1` — it was never tagged or pushed — so no consumer holds
/// such an artifact. The id moves anyway, because the alternative is two builds in this
/// repository's own history producing different bytes under one id, and *that* is the state a rule
/// id exists to make impossible. A version that is cheap to move is exactly the one worth moving.
pub const HTML_RULE_BLOCKS_V2: &str = "html-blocks-v2";

// -------------------------------------------------------------------------------------------
// The artifact
// -------------------------------------------------------------------------------------------

/// HTML and the map that makes it citable — **one artifact, both halves**.
///
/// Deliberately parallel to [`crate::MarkdownArtifact`], field for field, with `markdown` renamed
/// to `html`. A consumer that learned one shape already knows this one, and the `artifact_type`
/// tells the two apart without inspecting the string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HtmlArtifact {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
    #[serde(flatten)]
    pub identity: ArtifactIdentity,
    /// Digest of the original source bytes, carried through from the representation.
    pub source_sha256: Sha256Hex,
    /// Digest of the representation this is a projection of.
    pub representation_sha256: Sha256Hex,
    /// Which projection rule produced this.
    pub html_rule: String,
    /// The HTML. Never emitted without [`Self::anchor_map`].
    pub html: String,
    /// The map from `html` bytes back to representation nodes.
    pub anchor_map: AnchorMap,
    /// The census of what reached the HTML and what did not.
    pub coverage: Coverage,
}

impl HtmlArtifact {
    /// The canonical bytes this artifact serializes to.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact will not canonicalize.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        crate::c14n::canonical_bytes_of(self).map_err(|e| EngineError::Malformed {
            what: "html artifact".into(),
            detail: e.to_string(),
        })
    }

    /// Re-check both laws on an artifact that came from bytes.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the map does not tile or the census does not balance.
    pub fn validate(&self) -> Result<(), EngineError> {
        AnchorMap::new(self.anchor_map.segments.clone(), &self.html)?;
        if !self.coverage.balances() {
            return Err(EngineError::Malformed {
                what: "html coverage".into(),
                detail: format!(
                    "{} emitted + {} dropped != {} in representation",
                    self.coverage.source_chars_emitted,
                    self.coverage.source_chars_dropped,
                    self.coverage.source_chars_in_representation
                ),
            });
        }
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------
// Escaping
// -------------------------------------------------------------------------------------------

/// The three characters HTML reads as structure, replaced by their entities.
///
/// `&` first, or the ampersands this function itself introduces would be escaped again.
///
/// # The whole entity is `source`, and that is not the markdown answer
///
/// GFM needs `a|b` written `a\|b`, and v1.1-S2 splits that: the backslash is `syntax` and the pipe
/// stays `source`, because the document really did draw a `|` and the exporter really did add a
/// `\` beside it. Each byte gets the honest label.
///
/// An entity is not an added byte beside a real one — it **replaces** the character. `&lt;`
/// contains no `<` at all, so there is no byte to call `source` and no byte to call the character
/// the page drew. Splitting it would mean labelling `&` as source text the document never wrote.
///
/// So the entity is emitted whole, as `source`, naming the node it came from, and
/// [`Emit::source_encoded`] is told it stands for one character rather than four. Inverting a
/// `source` segment on this artifact means HTML-unescaping it — a total, lossless transform, said
/// out loud here and in the schema, for the same reason [`normalize`] says it: a map that claimed
/// exact bytes and delivered encoded ones would be the subtlest lie available to this module.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Emit one node's normalized text as escaped `source`.
fn escaped_source(e: &mut Emit, text: &str, node: &str) {
    e.source_encoded(&escape(text), text.chars().count(), node);
}

// -------------------------------------------------------------------------------------------
// The projection
// -------------------------------------------------------------------------------------------

/// Where the emitter is inside a tagged list.
///
/// A Markdown list item is one line and needs no state; an HTML one is a `<li>` inside a `<ul>`
/// that has to be opened before it and closed after it, and a nested list lives **inside** the
/// open `<li>` of its parent. That is the one structural difference between the two projections,
/// and it is why this file has a walk of its own rather than a vocabulary passed to the other one.
#[derive(Default)]
struct ListState {
    /// One entry per open `<ul>`, innermost last: whether **that level** has an unclosed `<li>`.
    ///
    /// # A vector rather than a count and a flag
    ///
    /// The first version of this tracked `open: usize` and one `item_open: bool`, and assumed that
    /// closing an inner `<ul>` always lands back inside a parent `<li>`. That is false whenever a
    /// level was opened without an item under it, which happens as soon as the tree skips a depth
    /// — a run at `Document/L/L/LI/LBody` is depth 1 with no depth-0 item before it, and the tagged
    /// tree is free to say that. The result was `</li>` emitted for a level that never opened one:
    /// unbalanced HTML, from a representation the engine accepts.
    ///
    /// Per level, the question has an answer that is always right.
    levels: Vec<bool>,
}

impl ListState {
    /// Close nested lists until `levels.len() == down_to`, emitting the tags that requires.
    fn close_to(&mut self, e: &mut Emit, down_to: usize) {
        while self.levels.len() > down_to {
            if self.levels.pop() == Some(true) {
                e.syntax("</li>\n");
            }
            e.syntax("</ul>\n");
        }
    }

    /// Open the `<ul>`s needed to reach `depth`, then start an `<li>` in the innermost one.
    ///
    /// **A skipped depth opens a bare `<ul>` inside a `<ul>`.** That is not valid HTML5 — a `<ul>`
    /// may only contain `<li>` — and it is still the right answer here, because the alternative is
    /// to invent an `<li>` the document never declared so the output validates. This projection
    /// does not synthesize structure to look tidy; it does not synthesize a header row either. The
    /// output is balanced, every byte is `syntax`, and the nesting is exactly what the tree said.
    fn open_item(&mut self, e: &mut Emit, depth: usize) {
        self.close_to(e, depth + 1);
        while self.levels.len() < depth + 1 {
            // Opened INSIDE the parent's still-open `<li>` when there is one, which is what
            // nesting means.
            e.syntax("<ul>\n");
            self.levels.push(false);
        }
        if self.levels.last() == Some(&true) {
            e.syntax("</li>\n");
        }
        e.syntax("<li>");
        if let Some(last) = self.levels.last_mut() {
            *last = true;
        }
    }
}

/// Project a representation into HTML plus its map.
///
/// # The rule, in full — `html-blocks-v2`
///
/// 1. **Text runs only**, with every other node kind dropped into the same named bucket the
///    Markdown projection uses. Page artifacts are **not** dropped (O21/O22).
/// 2. **`<h1>`–`<h6>` when the structure tree says so**, `<p>` otherwise. No font size is read.
/// 3. **`<table>`** at the position of the first run one of its cells claims, with the merge
///    carried as `rowspan`/`colspan` and covered slots emitting nothing. Every cell is a `<td>`.
/// 4. **`<ul>`/`<li>`** for runs the tree places in an `/L`, nested from nested `/L`. The document
///    drew no bullet, so the marker here is a tag rather than a `- `; where it drew its own
///    `/Lbl`, that is source text inside the `<li>`.
/// 5. **Every tag, attribute and newline is `syntax`.** There is no indentation — an indent is
///    bytes no document drew, and while the map would tile them honestly, they would be `syntax`
///    inside a quote a consumer is likely to lift.
/// 6. **Node text is normalized** by [`normalize`] and then entity-escaped.
/// 7. **A word broken across a line is closed up**, by the same [`hyphen_tail`] the Markdown
///    projection calls. The joined word is readable here too, and citable on neither artifact.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the map this builds does not tile its own output.
pub fn to_html(
    repr: &DocumentRepresentation,
    parser_version: &str,
    profile_sha256: &Sha256Hex,
    html_rule: &str,
) -> Result<HtmlArtifact, EngineError> {
    let payload = repr.payload();

    let mut e = Emit::new();
    let mut buckets: std::collections::BTreeMap<&'static str, (usize, usize)> =
        std::collections::BTreeMap::new();
    // **Only the two erasures GFM alone commits are dropped.** See this module's header: the other
    // table codes describe faults in the RECORD, which this projection meets identically.
    let mut erasures: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    let mut in_representation = 0usize;

    let plans = plan_tables(payload, &mut erasures);
    // Every merge costs GFM a slot; here a merge costs nothing unless the grid could not hold it,
    // so the blanket count is replaced by what was actually clamped away.
    erasures.remove(GFM_SPAN_SLOTS_UNREPRESENTABLE);
    // HTML has no delimiter row, so it asserts no header and owes no count for one.
    erasures.remove(GFM_ROW_ZERO_SEPARATOR);
    let clamped: usize = plans.iter().map(|p| p.spans_clamped).sum();
    if clamped > 0 {
        *erasures.entry(GFM_SPAN_SLOTS_UNREPRESENTABLE).or_insert(0) += clamped;
    }
    let mut owner: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (t, plan) in plans.iter().enumerate() {
        for slot in &plan.slots {
            for node in slot {
                owner.insert(node.id.as_str(), t);
            }
        }
    }

    let mut emitted_tables = vec![false; plans.len()];
    let mut list = ListState::default();
    // The list item the previous run belonged to, and whether that run was the item's own `/Lbl`.
    // Tracked exactly as the Markdown projection tracks it, because the question it answers —
    // "is this run the rest of an item or the start of the next one?" — is about the TREE, and
    // both projections have to give it the same answer or they describe different documents.
    let mut open_item: Option<(usize, bool)> = None;
    let mut joined_tail = false;

    for (i, node) in payload.nodes.iter().enumerate() {
        in_representation += node.text.chars().count();

        if joined_tail {
            joined_tail = false;
            continue;
        }

        if let Some(code) = dropped_code(node.kind) {
            let b = buckets.entry(code).or_insert((0, 0));
            b.0 += node.text.chars().count();
            b.1 += 1;
            continue;
        }

        if let Some(&t) = owner.get(node.id.as_str()) {
            if !emitted_tables[t] {
                emitted_tables[t] = true;
                list.close_to(&mut e, 0);
                emit_table(&mut e, &plans[t]);
                open_item = None;
            }
            continue;
        }

        let text = normalize(&node.text);
        if text.is_empty() {
            continue;
        }

        if let Some(role) = list_role(node) {
            let continues = !role.label && open_item.map(|(d, _)| d) == Some(role.depth);
            if continues {
                // A second body run inside an item the tree did not itself close. **This
                // projection commits the same erasure the Markdown one does**, so it declares it
                // under the same code — see `GFM_LIST_ITEM_RUN_JOINS`, whose `gfm-` prefix is
                // historical rather than descriptive: two sibling `/LI`s have identical role
                // paths, so nothing in the representation distinguishes them, and every
                // projection has to guess. HTML guessing differently would make the two artifacts
                // disagree about how many items the document has.
                if !matches!(open_item, Some((_, true))) {
                    *erasures.entry(GFM_LIST_ITEM_RUN_JOINS).or_insert(0) += 1;
                }
                e.syntax(" ");
            } else {
                list.open_item(&mut e, role.depth);
            }
            escaped_source(&mut e, &text, node.id.as_str());
            open_item = Some((role.depth, role.label));
            continue;
        }

        open_item = None;
        list.close_to(&mut e, 0);

        let level = heading_level(node);
        match level {
            Some(l) => e.syntax(&format!("<h{l}>")),
            None => e.syntax("<p>"),
        }

        // The same join, from the same predicate. See `crate::markdown::hyphen_tail`.
        if let Some((tail, joined)) = hyphen_tail(node, &text, payload.nodes.get(i + 1), &owner) {
            e.joined_source_encoded(
                &escape(&joined),
                joined.chars().count(),
                node.id.as_str(),
                tail.id.as_str(),
            );
            let b = buckets.entry(HYPHENATION_REJOIN_DROPPED).or_insert((0, 0));
            b.0 += 1;
            b.1 += 1;
            joined_tail = true;
        } else {
            escaped_source(&mut e, &text, node.id.as_str());
        }

        match level {
            Some(l) => e.syntax(&format!("</h{l}>\n")),
            None => e.syntax("</p>\n"),
        }
    }

    list.close_to(&mut e, 0);

    // A table whose cells enclose no run has no first run to sit behind, so it has no position in
    // reading order at all — emitted here, in document order, for the reason the Markdown
    // projection gives: the alternative is dropping a grid the document drew because nobody
    // wrote in it.
    for (t, plan) in plans.iter().enumerate() {
        if plan.projected && !emitted_tables[t] {
            emit_table(&mut e, plan);
        }
    }

    let emitted_chars = e.emitted_chars;
    let anchor_map = AnchorMap::new(e.segments, &e.markdown)?;
    let html = e.markdown;

    // See the module header: this artifact declares the erasures it actually commits, and no
    // others.
    let coverage = census(payload, in_representation, emitted_chars, buckets, erasures);

    let artifact = HtmlArtifact {
        identity: ArtifactIdentity {
            artifact_type: HTML_ARTIFACT_TYPE.to_string(),
            schema_version: HTML_SCHEMA_VERSION.to_string(),
            parser_version: parser_version.to_string(),
            profile_sha256: profile_sha256.clone(),
        },
        source_sha256: payload.source.sha256.clone(),
        representation_sha256: repr.fingerprint().clone(),
        html_rule: html_rule.to_string(),
        html,
        anchor_map,
        coverage,
    };
    artifact.validate()?;
    Ok(artifact)
}

/// Write one table: `<tr>` per row, one `<td>` per slot that is not covered by a merge.
///
/// **The merge is carried rather than counted.** An origin cell spanning two columns comes out as
/// one `<td colspan="2">`, and the slot it covers emits nothing at all — emitting an empty `<td>`
/// beside it would put the flattened width back into the row and make the table a different table,
/// which is exactly the erasure v1.1-S2 had to disclose because GFM left it no choice.
///
/// **A hole is still a cell.** A slot no cell originates in and no merge reaches comes out as
/// `<td></td>`: the document drew that position and wrote nothing in it, and truncating it to
/// tidy the row is the competitor erasure A14 names.
fn emit_table(e: &mut Emit, plan: &TablePlan) {
    e.syntax("<table>\n");
    for row in 0..plan.rows {
        e.syntax("<tr>\n");
        for column in 0..plan.columns {
            let index = row * plan.columns + column;
            let (rowspan, colspan) = match plan.roles[index] {
                // The merge already said this slot is taken. Saying it twice makes it two cells.
                SlotRole::Covered => continue,
                SlotRole::Origin { rowspan, colspan } => (rowspan, colspan),
                SlotRole::Empty => (1, 1),
            };

            // **Always `<td>`, never `<th>`.** No detector reads `/TH` and the representation
            // carries no header declaration, so a `<th>` would be this exporter deciding what the
            // document meant. GFM had no such choice — its delimiter row makes row 0 a header on
            // every renderer — which is why `gfm-row-zero-separator-v1` exists there and no
            // equivalent is needed here.
            e.syntax("<td");
            if rowspan != 1 {
                e.syntax(&format!(" rowspan=\"{rowspan}\""));
            }
            if colspan != 1 {
                e.syntax(&format!(" colspan=\"{colspan}\""));
            }
            e.syntax(">");

            let mut first = true;
            for node in &plan.slots[index] {
                let text = normalize(&node.text);
                if text.is_empty() {
                    continue;
                }
                if !first {
                    // Two runs in one cell. The detector concatenates their raw text with nothing
                    // between; the space is this exporter's, and saying so is why it is syntax.
                    e.syntax(" ");
                }
                first = false;
                escaped_source(e, &text, node.id.as_str());
            }
            e.syntax("</td>\n");
        }
        e.syntax("</tr>\n");
    }
    e.syntax("</table>\n");
}

/// The digest of an artifact's canonical bytes, for callers that want to pin one.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the artifact will not canonicalize.
pub fn fingerprint(artifact: &HtmlArtifact) -> Result<Sha256Hex, EngineError> {
    let bytes = artifact.to_canonical_bytes()?;
    Sha256Hex::parse(format!("sha256:{}", sha256_hex_bytes(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::tests::{
        cell, repr_of, repr_of_lines, repr_of_paths, repr_with_table, simple_repr, spanning,
    };
    use crate::{DocumentRepresentation, Profile, SegmentKind};

    fn artifact_of(repr: DocumentRepresentation) -> HtmlArtifact {
        let profile = Profile::default();
        to_html(
            &repr,
            &profile.parser_version,
            &profile.profile_sha256().unwrap(),
            &profile.html_rule,
        )
        .expect("projects")
    }

    /// Every byte of the string belongs to exactly one segment, and reconstructing them gives it
    /// back. **Law 2**, on whatever the caller built.
    fn assert_tiles(a: &HtmlArtifact) {
        let mut cursor = 0usize;
        let mut rebuilt = String::new();
        for s in &a.anchor_map.segments {
            assert_eq!(s.start, cursor, "gap or overlap at {}", s.start);
            assert!(s.end > s.start, "empty segment");
            rebuilt.push_str(&a.html[s.start..s.end]);
            cursor = s.end;
        }
        assert_eq!(cursor, a.html.len(), "the map must cover the whole string");
        assert_eq!(rebuilt, a.html);
        assert!(a.coverage.balances(), "law 4: {:?}", a.coverage);
    }

    // ---------------------------------------------------------------------------------------
    // The shape
    // ---------------------------------------------------------------------------------------

    #[test]
    fn one_run_projects_to_one_paragraph_with_the_tags_as_syntax() {
        let a = artifact_of(simple_repr());
        assert_eq!(a.html, "<p>Hello Ethos</p>\n");
        assert_tiles(&a);

        // `<p>` · `Hello Ethos` · `</p>\n` — the text is the only source in it.
        let kinds: Vec<SegmentKind> = a.anchor_map.segments.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SegmentKind::Syntax,
                SegmentKind::Source,
                SegmentKind::Syntax
            ]
        );
        assert_eq!(a.coverage.source_chars_emitted, 11);
        assert!(a.coverage.dropped.is_empty());
        // No `gfm-*` code reaches this artifact — see the module header.
        assert!(a.coverage.structural_erasures.is_empty());
    }

    #[test]
    fn a_heading_role_from_the_tree_projects_as_a_heading_element() {
        let a = artifact_of(repr_of(&[
            ("Chapter One", Some("H1")),
            ("Body text", None),
            ("Sub", Some("H3")),
        ]));
        assert_eq!(
            a.html,
            "<h1>Chapter One</h1>\n<p>Body text</p>\n<h3>Sub</h3>\n"
        );
        assert_tiles(&a);
    }

    /// **No font size is read here either.** L29 is REFUSE on both projections.
    #[test]
    fn an_untagged_run_is_a_paragraph_however_it_was_drawn() {
        let a = artifact_of(repr_of(&[("Looks like a title", None)]));
        assert_eq!(a.html, "<p>Looks like a title</p>\n");
    }

    // ---------------------------------------------------------------------------------------
    // Escaping — the three characters HTML reads as structure
    // ---------------------------------------------------------------------------------------

    /// `&`, `<` and `>` become entities, and the entity is **source** naming the run that drew it.
    ///
    /// No fixture in either corpus contains one, which is why this is a hand-built representation:
    /// a projection that left a raw `<` in text would produce a document whose structure depends
    /// on what the page happened to say.
    #[test]
    fn the_three_structural_characters_are_escaped_and_still_invert() {
        let a = artifact_of(repr_of(&[("Tom & Jerry <b> 3 > 2", None)]));
        assert_eq!(
            a.html, "<p>Tom &amp; Jerry &lt;b&gt; 3 &gt; 2</p>\n",
            "the document's own characters, encoded rather than dropped and rather than left live"
        );
        assert_tiles(&a);

        // **The census counts CHARACTERS the document drew, not the bytes the entity took.**
        // `Tom & Jerry <b> 3 > 2` is 21 characters; the escaped form is 33 bytes. Counting bytes
        // would make `emitted` exceed what the representation holds and the residue underflow.
        assert_eq!(a.coverage.source_chars_emitted, 21);
        assert_eq!(a.coverage.source_chars_in_representation, 21);
        assert_eq!(a.coverage.source_chars_dropped, 0);

        // The whole entity is inside the source segment: there is no byte of `&lt;` that is the
        // `<` the page drew, so labelling part of it syntax would name bytes no document wrote.
        let source: Vec<&str> = a
            .anchor_map
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Source)
            .map(|s| &a.html[s.start..s.end])
            .collect();
        assert_eq!(source, vec!["Tom &amp; Jerry &lt;b&gt; 3 &gt; 2"]);
    }

    #[test]
    fn an_ampersand_is_escaped_once_and_not_twice() {
        let a = artifact_of(repr_of(&[("a &amp; b", None)]));
        assert_eq!(
            a.html, "<p>a &amp;amp; b</p>\n",
            "the page drew the five characters `&amp;`, so the artifact must say so rather than \
             silently agree with a reader who thinks they were an entity"
        );
        assert_tiles(&a);
    }

    // ---------------------------------------------------------------------------------------
    // Tables — the reason this artifact exists
    // ---------------------------------------------------------------------------------------

    /// **The merge is carried, not flattened.** One `<td colspan="2">`, and the covered slot
    /// emits nothing at all.
    ///
    /// This is the whole case for a second projection. The Markdown artifact has to expand the
    /// same cell and count the slot in `gfm-span-slots-unrepresentable-v1`; here the erasure is
    /// not committed, so no such code appears on the coverage.
    #[test]
    fn a_merged_cell_becomes_one_td_with_a_colspan() {
        let a = artifact_of(repr_with_table(
            &["North", "merged span", "Total"],
            2,
            3,
            &[
                cell(0, 0, &[0]),
                spanning(0, 1, 1, 2, &[1]),
                cell(1, 0, &[2]),
            ],
        ));
        assert_eq!(
            a.html,
            "<table>\n<tr>\n<td>North</td>\n<td colspan=\"2\">merged span</td>\n</tr>\n\
             <tr>\n<td>Total</td>\n<td></td>\n<td></td>\n</tr>\n</table>\n"
        );
        assert_tiles(&a);
        assert!(
            a.coverage.structural_erasures.is_empty(),
            "HTML says what GFM could not, so it carries none of GFM's erasure codes: {:?}",
            a.coverage.structural_erasures
        );
        assert_eq!(
            a.html.matches("merged span").count(),
            1,
            "the cell's text appears once — in the origin, not again in the slot it covers"
        );
    }

    #[test]
    fn a_rowspan_is_carried_too_and_the_covered_slot_below_emits_nothing() {
        let a = artifact_of(repr_with_table(
            &["tall", "b", "c"],
            2,
            2,
            &[
                spanning(0, 0, 2, 1, &[0]),
                cell(0, 1, &[1]),
                cell(1, 1, &[2]),
            ],
        ));
        assert_eq!(
            a.html,
            "<table>\n<tr>\n<td rowspan=\"2\">tall</td>\n<td>b</td>\n</tr>\n\
             <tr>\n<td>c</td>\n</tr>\n</table>\n"
        );
        assert_tiles(&a);
    }

    /// **Every row is exactly as wide as the table declares**, whatever the record asked for.
    ///
    /// Found by review, on a shipped fixture. `fixtures/engine/ruled-table-overlap` declares a 2x2
    /// whose row 1 holds BOTH a `colspan: 2` cell at (1,0) and an ordinary cell at (1,1). Before
    /// the span-resolution pass, the later cell promoted the covered slot back to an origin and
    /// the row emitted three cells wide in a two-column table.
    ///
    /// GFM never had to care — it expands every merge, so the arithmetic is the same either way.
    /// A projection that USES the span has to resolve the collision, and clamping is the only
    /// answer that neither drops a cell nor widens the row.
    #[test]
    fn a_span_colliding_with_another_origin_is_clamped_rather_than_widening_the_row() {
        let a = artifact_of(repr_with_table(
            &["A", "B", "C", ""],
            2,
            2,
            &[
                cell(0, 0, &[0]),
                cell(0, 1, &[1]),
                spanning(1, 0, 1, 2, &[2]),
                cell(1, 1, &[3]),
            ],
        ));
        assert_eq!(
            a.html,
            "<table>\n<tr>\n<td>A</td>\n<td>B</td>\n</tr>\n\
             <tr>\n<td>C</td>\n<td></td>\n</tr>\n</table>\n",
            "row 1 is two cells wide, and the colspan the record asked for is gone"
        );
        assert_tiles(&a);
        assert_eq!(
            a.coverage
                .structural_erasures
                .iter()
                .find(|e| e.code == "gfm-span-slots-unrepresentable-v1")
                .map(|e| e.count),
            Some(1),
            "and the merge that could not be held is DECLARED, not silently dropped: {:?}",
            a.coverage.structural_erasures
        );
    }

    /// A span running off the grid edge is clamped too, and costs a bounded walk.
    ///
    /// `rowspan` is a `u32` on the wire. Filling the covered slots by iterating the DECLARED span
    /// rather than the part that intersects the grid made a four-byte number into minutes of CPU,
    /// on the shared plan both projections use.
    #[test]
    fn a_span_running_off_the_grid_is_clamped_and_costs_a_bounded_walk() {
        let a = artifact_of(repr_with_table(
            &["huge"],
            1,
            1,
            &[spanning(0, 0, 4_000_000_000, 4_000_000_000, &[0])],
        ));
        assert_eq!(
            a.html, "<table>\n<tr>\n<td>huge</td>\n</tr>\n</table>\n",
            "one slot exists, so the span is one slot"
        );
        assert_tiles(&a);
    }

    /// A hole the document drew is still a cell. Truncating it is the erasure A14 names.
    #[test]
    fn an_empty_trailing_row_is_not_truncated() {
        let a = artifact_of(repr_with_table(&["only"], 2, 2, &[cell(0, 0, &[0])]));
        assert_eq!(
            a.html,
            "<table>\n<tr>\n<td>only</td>\n<td></td>\n</tr>\n\
             <tr>\n<td></td>\n<td></td>\n</tr>\n</table>\n"
        );
        assert_tiles(&a);
    }

    /// **Never `<th>`.** No detector reads `/TH`, so a header row would be invented — which
    /// is exactly what GFM forces and `gfm-row-zero-separator-v1` had to disclose.
    #[test]
    fn no_header_row_is_synthesized() {
        let a = artifact_of(repr_with_table(
            &["Region", "Total"],
            1,
            2,
            &[cell(0, 0, &[0]), cell(0, 1, &[1])],
        ));
        assert!(!a.html.contains("<th"), "no header claim: {}", a.html);
        assert!(!a.html.contains("<thead"), "and no header section either");
    }

    // ---------------------------------------------------------------------------------------
    // Lists
    // ---------------------------------------------------------------------------------------

    /// A nested `/L` becomes a `<ul>` **inside the open `<li>`**, and everything closes.
    #[test]
    fn a_nested_tagged_list_nests_and_closes() {
        let a = artifact_of(repr_of_paths(&[
            ("First", Some(&["Document", "L", "LI", "LBody"])),
            ("Second", Some(&["Document", "L", "LI", "LBody"])),
            ("Deep", Some(&["Document", "L", "LI", "L", "LI", "LBody"])),
            ("Third", Some(&["Document", "L", "LI", "LBody"])),
            ("After", None),
        ]));
        // **`First` and `Second` land in ONE item, and that is the shared guess.** Two sibling
        // `/LI`s have identical role paths, so the tree does not distinguish "the rest of this
        // item" from "the next one"; the Markdown projection joins them too, and both count it.
        assert_eq!(
            a.html,
            "<ul>\n<li>First Second<ul>\n<li>Deep</li>\n</ul>\n</li>\n\
             <li>Third</li>\n</ul>\n<p>After</p>\n"
        );
        assert_tiles(&a);
        assert_eq!(
            a.coverage
                .structural_erasures
                .iter()
                .find(|e| e.code == "gfm-list-item-run-joins-v1")
                .map(|e| e.count),
            Some(1),
            "the join is DECLARED here, not only in the Markdown artifact — this projection \
             commits the same erasure, so it owes the same count: {:?}",
            a.coverage.structural_erasures
        );
        assert!(
            !a.coverage
                .structural_erasures
                .iter()
                .any(|e| e.code.starts_with("gfm-") && e.code.contains("span")),
            "and no TABLE erasure, because `<td rowspan>` commits none"
        );
    }

    /// **The tags balance and nest, on every depth sequence the tree can produce.**
    ///
    /// Found by review. The first `ListState` tracked one `item_open` flag and assumed closing an
    /// inner `<ul>` always lands inside a parent `<li>`. A tree that SKIPS a depth breaks that —
    /// `Document/L/L/LI/LBody` is depth 1 with no depth-0 item before it — and the projection
    /// emitted `</li>` for a level that never opened one.
    #[test]
    fn the_list_tags_balance_on_every_depth_sequence() {
        /// Every tag this projection writes, matched against its closer in order.
        fn balanced(html: &str) -> bool {
            let mut stack: Vec<&str> = Vec::new();
            let mut rest = html;
            while let Some(at) = rest.find('<') {
                rest = &rest[at + 1..];
                let Some(end) = rest.find('>') else {
                    return false;
                };
                let (tag, after) = (&rest[..end], &rest[end + 1..]);
                rest = after;
                // `td colspan="2"` closes as `td`, so the name is the first word.
                let name = tag.split_whitespace().next().unwrap_or("");
                match name.strip_prefix('/') {
                    Some(close) if stack.pop() != Some(close) => return false,
                    Some(_) => {}
                    None => stack.push(name),
                }
            }
            stack.is_empty()
        }

        // **The checker checks.** A balance assertion that cannot fail would make every case below
        // pass for free, so it is pointed at the exact shape this test exists to catch: the stray
        // `</li>` the old single-flag `ListState` emitted for a level that never opened one.
        assert!(balanced("<ul>\n<li>x</li>\n</ul>\n"));
        assert!(
            !balanced("<ul>\n<ul>\n<li>x</li>\n</ul>\n</li>\n</ul>\n"),
            "the balance checker must reject a stray `</li>`"
        );

        let path_at = |depth: usize| -> Vec<&'static str> {
            let mut p = vec!["Document"];
            for _ in 0..=depth {
                p.push("L");
                p.push("LI");
            }
            p.push("LBody");
            p
        };

        // Includes the sequences that used to fail: a first item below depth 0, a jump of two, and
        // a descent from a skipped level.
        for depths in [
            vec![0usize],
            vec![0, 1, 0],
            vec![0, 0],
            vec![1],
            vec![0, 2],
            vec![2, 0],
            vec![0, 2, 0],
            vec![1, 0],
            vec![0, 1, 1, 0],
            vec![2],
            vec![3, 1],
        ] {
            let owned: Vec<Vec<&str>> = depths.iter().map(|d| path_at(*d)).collect();
            let specs: Vec<(&str, Option<&[&str]>)> =
                owned.iter().map(|p| ("item", Some(p.as_slice()))).collect();
            let a = artifact_of(repr_of_paths(&specs));
            assert!(
                balanced(&a.html),
                "depths {depths:?} produced unbalanced HTML: {}",
                a.html
            );
            assert_eq!(
                a.html.matches("<ul>").count(),
                a.html.matches("</ul>").count(),
                "depths {depths:?}: {}",
                a.html
            );
            assert_eq!(
                a.html.matches("<li>").count(),
                a.html.matches("</li>").count(),
                "depths {depths:?}: {}",
                a.html
            );
            assert_tiles(&a);
        }
    }

    /// An untagged document grows no list, exactly as in Markdown.
    #[test]
    fn an_untagged_bullet_glyph_is_not_a_list() {
        let a = artifact_of(repr_of(&[("- Not a list item", None)]));
        assert_eq!(a.html, "<p>- Not a list item</p>\n");
        assert!(!a.html.contains("<ul>"));
    }

    /// A list left open at the end of the document is closed by the projection, not by the reader.
    #[test]
    fn a_list_at_the_end_of_the_document_is_closed() {
        let a = artifact_of(repr_of_paths(&[(
            "Only",
            Some(&["Document", "L", "LI", "LBody"]),
        )]));
        assert_eq!(a.html, "<ul>\n<li>Only</li>\n</ul>\n");
        assert_tiles(&a);
    }

    // ---------------------------------------------------------------------------------------
    // The hyphen join — the same rule, called rather than copied
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_hyphen_at_a_line_end_is_joined_here_too_and_counted_once() {
        let a = artifact_of(repr_of_lines(&["hyphen-", "ated"]));
        assert_eq!(a.html, "<p>hyphenated</p>\n");
        assert_tiles(&a);

        let joined = a
            .anchor_map
            .segments
            .iter()
            .find(|s| s.kind == SegmentKind::Source)
            .expect("a source segment");
        assert_eq!(
            joined.node_ids.len(),
            2,
            "the joined bytes came from two runs, and the map says so on this artifact too"
        );
        let b = a
            .coverage
            .dropped
            .iter()
            .find(|b| b.code == "hyphenation-rejoin-dropped-v1")
            .expect("the removed hyphen is a named bucket here as well");
        assert_eq!((b.chars, b.nodes), (1, 1));
    }

    /// The same refusals, because it is the same predicate — not a second copy of it.
    #[test]
    fn a_hyphen_inside_a_line_is_not_joined_here_either() {
        let a = artifact_of(repr_of(&[("non-", None), ("escr", None)]));
        assert_eq!(a.html, "<p>non-</p>\n<p>escr</p>\n");
        assert!(!a.html.contains("nonescr"));
        assert_tiles(&a);
    }

    // ---------------------------------------------------------------------------------------
    // The artifact
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_hand_edited_artifact_is_refused_on_validate() {
        let mut a = artifact_of(simple_repr());
        a.coverage.source_chars_emitted += 1;
        assert!(
            a.validate().is_err(),
            "an unbalanced census is the failure this type exists to make visible"
        );
    }

    #[test]
    fn the_two_projections_account_for_the_same_characters() {
        let repr = repr_of_lines(&["hyphen-", "ated"]);
        let html = artifact_of(repr.clone());
        let profile = Profile::default();
        let md = crate::to_markdown(
            &repr,
            &profile.parser_version,
            &profile.profile_sha256().unwrap(),
            &profile.markdown_rule,
        )
        .expect("projects");

        // **The same census, from the same function.** Two artifacts of one document that
        // disagreed about how many characters it contains would make both unciteable.
        assert_eq!(
            html.coverage.source_chars_in_representation,
            md.coverage.source_chars_in_representation
        );
        assert_eq!(
            html.coverage.source_chars_emitted,
            md.coverage.source_chars_emitted
        );
        assert_eq!(html.coverage.dropped, md.coverage.dropped);
    }
}
