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

//! The extraction-assurance state — the L1 gate (`docs/01-CONTRACT.md` §7).
//!
//! L1's achievement condition names capability declarations explicitly: *"a versioned
//! processor/profile produced a representation with declared capabilities and an
//! extraction-assurance state."* **An artifact without them has not reached "extracted,"
//! regardless of how good its text is.**
//!
//! # The four things an artifact must say
//!
//! | Block | Answers |
//! | --- | --- |
//! | [`Capabilities`] | What this *profile* can do at all |
//! | [`Limitation`] | What it could not do — on the profile, on this document, or on one page |
//! | [`PageState`] | What happened to each authorized page |
//! | [`CoverageSummary`] | The reconciliation: authorized == the sum of the dispositions |
//!
//! Plus [`ProcessingTerminalState`], which makes partial processing a **first-class outcome**
//! rather than "success with some pages missing". A verification over a partially processed
//! document must never render as a clean verification of the whole document, and the only way to
//! guarantee that is to make "complete" impossible to claim when a gap exists — which is why
//! [`Assurance::new`] computes the terminal state from the page states instead of accepting one.
//!
//! # Capability-limited beats negative
//!
//! Workbench rule 4: **absence of extractable content is never evidence of absence in the
//! source.** [`page_binding_status`] is the function that enforces it. A query bound to a page
//! that failed, was quarantined, or was never attempted returns
//! [`PageBindingResult::CapabilityLimited`] carrying the limitation code — never a boolean
//! "not present", which is the shape that turns "we could not read page 7" into "page 7 does not
//! say that".
//!
//! # No confidence
//!
//! Nothing here summarises quality. There is no score, no grade, and no implied `1.0`: a
//! processor with no uncertainty to report emits an **absent field**, and v0 has no uncertainty
//! to report at all because every node is read by a deterministic reader
//! (`docs/01-CONTRACT.md` §9.1).

use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::profile::Capabilities;

/// Stable limitation codes owned by the contract layer.
///
/// **Wire spelling is kebab-case and stable**: callers match on these strings, so renaming one is
/// a schema change, not a refactor. Codes here are format-independent — every one of them is
/// derivable from [`Capabilities`] or from a run-level budget. Format-specific codes (a PDF
/// backend's xref strictness, an undescended form XObject) belong to the crate that owns the
/// format; `engine-core` never learns what a PDF is (`docs/04-ARCHITECTURE.md` §1).
pub mod codes {
    /// [`Capabilities::spans`] is false: no text spans are emitted.
    pub const SPANS_NOT_EMITTED: &str = "spans-not-emitted";
    /// [`Capabilities::char_offsets`] is false: spans carry no character offsets.
    pub const CHAR_OFFSETS_NOT_EMITTED: &str = "char-offsets-not-emitted";
    /// [`Capabilities::tables`] is false: no table is detected or emitted.
    pub const TABLES_NOT_EXTRACTED: &str = "tables-not-extracted";
    /// [`Capabilities::tables`] is **true**, and the detector finds ruled tables only.
    ///
    /// The one limitation here that partners a `true` capability rather than a `false` one, and
    /// deliberately so. `tables: true` claims *this profile looked* — it does not claim every
    /// table is found, and a table drawn without ruling lines is a real miss (v1-S2). Without
    /// this, an empty `tables` array would read as "there are no tables here", when what it
    /// means is "there are no **ruled** tables here".
    pub const UNRULED_TABLES_NOT_DETECTED: &str = "unruled-tables-not-detected";
    /// [`Capabilities::measured_ink_boxes`] is false: geometry is typed-absent throughout.
    pub const MEASURED_INK_BOXES_NOT_EMITTED: &str = "measured-ink-boxes-not-emitted";
    /// [`Capabilities::multi_column_reading_order`] is false: order is single-column.
    ///
    /// **The limitation v0 must declare explicitly** (`docs/03-V0-SCOPE.md` §3.2). A two-column
    /// document is read in the wrong order and the artifact says so, rather than silently
    /// producing interleaved text.
    pub const MULTI_COLUMN_READING_ORDER: &str = "multi-column-reading-order";
    /// [`Capabilities::structural_locators`] is false: no structural address is claimed.
    pub const STRUCTURAL_LOCATORS_NOT_CLAIMED: &str = "structural-locators-not-claimed";
    /// A configured page budget stopped processing before the document ended.
    ///
    /// Document-scoped, and only present when the budget actually bit.
    pub const RESOURCE_LIMIT_PAGES: &str = "resource-limit-pages";

    /// Some nodes carry no measurable ink box, so a grounding projection must omit them.
    ///
    /// Document-scoped, and only present when it applies. The count travels in the detail
    /// because `ethos.grounding.v1` is `additionalProperties: false` and cannot carry a
    /// limitation list of its own — so the record is the only place a consumer can come back to
    /// and find out what the projection dropped.
    pub const GEOMETRY_ABSENT_NOT_GROUNDABLE: &str = "geometry-absent-not-groundable";
}

/// How wide a limitation reaches.
///
/// Ordered `Profile < Document < Page` so a normalized limitation list sorts from "this engine
/// can never" down to "this one page could not", which is also the order a reader wants them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "kind",
    content = "value",
    deny_unknown_fields
)]
pub enum LimitationScope {
    /// True of every run under this profile, whatever the document. Always declared.
    Profile,
    /// True of this document under this profile. Declared only when it actually applies.
    Document,
    /// True of one **1-based** page.
    Page(u32),
}

/// A named gap: something this profile cannot do, or this document/run could not do.
///
/// **Limitations live in the artifact, not in a doc comment.** A limitation a reviewer can read
/// and a consumer cannot is not a declaration; it is an apology. `05-MILESTONES.md` M4 states
/// the rule as an "Out": *any limitation that exists only in a doc comment*.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limitation {
    /// Stable machine-matchable code, kebab-case. See [`codes`].
    pub code: String,
    /// Human-readable detail. Stable enough to assert on, because a limitation nobody can match
    /// against is only marginally better than an undeclared one.
    pub detail: String,
    /// How far the limitation reaches.
    pub scope: LimitationScope,
}

impl Limitation {
    /// A limitation of the profile itself.
    pub fn profile(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            detail: detail.into(),
            scope: LimitationScope::Profile,
        }
    }

    /// A limitation of this document under this profile.
    pub fn document(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            detail: detail.into(),
            scope: LimitationScope::Document,
        }
    }

    /// A limitation of one 1-based page.
    pub fn page(index: u32, code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            detail: detail.into(),
            scope: LimitationScope::Page(index),
        }
    }

    /// Sort and deduplicate a limitation list into its canonical order.
    ///
    /// Order on the wire must not depend on the order code happened to append in, or two runs
    /// that observed the same gaps would produce different bytes — and byte identity is a test
    /// here, not an aspiration (`docs/05-MILESTONES.md`, standing rule 6).
    pub fn normalize(list: &mut Vec<Self>) {
        list.sort();
        list.dedup();
    }
}

impl Capabilities {
    /// Every `false` capability, as a declared profile-scope [`Limitation`].
    ///
    /// This is the mirror of the rule that no capability may be `true` without a proof test: no
    /// capability may be `false` without a declared limitation. Both directions are gated
    /// exhaustively — the destructuring below fails to **compile** when a capability is added
    /// without a code, and `engine-pdf`'s capability table fails the build when a `true` one
    /// arrives without a named proof.
    ///
    /// Deriving these rather than hand-listing them is what stops the two from drifting: a
    /// capability flipped to `false` in a profile cannot leave a stale "we do this" behind.
    pub fn declared_limitations(&self) -> Vec<Limitation> {
        // Exhaustiveness gate. Adding a capability without a limitation code is a compile error.
        let Capabilities {
            spans,
            char_offsets,
            tables,
            measured_ink_boxes,
            multi_column_reading_order,
            structural_locators,
        } = *self;

        let mut out = Vec::new();

        if !spans {
            out.push(Limitation::profile(
                codes::SPANS_NOT_EMITTED,
                "This profile emits no text spans, so no run-level evidence exists to bind a \
                 claim to. An empty node list is a statement about this profile, not about the \
                 document.",
            ));
        }
        if !char_offsets {
            out.push(Limitation::profile(
                codes::CHAR_OFFSETS_NOT_EMITTED,
                "Spans carry no character offsets into a parent element's text. The hierarchy \
                 exists — a projection emits an element and a span per run, and the span names \
                 its element — but v0 performs no line or block grouping, so the two are the \
                 SAME object and an offset would always be 0..len. Emitting it would advertise \
                 sub-element addressing this profile cannot do. It becomes informative at v1, \
                 when grouping makes elements coarser than spans. A consumer needing \
                 `char_start`/`char_end` must not infer them from concatenation order.",
            ));
        }
        if tables {
            // **A limitation partnering a TRUE capability.** Every other arm here declares what a
            // `false` capability does not do; this one declares the scope of what a `true` one
            // does. `tables: true` says the detector looked, and a reader seeing an empty array
            // needs to know it looked for *ruled* tables — otherwise "no tables found" reads as
            // "this document has no tables", which is a much stronger claim than v1-S1 makes.
            out.push(Limitation::profile(
                codes::UNRULED_TABLES_NOT_DETECTED,
                "Tables are detected from RULING LINES only: the rectangles and axis-aligned \
                 edges a document actually painted. A table laid out by alignment alone — no \
                 rules drawn — is NOT found, and an empty `tables` array therefore means `no \
                 ruled table was found here`, never `this page has no table`. The unruled case \
                 is a separate detection rule under its own derivation and lands at v1-S2. \
                 Curves are never flattened into ruling lines, so a grid drawn with Béziers is \
                 also missed rather than approximated.",
            ));
        } else {
            out.push(Limitation::profile(
                codes::TABLES_NOT_EXTRACTED,
                "No table is detected, reconstructed, or emitted. Ruling lines may still be \
                 counted as a layout reason code, which is an observation about the page and not \
                 a table.",
            ));
        }
        if !measured_ink_boxes {
            out.push(Limitation::profile(
                codes::MEASURED_INK_BOXES_NOT_EMITTED,
                "No ink box is measured, so geometry is typed-absent throughout. A box is never \
                 derived from a font size to fill the gap.",
            ));
        }
        if !multi_column_reading_order {
            out.push(Limitation::profile(
                codes::MULTI_COLUMN_READING_ORDER,
                "Reading order is single-column: runs appear in content-stream order with no \
                 reordering. A multi-column document is therefore read in the WRONG ORDER, and \
                 this declaration is the engine saying so rather than silently interleaving \
                 text. No multi-column detector runs either — pdf-inspector's flips on a single \
                 line of text (min_lines < 15), so a one-line edit reorders a whole page, and a \
                 cliff-shaped heuristic cannot sit under a determinism contract. Ships at v1 \
                 with a stable rule and a fixture.",
            ));
        }
        if !structural_locators {
            out.push(Limitation::profile(
                codes::STRUCTURAL_LOCATORS_NOT_CLAIMED,
                "No structural address is claimed. A marked-content id is captured verbatim \
                 where a page's own content stream supplies one via BDC, and omitted where it \
                 does not — but the tagged-structure tree is not read, so an absent id is not \
                 evidence the document is untagged, and no role path is available. Full \
                 structural addressing is v1.",
            ));
        }

        out
    }
}

/// What happened to one authorized page.
///
/// **Naming is load-bearing here.** A caller must not be able to read "this page was never
/// looked at" as "this page has no text". [`Self::NotAttempted`] says which of the two it is, in
/// the variant name, before anyone reads a reason string — the same discipline that makes
/// [`crate::GeometryPresence`] a type rather than an `Option`.
///
/// Every non-[`Self::Processed`] variant carries a **limitation code**, not free prose. The prose
/// lives once, in the matching [`Limitation`]; the page state points at it. That keeps a
/// 500-page document's state list small and makes "every gap names a declared limitation" an
/// invariant a test can check rather than a convention.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "state",
    content = "reason",
    deny_unknown_fields
)]
pub enum PageState {
    /// Read to this profile's declared capabilities. The only state that supports a positive
    /// answer to a query.
    Processed,
    /// Processing was attempted and failed.
    Failed(String),
    /// The page holds something this profile does not claim to handle.
    ///
    /// Distinct from [`Self::Failed`]: the page may be perfectly valid and the gap is ours —
    /// the same distinction [`EngineError::Unsupported`] draws against
    /// [`EngineError::Malformed`].
    Unsupported(String),
    /// Held back deliberately — a resource budget, or a policy this profile declares.
    Quarantined(String),
    /// Never attempted. **Not** "empty", and not a failure.
    ///
    /// Bounded classification reaches this state by design: sampling `N` pages and stopping is
    /// the point, and the pages past `N` were not observed at all. Reporting them as processed
    /// with nothing on them would be the lie this variant exists to prevent.
    NotAttempted(String),
}

impl PageState {
    /// Whether the page was read to the profile's declared capabilities.
    pub fn is_processed(&self) -> bool {
        matches!(self, Self::Processed)
    }

    /// The limitation code this state points at, if it is a gap.
    pub fn limitation_code(&self) -> Option<&str> {
        match self {
            Self::Processed => None,
            Self::Failed(c)
            | Self::Unsupported(c)
            | Self::Quarantined(c)
            | Self::NotAttempted(c) => Some(c.as_str()),
        }
    }
}

/// One page's disposition, keyed by its **1-based** index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageStateEntry {
    /// 1-based page number, as the document numbers its own pages.
    pub index: u32,
    /// What happened to it.
    pub state: PageState,
}

/// The reconciliation between what a run was authorized to read and what it did read.
///
/// **Present on every artifact representing a read attempt**, including a fully successful one.
/// Zeros in the gap buckets are the correct output for a clean document; the *absence* of this
/// object is what is forbidden, because then "no gaps declared" and "gaps not tracked" look
/// identical.
///
/// The invariant that makes it worth having:
/// `pages_authorized == processed + failed + unsupported + quarantined + not_attempted`.
/// [`Self::reconciles`] checks it, [`Self::from_page_states`] cannot produce a summary that
/// violates it, and a test asserts it on every fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageSummary {
    /// Pages this run was authorized to read — the denominator, not a bucket.
    pub pages_authorized: u32,
    /// Pages read to the profile's declared capabilities.
    pub pages_processed: u32,
    /// Pages where processing was attempted and failed.
    pub pages_failed: u32,
    /// Pages holding something this profile does not claim to handle.
    pub pages_unsupported: u32,
    /// Pages held back by a budget or a declared policy.
    pub pages_quarantined: u32,
    /// Pages never attempted.
    ///
    /// **The bucket a four-bucket summary would have had to lie about.** Bounded classification
    /// samples `N` pages and stops; folding the rest into "processed" would claim observations
    /// nobody made, and folding them into "failed" would claim failures that never happened.
    pub pages_not_attempted: u32,
}

impl CoverageSummary {
    /// Tally a page-state list into a summary.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the states do not describe exactly the authorized pages:
    /// a wrong count, a duplicate index, a zero index (pages are 1-based), or an index past the
    /// authorized count. Each of those would produce a summary that reconciles arithmetically
    /// while describing a document that does not exist.
    pub fn from_page_states(
        pages_authorized: u32,
        states: &[PageStateEntry],
    ) -> Result<Self, EngineError> {
        let malformed = |detail: String| EngineError::Malformed {
            what: "coverage summary".into(),
            detail,
        };

        if states.len() as u64 != u64::from(pages_authorized) {
            return Err(malformed(format!(
                "{} page states for {pages_authorized} authorized pages; every authorized page \
                 needs exactly one disposition",
                states.len()
            )));
        }

        let mut seen = vec![false; pages_authorized as usize];
        let mut s = Self {
            pages_authorized,
            pages_processed: 0,
            pages_failed: 0,
            pages_unsupported: 0,
            pages_quarantined: 0,
            pages_not_attempted: 0,
        };

        for entry in states {
            if entry.index == 0 || entry.index > pages_authorized {
                return Err(malformed(format!(
                    "page index {} is outside 1..={pages_authorized}; page numbers are 1-based",
                    entry.index
                )));
            }
            let slot = &mut seen[(entry.index - 1) as usize];
            if *slot {
                return Err(malformed(format!(
                    "page {} has more than one disposition",
                    entry.index
                )));
            }
            *slot = true;

            match entry.state {
                PageState::Processed => s.pages_processed += 1,
                PageState::Failed(_) => s.pages_failed += 1,
                PageState::Unsupported(_) => s.pages_unsupported += 1,
                PageState::Quarantined(_) => s.pages_quarantined += 1,
                PageState::NotAttempted(_) => s.pages_not_attempted += 1,
            }
        }

        debug_assert!(s.reconciles(), "construction guarantees reconciliation");
        Ok(s)
    }

    /// Whether the buckets sum to the authorized count.
    pub fn reconciles(&self) -> bool {
        let sum = u64::from(self.pages_processed)
            + u64::from(self.pages_failed)
            + u64::from(self.pages_unsupported)
            + u64::from(self.pages_quarantined)
            + u64::from(self.pages_not_attempted);
        sum == u64::from(self.pages_authorized)
    }

    /// Pages that did not reach [`PageState::Processed`].
    pub fn pages_not_processed(&self) -> u32 {
        self.pages_authorized.saturating_sub(self.pages_processed)
    }

    /// Whether every authorized page was processed.
    pub fn is_fully_processed(&self) -> bool {
        self.reconciles() && self.pages_not_processed() == 0
    }
}

/// Where the gap is, when processing did not complete.
///
/// A **pointer** to the gaps, not a copy of them: the full disposition is in the page-state list,
/// and duplicating 484 skipped page numbers into the terminal state would make a bounded run's
/// output scale with the page count it exists to avoid scaling with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessingGaps {
    /// How many authorized pages did not reach [`PageState::Processed`].
    pub pages_not_processed: u32,
    /// The lowest 1-based page index that did not.
    pub first_gap_page: u32,
}

/// How a processing run ended. **Partial is a terminal state, not a degraded success.**
///
/// `docs/01-CONTRACT.md` §7: a representation where some pages failed may still support claims
/// binding to pages that succeeded — *but the gap must be visible to every consumer*. Making
/// this an enum rather than a boolean is what stops "complete" from being the default a caller
/// gets by not looking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "state",
    content = "value",
    deny_unknown_fields
)]
pub enum ProcessingTerminalState {
    /// Every authorized page was processed.
    Complete,
    /// At least one authorized page was not.
    Partial(ProcessingGaps),
    /// The source could not be opened or parsed, so no page was ever authorized.
    ///
    /// **Not reachable from an emitted v0 artifact**, and that is deliberate rather than an
    /// oversight: a hard open failure (encrypted, bad magic, a malformed xref) exits 2 with a
    /// named error and *no body at all*, because a body would be a representation of a document
    /// nobody read. The variant exists so a caller modelling outcomes has a name for that case
    /// and does not reach for `Partial`, which means something materially different — refusal is
    /// "I read nothing", partial is "I read some of it and here is exactly which".
    Refused(RefusalCode),
}

impl ProcessingTerminalState {
    /// Derive the terminal state from a coverage summary and its page states.
    ///
    /// Private-by-discipline in spirit: [`Assurance::new`] is the only caller, so an artifact
    /// cannot be handed a terminal state that disagrees with its own coverage.
    fn derive(coverage: &CoverageSummary, states: &[PageStateEntry]) -> Self {
        if coverage.is_fully_processed() {
            return Self::Complete;
        }
        let first_gap_page = states
            .iter()
            .filter(|e| !e.state.is_processed())
            .map(|e| e.index)
            .min()
            // Unreachable while `states` is the list `coverage` was tallied from: a non-zero
            // not-processed count means at least one such entry exists. Reported as page 0 —
            // an impossible 1-based index — rather than panicking or inventing page 1.
            .unwrap_or(0);
        Self::Partial(ProcessingGaps {
            pages_not_processed: coverage.pages_not_processed(),
            first_gap_page,
        })
    }

    /// Whether the run processed everything it was authorized to.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// Why a run was refused before any page was authorized.
///
/// Mirrors [`EngineError::code`] rather than restating it: the taxonomy already exists and a
/// second, drifting copy of it is worse than a mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalCode {
    /// A format, version, or feature this profile does not claim to handle.
    Unsupported,
    /// The source violates its own format specification.
    Malformed,
    /// The source is encrypted.
    Encrypted,
    /// A declared resource bound was hit before reading began.
    ResourceLimit,
    /// A required structure was absent.
    MissingPart,
    /// The source could not be read at all.
    Io,
}

impl RefusalCode {
    /// The refusal a given error represents.
    ///
    /// The match is **exhaustive on purpose**, with no wildcard arm. `EngineError` is
    /// `#[non_exhaustive]` for downstream crates, but inside this one a new variant must stop
    /// the build until someone decides what it means to a caller — which is the whole reason the
    /// taxonomy exists. A `_ => Unsupported` arm would let a new outcome ship silently wearing
    /// another one's name.
    pub fn of(error: &EngineError) -> Self {
        match error {
            EngineError::Unsupported { .. } => Self::Unsupported,
            EngineError::Malformed { .. } => Self::Malformed,
            EngineError::Encrypted { .. } => Self::Encrypted,
            EngineError::ResourceLimit { .. } => Self::ResourceLimit,
            EngineError::MissingPart { .. } => Self::MissingPart,
            EngineError::Io { .. } => Self::Io,
        }
    }
}

/// The assurance envelope every classify/extract artifact embeds.
///
/// Constructed, never assembled field by field: [`Self::new`] tallies the coverage and derives
/// the terminal state from the page states it is given, so the three cannot disagree with each
/// other. An artifact that claims `Complete` while carrying a quarantined page is not a bug this
/// type can have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assurance {
    /// What the profile that produced this artifact claims it can do.
    pub capabilities: Capabilities,
    /// What it could not do — profile-wide, on this document, or on a page. Canonically ordered.
    pub limitations: Vec<Limitation>,
    /// The authorized/processed reconciliation.
    pub coverage: CoverageSummary,
    /// Every authorized page's disposition, in page order.
    pub page_states: Vec<PageStateEntry>,
    /// How the run ended. Derived from the page states, never asserted independently.
    pub terminal_state: ProcessingTerminalState,
}

impl Assurance {
    /// Build an envelope from a profile's capabilities, a page-state list, and declared gaps.
    ///
    /// The capability-derived limitations are added here, so no caller can emit an artifact whose
    /// `capabilities.tables == false` is not matched by a declared limitation.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] when the page states do not describe exactly the authorized
    /// pages — see [`CoverageSummary::from_page_states`].
    pub fn new(
        capabilities: Capabilities,
        pages_authorized: u32,
        mut page_states: Vec<PageStateEntry>,
        extra_limitations: Vec<Limitation>,
    ) -> Result<Self, EngineError> {
        page_states.sort_by_key(|e| e.index);
        let coverage = CoverageSummary::from_page_states(pages_authorized, &page_states)?;
        let terminal_state = ProcessingTerminalState::derive(&coverage, &page_states);

        let mut limitations = capabilities.declared_limitations();
        limitations.extend(extra_limitations);
        Limitation::normalize(&mut limitations);

        Ok(Self {
            capabilities,
            limitations,
            coverage,
            page_states,
            terminal_state,
        })
    }

    /// Whether this artifact may be presented as a complete reading of its source.
    ///
    /// The one question a consumer must ask before treating an artifact as the whole document.
    pub fn is_complete(&self) -> bool {
        self.terminal_state.is_complete()
    }

    /// What a query binding to `page` may honestly conclude. See [`page_binding_status`].
    pub fn page_binding_status(&self, page: u32) -> PageBindingResult {
        page_binding_status(&self.coverage, &self.page_states, page)
    }

    /// Whether every gap names a limitation this artifact actually declares.
    ///
    /// The cross-check that keeps page states and limitations from drifting: a page pointing at
    /// `resource-limit-pages` is only informative if that code is in [`Self::limitations`] with
    /// prose attached.
    pub fn every_gap_names_a_declared_limitation(&self) -> bool {
        self.page_states
            .iter()
            .all(|e| match e.state.limitation_code() {
                None => true,
                Some(code) => self.limitations.iter().any(|l| l.code == code),
            })
    }
}

/// What a claim binding to a given page may honestly conclude.
///
/// There is deliberately **no "not present" variant**. Workbench rule 4: absence of extractable
/// content is never evidence of absence in the source. A caller that wants to say "the document
/// does not contain X" must reach that conclusion from [`Self::Ok`] pages only; every other
/// answer here carries the reason the engine cannot help.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageBindingResult {
    /// The page was processed. A negative search result over it is a real observation.
    Ok,
    /// The page exists and was authorized, but was not processed. **Indeterminate, never
    /// negative** — the limitation code says why.
    CapabilityLimited {
        /// The declared limitation explaining the gap.
        limitation_code: String,
    },
    /// The page is outside the document this run read.
    ///
    /// A structural fact about the source, not a processing gap: page 900 of a 3-page document
    /// was never authorized because it does not exist. Distinct from `CapabilityLimited` on
    /// purpose — conflating "we skipped it" with "there is no such page" is the same category
    /// error as conflating "we could not read it" with "it does not say that".
    NotInDocument,
}

impl PageBindingResult {
    /// Whether this result supports a positive or negative conclusion about page content.
    pub fn is_determinate(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

/// What a query binding to `page` (1-based) may conclude, given a run's coverage and states.
///
/// **Never returns a boolean.** The signature is the enforcement: there is no way to express
/// "missing" here, so a caller cannot accidentally turn an unprocessed page into evidence of
/// absence (`docs/01-CONTRACT.md` §7, Workbench rule 4).
pub fn page_binding_status(
    coverage: &CoverageSummary,
    states: &[PageStateEntry],
    page: u32,
) -> PageBindingResult {
    if page == 0 || page > coverage.pages_authorized {
        return PageBindingResult::NotInDocument;
    }
    match states.iter().find(|e| e.index == page) {
        Some(entry) => match entry.state.limitation_code() {
            None => PageBindingResult::Ok,
            Some(code) => PageBindingResult::CapabilityLimited {
                limitation_code: code.to_string(),
            },
        },
        // Authorized but undescribed. Unreachable through `Assurance::new`, which refuses a
        // state list that does not cover every authorized page — and if it were ever reachable,
        // "we have no record of what happened to this page" is capability-limited, not fine.
        None => PageBindingResult::CapabilityLimited {
            limitation_code: "page-state-unrecorded".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c14n::c14n_bytes;

    fn states(v: &[(u32, PageState)]) -> Vec<PageStateEntry> {
        v.iter()
            .map(|(index, state)| PageStateEntry {
                index: *index,
                state: state.clone(),
            })
            .collect()
    }

    fn processed(n: u32) -> Vec<PageStateEntry> {
        (1..=n)
            .map(|index| PageStateEntry {
                index,
                state: PageState::Processed,
            })
            .collect()
    }

    #[test]
    fn a_clean_document_still_carries_a_coverage_object() {
        let a = Assurance::new(Capabilities::V0, 3, processed(3), Vec::new()).unwrap();
        assert_eq!(a.coverage.pages_authorized, 3);
        assert_eq!(a.coverage.pages_processed, 3);
        assert_eq!(a.coverage.pages_not_attempted, 0);
        assert!(a.is_complete());

        // The object exists in the bytes even when every gap bucket is zero. "No gaps declared"
        // and "gaps not tracked" must not look the same on the wire.
        let s = String::from_utf8(c14n_bytes(&serde_json::to_value(&a).unwrap()).unwrap()).unwrap();
        assert!(s.contains("\"coverage\""), "{s}");
        assert!(s.contains("\"pages_failed\":0"), "{s}");
    }

    #[test]
    fn the_buckets_always_reconcile_with_the_authorized_count() {
        let s = states(&[
            (1, PageState::Processed),
            (2, PageState::Failed("x".into())),
            (3, PageState::Unsupported("x".into())),
            (4, PageState::Quarantined("x".into())),
            (5, PageState::NotAttempted("x".into())),
        ]);
        let c = CoverageSummary::from_page_states(5, &s).unwrap();
        assert!(c.reconciles());
        assert_eq!(c.pages_processed, 1);
        assert_eq!(c.pages_failed, 1);
        assert_eq!(c.pages_unsupported, 1);
        assert_eq!(c.pages_quarantined, 1);
        assert_eq!(c.pages_not_attempted, 1);
        assert_eq!(c.pages_not_processed(), 4);
        assert!(!c.is_fully_processed());
    }

    #[test]
    fn a_state_list_that_does_not_describe_the_document_is_refused() {
        // Too few.
        assert!(CoverageSummary::from_page_states(3, &processed(2)).is_err());
        // Duplicate index.
        let dup = states(&[(1, PageState::Processed), (1, PageState::Processed)]);
        assert!(CoverageSummary::from_page_states(2, &dup).is_err());
        // Zero index — pages are 1-based.
        let zero = states(&[(0, PageState::Processed), (1, PageState::Processed)]);
        assert!(CoverageSummary::from_page_states(2, &zero).is_err());
        // Past the end.
        let past = states(&[(1, PageState::Processed), (9, PageState::Processed)]);
        assert!(CoverageSummary::from_page_states(2, &past).is_err());
    }

    #[test]
    fn partial_processing_cannot_be_presented_as_complete() {
        let mut s = processed(4);
        s[2].state = PageState::Quarantined(codes::RESOURCE_LIMIT_PAGES.into());
        let a = Assurance::new(
            Capabilities::V0,
            4,
            s,
            vec![Limitation::document(codes::RESOURCE_LIMIT_PAGES, "budget")],
        )
        .unwrap();

        assert!(!a.is_complete(), "a gap must not read as a complete run");
        assert_eq!(
            a.terminal_state,
            ProcessingTerminalState::Partial(ProcessingGaps {
                pages_not_processed: 1,
                first_gap_page: 3,
            })
        );
    }

    /// The whole point of deriving rather than accepting the terminal state.
    #[test]
    fn the_terminal_state_cannot_disagree_with_the_coverage() {
        for gap in [
            PageState::Failed("c".into()),
            PageState::Unsupported("c".into()),
            PageState::Quarantined("c".into()),
            PageState::NotAttempted("c".into()),
        ] {
            let mut s = processed(2);
            s[1].state = gap.clone();
            let a = Assurance::new(
                Capabilities::V0,
                2,
                s,
                vec![Limitation::document("c", "detail")],
            )
            .unwrap();
            assert!(!a.is_complete(), "{gap:?} must not produce Complete");
            assert!(!a.coverage.is_fully_processed());
        }
    }

    #[test]
    fn capability_limited_beats_negative_for_every_gap_state() {
        for gap in [
            PageState::Failed("gap-code".into()),
            PageState::Unsupported("gap-code".into()),
            PageState::Quarantined("gap-code".into()),
            PageState::NotAttempted("gap-code".into()),
        ] {
            let mut s = processed(3);
            s[1].state = gap.clone();
            let a = Assurance::new(
                Capabilities::V0,
                3,
                s,
                vec![Limitation::document("gap-code", "detail")],
            )
            .unwrap();

            assert_eq!(a.page_binding_status(1), PageBindingResult::Ok);
            assert_eq!(
                a.page_binding_status(2),
                PageBindingResult::CapabilityLimited {
                    limitation_code: "gap-code".into()
                },
                "{gap:?} must bind capability-limited, never absent"
            );
            assert!(!a.page_binding_status(2).is_determinate());
        }
    }

    #[test]
    fn a_page_outside_the_document_is_not_a_processing_gap() {
        let a = Assurance::new(Capabilities::V0, 2, processed(2), Vec::new()).unwrap();
        assert_eq!(a.page_binding_status(3), PageBindingResult::NotInDocument);
        assert_eq!(a.page_binding_status(0), PageBindingResult::NotInDocument);
        assert_eq!(a.page_binding_status(2), PageBindingResult::Ok);
    }

    /// An unrecorded page is indeterminate, not fine.
    #[test]
    fn an_unrecorded_authorized_page_is_capability_limited() {
        // Constructed by hand: `Assurance::new` refuses this shape, which is the point.
        let coverage = CoverageSummary {
            pages_authorized: 3,
            pages_processed: 2,
            pages_failed: 0,
            pages_unsupported: 0,
            pages_quarantined: 0,
            pages_not_attempted: 1,
        };
        let s = processed(2);
        assert!(!page_binding_status(&coverage, &s, 3).is_determinate());
    }

    #[test]
    fn every_false_capability_declares_a_limitation() {
        let none = Capabilities {
            spans: false,
            char_offsets: false,
            tables: false,
            measured_ink_boxes: false,
            multi_column_reading_order: false,
            structural_locators: false,
        };
        let declared = none.declared_limitations();
        for code in [
            codes::SPANS_NOT_EMITTED,
            codes::CHAR_OFFSETS_NOT_EMITTED,
            codes::TABLES_NOT_EXTRACTED,
            codes::MEASURED_INK_BOXES_NOT_EMITTED,
            codes::MULTI_COLUMN_READING_ORDER,
            codes::STRUCTURAL_LOCATORS_NOT_CLAIMED,
        ] {
            assert!(
                declared.iter().any(|l| l.code == code),
                "`{code}` must be declared when its capability is false"
            );
        }
        assert_eq!(
            declared.len(),
            6,
            "one limitation per false capability, plus none for the true ones"
        );

        // The mirror, with one deliberate exception. A profile claiming everything declares no
        // *false-capability* limitations — but `tables: true` still declares the SCOPE of what it
        // looked for, because "the detector found no ruled table" and "this page has no table"
        // are different statements and only the first one is true (v1-S1).
        let all = Capabilities {
            spans: true,
            char_offsets: true,
            tables: true,
            measured_ink_boxes: true,
            multi_column_reading_order: true,
            structural_locators: true,
        };
        let all_declared = all.declared_limitations();
        let remaining: Vec<&str> = all_declared.iter().map(|l| l.code.as_str()).collect();
        assert_eq!(
            remaining,
            vec![codes::UNRULED_TABLES_NOT_DETECTED],
            "the only limitation an all-true profile keeps is the ruled-only scope"
        );
    }

    #[test]
    fn v0_declares_the_multi_column_limitation() {
        let a = Assurance::new(Capabilities::V0, 1, processed(1), Vec::new()).unwrap();
        let l = a
            .limitations
            .iter()
            .find(|l| l.code == codes::MULTI_COLUMN_READING_ORDER)
            .expect("v0 reads single-column and must say so");
        assert_eq!(l.scope, LimitationScope::Profile);
        assert!(l.detail.contains("WRONG ORDER"));
    }

    #[test]
    fn every_gap_state_points_at_a_declared_limitation() {
        let mut s = processed(2);
        s[0].state = PageState::Quarantined(codes::RESOURCE_LIMIT_PAGES.into());
        let with = Assurance::new(
            Capabilities::V0,
            2,
            s.clone(),
            vec![Limitation::document(codes::RESOURCE_LIMIT_PAGES, "budget")],
        )
        .unwrap();
        assert!(with.every_gap_names_a_declared_limitation());

        let without = Assurance::new(Capabilities::V0, 2, s, Vec::new()).unwrap();
        assert!(
            !without.every_gap_names_a_declared_limitation(),
            "a page pointing at an undeclared code is exactly the drift this check exists for"
        );
    }

    #[test]
    fn limitations_are_canonically_ordered_and_deduplicated() {
        let mut list = vec![
            Limitation::page(2, "b", "d"),
            Limitation::document("a", "d"),
            Limitation::profile("z", "d"),
            Limitation::document("a", "d"),
            Limitation::page(1, "b", "d"),
        ];
        Limitation::normalize(&mut list);
        assert_eq!(list.len(), 4, "the duplicate is gone");
        let scopes: Vec<LimitationScope> = list.iter().map(|l| l.scope).collect();
        assert_eq!(
            scopes,
            vec![
                LimitationScope::Document,
                LimitationScope::Page(1),
                LimitationScope::Page(2),
                LimitationScope::Profile,
            ],
            "sorted by code first, then scope: a(document), b(page 1), b(page 2), z(profile)"
        );
    }

    #[test]
    fn the_envelope_is_order_stable_regardless_of_input_order() {
        let a = Assurance::new(
            Capabilities::V0,
            3,
            states(&[
                (3, PageState::Processed),
                (1, PageState::Processed),
                (2, PageState::Processed),
            ]),
            Vec::new(),
        )
        .unwrap();
        let indices: Vec<u32> = a.page_states.iter().map(|e| e.index).collect();
        assert_eq!(
            indices,
            vec![1, 2, 3],
            "page states are emitted in page order"
        );

        let b = Assurance::new(Capabilities::V0, 3, processed(3), Vec::new()).unwrap();
        assert_eq!(
            c14n_bytes(&serde_json::to_value(&a).unwrap()).unwrap(),
            c14n_bytes(&serde_json::to_value(&b).unwrap()).unwrap()
        );
    }

    #[test]
    fn refusal_names_the_error_it_came_from() {
        assert_eq!(
            RefusalCode::of(&EngineError::Encrypted { detail: "x".into() }),
            RefusalCode::Encrypted
        );
        assert_eq!(
            RefusalCode::of(&EngineError::Malformed {
                what: "xref entry".into(),
                detail: "19 bytes, expected 20".into(),
            }),
            RefusalCode::Malformed
        );
        // Refusal is not partial: nothing was read, so there is no gap to point at.
        let refused = ProcessingTerminalState::Refused(RefusalCode::Encrypted);
        assert!(!refused.is_complete());
        assert!(!matches!(refused, ProcessingTerminalState::Partial(_)));
    }

    #[test]
    fn the_envelope_round_trips_through_c14n() {
        let mut s = processed(3);
        s[1].state = PageState::NotAttempted(codes::RESOURCE_LIMIT_PAGES.into());
        let a = Assurance::new(
            Capabilities::V0,
            3,
            s,
            vec![Limitation::page(2, codes::RESOURCE_LIMIT_PAGES, "budget")],
        )
        .unwrap();
        let bytes = c14n_bytes(&serde_json::to_value(&a).unwrap()).unwrap();
        let back: Assurance = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn every_assurance_type_refuses_an_unknown_field() {
        let cases: Vec<serde_json::Value> = vec![
            serde_json::json!({"code":"c","detail":"d","scope":{"kind":"profile"},"extra":1}),
            serde_json::json!({"index":1,"state":{"state":"processed"},"extra":1}),
        ];
        assert!(serde_json::from_value::<Limitation>(cases[0].clone()).is_err());
        assert!(serde_json::from_value::<PageStateEntry>(cases[1].clone()).is_err());

        // Guard the guard: the exact shapes still parse.
        assert!(serde_json::from_value::<Limitation>(
            serde_json::json!({"code":"c","detail":"d","scope":{"kind":"page","value":2}})
        )
        .is_ok());
    }

    /// Absence is expressed by a variant, never by an implied full value.
    ///
    /// `docs/01-CONTRACT.md` §9.1: *a processor that reports no uncertainty produces an absent
    /// field, never an implied `1.0`.* v0 has no uncertainty to report, so the check is that no
    /// numeric field appears anywhere in this envelope that could be read as one — every number
    /// it carries is a page count.
    #[test]
    fn nothing_here_carries_an_implied_full_value() {
        let a = Assurance::new(Capabilities::V0, 1, processed(1), Vec::new()).unwrap();
        let s = String::from_utf8(c14n_bytes(&serde_json::to_value(&a).unwrap()).unwrap()).unwrap();

        // Prose legitimately contains full stops; a bare decimal point outside a string means a
        // float reached canonical output, which c14n should already have refused.
        let mut in_string = false;
        let chars: Vec<char> = s.chars().collect();
        for i in 0..chars.len() {
            match chars[i] {
                '"' if i == 0 || chars[i - 1] != '\\' => in_string = !in_string,
                '.' if !in_string => panic!("a float reached the assurance envelope: {s}"),
                _ => {}
            }
        }
    }
}
