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

//! Profile-as-identity (`docs/01-CONTRACT.md` §2, `docs/04-ARCHITECTURE.md` §3).
//!
//! The profile is every knob that can change output. Its hash goes on every artifact, and two
//! artifacts are comparable if and only if their `profile_sha256` matches.
//!
//! This is the cheapest load-bearing idea in the design: one hash buys OCR isolation, backend
//! isolation, and comparability, with no other machinery. An OCR run gets a different profile,
//! so its output is non-comparable with a born-digital parse *by contract* rather than by
//! anyone remembering to check.
//!
//! **Anything that can change a byte of output belongs here or is a bug.** The sensitivity test
//! destructures `Profile` exhaustively, so adding a field without covering it fails to compile.

use serde::{Deserialize, Serialize};

use crate::c14n::{c14n_bytes, C14nError};
use crate::geom::QUANTUM_PER_POINT;
use crate::identity::{CoordinateSystem, Sha256Hex};

/// The reading-order rule v0 ships: single column, no multi-column detection.
///
/// Versioned as a string because the *rule* is part of identity. pdf-inspector's multi-column
/// detection flips on `min_lines < 15`, so a one-line document edit reorders the whole page; a
/// cliff-shaped heuristic cannot sit under a determinism contract, and when a stable rule lands
/// it gets a new id here rather than silently replacing this one.
pub const READING_ORDER_RULE_V0: &str = "single-column-v1";

/// Identity of the character-decoding data this profile carries.
///
/// Names what is **actually** vendored rather than what was planned. At M3 that is the
/// PDF 32000-1 Annex D encoding tables — `WinAnsiEncoding`, the ASCII range of
/// `StandardEncoding`, and a glyph-name subset — held in `engine-pdf`'s `encoding` module.
///
/// The Adobe predefined CJK CMaps are **not** carried, so a document naming one is refused
/// rather than decoded approximately. That is a declared limitation, recorded in every extract
/// artifact's `not_decoded` list and in `vendor/README.md`. When those files land this string
/// changes, which moves `profile_sha256` — artifacts from before and after are then correctly
/// non-comparable, because they really were produced by different decoders.
pub const CMAP_DATA_VERSION: &str = "annex-d-encodings-1";

/// Identity of the object/xref backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendIdentity {
    /// Backend crate or library name.
    pub name: String,
    /// Exact version string.
    pub version: String,
}

impl Default for BackendIdentity {
    /// The v0 backend, resolved.
    ///
    /// The version moving changes the profile hash, which is the event we want visible: two
    /// artifacts produced by different backend builds are correctly non-comparable.
    fn default() -> Self {
        Self {
            name: "lopdf".into(),
            // The resolved `lopdf` version, wired in at M2 when the dependency landed. Bumping
            // the crate moves this string, which moves `profile_sha256` — which is the point:
            // a backend change is fingerprint-visible rather than silent.
            version: "0.44.0".into(),
        }
    }
}

/// What this profile claims it can do.
///
/// The first three mirror `ethos.grounding.v1`'s required `capabilities` object exactly, so the
/// M5 projection is a move rather than a translation. The rest are representation-level and
/// have no grounding counterpart yet.
///
/// L1's achievement condition names capability declarations explicitly, so an artifact without
/// them has not reached "extracted" regardless of how good its text is.
///
/// # Two rules bind this type, in both directions
///
/// 1. **No `true` without a proof test.** `crates/engine-pdf/tests/capabilities.rs` maps every
///    `true` field to a named test and fails the build when one is missing — a capability
///    asserted without a passing test is the failure mode this type exists to prevent.
/// 2. **No `false` without a declared limitation.** [`Capabilities::declared_limitations`]
///    derives one per `false` field, exhaustively, so a caller never has to infer a gap from
///    silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// Text spans are emitted. (grounding-aligned)
    pub spans: bool,
    /// Spans carry character offsets into their element's text. (grounding-aligned)
    ///
    /// **v0: false.** M5 built `DocumentRepresentation v0` and left this false on purpose: v0
    /// performs no line or block grouping, so an element and a span are the *same object* and an
    /// offset would always be `0..len` — advertising sub-element addressing the engine cannot do.
    /// Ethos's own validator also ties the two together (offsets present must equal the
    /// capability), so claiming it would oblige every span to carry them. It flips at v1, when
    /// grouping makes elements coarser than spans and the offsets start carrying information.
    pub char_offsets: bool,
    /// Tables are detected and emitted. **v0: false** — tables are M-later, v1 scope.
    /// (grounding-aligned)
    pub tables: bool,
    /// Ink boxes come from measured font metrics rather than being absent.
    pub measured_ink_boxes: bool,
    /// Multi-column reading order is detected. **v0: false**, and the corresponding limitation
    /// is declared on every artifact.
    pub multi_column_reading_order: bool,
    /// Structural locators (`mcid`, tagged-structure roles) are captured.
    ///
    /// **v0: false**, and this one is a judgement call worth recording. A marked-content id
    /// *is* captured where a page's content stream supplies one via `BDC`, and never invented
    /// where it does not — but the tagged-structure tree is not read, so there is no role path,
    /// and an absent `mcid` is not evidence the document is untagged. Claiming the capability
    /// on the strength of the partial half would promise an address consumers could not rely on.
    pub structural_locators: bool,
}

impl Capabilities {
    /// What v0 actually claims.
    ///
    /// Note how much is `false`. Two of these were `true` in the M1 sketch and are `false` here
    /// because M4 asked for the proof and the proof did not exist: `char_offsets` has no
    /// hierarchy to index into until grouping lands, and `structural_locators` would be claiming a full
    /// structural address on the strength of a best-effort `mcid`. Narrowing a declaration when
    /// the evidence does not support it is the mechanism working, not a regression.
    pub const V0: Self = Self {
        spans: true,
        char_offsets: false,
        tables: false,
        measured_ink_boxes: true,
        multi_column_reading_order: false,
        structural_locators: false,
    };
}

/// How many pages a run may process before it stops.
///
/// A **declared state, not an absent field**: `{"mode":"unlimited"}` says the budget was
/// considered and found unbounded, where an omitted `Option` would leave a reader unable to
/// distinguish "unbounded" from "this build has no such knob". It is also the difference between
/// a profile that round-trips honestly and one that does not — an `Option` field missing from
/// incoming JSON deserializes to `None` and silently re-hashes as though it had been there.
///
/// # Why a page count is the right knob
///
/// `failure/memory-limit-simulated` is **byte-identical to `synthetic/simple-text`** (both
/// `sha256:f2f6ab91…`). The fixture name means *the limit is simulated by configuration*, not
/// *this PDF is huge* — so a meaningful test must set the budget explicitly. A page count is the
/// bound that actually governs peak cost in a page-at-a-time reader, which is why it is the knob
/// rather than a byte ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "mode",
    content = "pages",
    deny_unknown_fields
)]
pub enum PageBudget {
    /// No page bound. The v0 default.
    Unlimited,
    /// Process at most this many pages; the rest are quarantined and declared.
    ///
    /// Zero is legal and means "process none" — the only budget that can bite on a one-page
    /// document, and therefore the one `memory-limit-simulated` needs.
    AtMost(u32),
}

impl PageBudget {
    /// The bound as a count, or `None` when unbounded.
    pub fn max_pages_to_process(self) -> Option<u32> {
        match self {
            Self::Unlimited => None,
            Self::AtMost(n) => Some(n),
        }
    }

    /// Whether a **1-based** page number is inside the budget.
    pub fn admits(self, page: u32) -> bool {
        match self {
            Self::Unlimited => true,
            Self::AtMost(n) => page <= n,
        }
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::V0
    }
}

/// The pinned configuration whose hash is the engine's identity.
///
/// Host-varying data (paths, timings, thread counts, machine identity) is deliberately absent:
/// including it would make every machine produce a different fingerprint for identical work,
/// which is the opposite of what this is for.
/// `deny_unknown_fields` is load-bearing, not tidiness. Without it, a profile written by a newer
/// engine — carrying a knob this build does not know about — would deserialize with that knob
/// silently dropped and then **re-hash to a different digest than the one it arrived with**. The
/// artifact would claim comparability it does not have. Failing closed on an unrecognised field
/// is `docs/01-CONTRACT.md` §8 applied to our own artifacts, which is where it matters most.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// The engine build that this profile describes.
    pub parser_version: String,
    /// Object/xref backend identity and version.
    pub backend: BackendIdentity,
    /// How many pages classification samples before it stops. Default 8.
    ///
    /// On the profile because changing it changes classification output. It is also the knob
    /// whose *boundedness* is the point: cost must not scale with total page count.
    pub classify_sample_pages: u32,
    /// Quanta per point. 100 (centipoints).
    pub quantum_per_point: u32,
    /// The declared coordinate system.
    pub coordinate_system: CoordinateSystem,
    /// Declared capabilities.
    pub capabilities: Capabilities,
    /// How many pages any stage may process before it stops. Default [`PageBudget::Unlimited`].
    ///
    /// On the profile because it changes output: a budgeted run emits a **partial** artifact
    /// with quarantined pages and a declared limitation, and an artifact produced under a
    /// different budget is correctly non-comparable with one produced under none.
    pub page_budget: PageBudget,
    /// Version id of the reading-order rule in force.
    pub reading_order_rule: String,
    /// Identity of the vendored character-decoding data. See [`CMAP_DATA_VERSION`].
    pub cmap_data_version: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            parser_version: env!("CARGO_PKG_VERSION").to_string(),
            backend: BackendIdentity::default(),
            classify_sample_pages: 8,
            quantum_per_point: QUANTUM_PER_POINT,
            coordinate_system: CoordinateSystem::V0,
            capabilities: Capabilities::V0,
            page_budget: PageBudget::Unlimited,
            reading_order_rule: READING_ORDER_RULE_V0.to_string(),
            cmap_data_version: CMAP_DATA_VERSION.to_string(),
        }
    }
}

impl Profile {
    /// The canonical bytes this profile hashes over.
    ///
    /// # Errors
    ///
    /// [`C14nError`] if the profile contains a value c14n rejects. Not reachable through the
    /// public API today — every field is a string, bool, or `u32` — but returned rather than
    /// unwrapped so a future field cannot make this panic.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, C14nError> {
        // serde_json::to_value only fails for types this struct does not contain (maps with
        // non-string keys, types whose Serialize impl errors). Reported rather than unwrapped
        // so a future field cannot turn a contract violation into a panic.
        let value = serde_json::to_value(self)
            .map_err(|e| C14nError::new(format!("profile is not serializable: {e}")))?;
        c14n_bytes(&value)
    }

    /// `sha256:<hex>` over [`Self::canonical_bytes`].
    ///
    /// # Errors
    ///
    /// Propagates [`C14nError`].
    pub fn profile_sha256(&self) -> Result<Sha256Hex, C14nError> {
        let bytes = self.canonical_bytes()?;
        let hex = crate::c14n::sha256_hex_bytes(&bytes);
        // `from_hex` can only fail on a malformed digest, which sha256 cannot produce.
        Ok(Sha256Hex::from_hex(&hex).expect("sha256 hex is always 64 lowercase hex digits"))
    }
}

/// Convenience wrapper matching the API named in `docs/05-MILESTONES.md` M1.
///
/// # Errors
///
/// Propagates [`C14nError`].
pub fn profile_sha256(profile: &Profile) -> Result<Sha256Hex, C14nError> {
    profile.profile_sha256()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mutate one field, hash, compare, restore.
    fn hash(p: &Profile) -> String {
        p.profile_sha256().unwrap().to_string()
    }

    /// Every field on `Profile` must change the hash when it changes.
    ///
    /// The destructuring binding below is the enforcement mechanism: adding a field to
    /// `Profile` without adding a case here is a **compile error**, not a silently uncovered
    /// knob. A knob that does not move the hash is a silent-drift bug waiting to happen.
    #[test]
    fn every_profile_field_is_hash_sensitive() {
        let base = Profile::default();
        let base_hash = hash(&base);

        // Exhaustiveness gate. If this fails to compile, a field was added — cover it below.
        //
        // Destructured to the LEAF, not just the top level: a field added to `Capabilities` or
        // `BackendIdentity` is as output-affecting as one added to `Profile`, and a gate that
        // stopped at `capabilities: _` would wave it straight through.
        let Profile {
            parser_version: _,
            backend:
                BackendIdentity {
                    name: _,
                    version: _,
                },
            classify_sample_pages: _,
            quantum_per_point: _,
            coordinate_system: CoordinateSystem { unit: _, origin: _ },
            capabilities:
                Capabilities {
                    spans: _,
                    char_offsets: _,
                    tables: _,
                    measured_ink_boxes: _,
                    multi_column_reading_order: _,
                    structural_locators: _,
                },
            page_budget: _,
            reading_order_rule: _,
            cmap_data_version: _,
        } = &base;

        /// One named single-field mutation.
        type Mutation = (&'static str, Box<dyn Fn(&mut Profile)>);

        let mutations: Vec<Mutation> = vec![
            (
                "parser_version",
                Box::new(|p: &mut Profile| p.parser_version = "9.9.9-mutated".into()),
            ),
            (
                "backend.name",
                Box::new(|p: &mut Profile| p.backend.name = "other-backend".into()),
            ),
            (
                "backend.version",
                Box::new(|p: &mut Profile| p.backend.version = "1.2.3-mutated".into()),
            ),
            (
                "classify_sample_pages",
                Box::new(|p: &mut Profile| p.classify_sample_pages = 16),
            ),
            (
                "quantum_per_point",
                Box::new(|p: &mut Profile| p.quantum_per_point = 1000),
            ),
            (
                "capabilities.spans",
                Box::new(|p: &mut Profile| p.capabilities.spans = false),
            ),
            (
                // Mutated toward `true`: `char_offsets` is false in V0, and a mutation to the
                // value a field already holds tests nothing.
                "capabilities.char_offsets",
                Box::new(|p: &mut Profile| p.capabilities.char_offsets = true),
            ),
            (
                "capabilities.tables",
                Box::new(|p: &mut Profile| p.capabilities.tables = true),
            ),
            (
                "capabilities.measured_ink_boxes",
                Box::new(|p: &mut Profile| p.capabilities.measured_ink_boxes = false),
            ),
            (
                "capabilities.multi_column_reading_order",
                Box::new(|p: &mut Profile| p.capabilities.multi_column_reading_order = true),
            ),
            (
                "capabilities.structural_locators",
                Box::new(|p: &mut Profile| p.capabilities.structural_locators = true),
            ),
            (
                "page_budget",
                Box::new(|p: &mut Profile| p.page_budget = PageBudget::AtMost(4)),
            ),
            (
                // Zero is a distinct budget from four, and from unlimited. A knob whose
                // *value* did not move the hash would be as bad as one whose presence did not.
                "page_budget.pages",
                Box::new(|p: &mut Profile| p.page_budget = PageBudget::AtMost(0)),
            ),
            (
                "reading_order_rule",
                Box::new(|p: &mut Profile| p.reading_order_rule = "xy-cut-v1".into()),
            ),
            (
                "cmap_data_version",
                Box::new(|p: &mut Profile| p.cmap_data_version = "adobe-2026-01".into()),
            ),
        ];

        let mut seen = std::collections::BTreeSet::new();
        seen.insert(base_hash.clone());

        for (name, mutate) in mutations {
            let mut p = Profile::default();
            mutate(&mut p);

            // Guard the guard. A "mutation" that writes back the value the field already holds
            // proves nothing about hash sensitivity while passing every assertion below — which
            // is exactly what happened to `capabilities.char_offsets` when its V0 value flipped
            // to `false` at M4 and the mutation still set `false`.
            assert_ne!(
                p, base,
                "the {name} mutation left the profile unchanged, so it tests nothing"
            );

            let h = hash(&p);
            assert_ne!(
                h, base_hash,
                "mutating {name} did not change profile_sha256"
            );
            assert!(
                seen.insert(h),
                "mutating {name} collided with another profile's hash"
            );
        }
    }

    #[test]
    fn coordinate_system_is_hash_sensitive_in_principle() {
        // Only one variant exists today, so this cannot be mutated into a different value. The
        // test records that the field IS hashed, by asserting it appears in canonical bytes —
        // otherwise adding a second origin later would silently not change identity.
        let bytes = Profile::default().canonical_bytes().unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("\"coordinate_system\":"), "field must be hashed");
        assert!(s.contains("\"origin\":\"top-left\""));
        assert!(s.contains("\"unit\":\"centipoint\""));
    }

    /// The default profile's canonical bytes and hash, pinned.
    ///
    /// Not redundant with the sensitivity test: that one proves a change is *detectable*, this
    /// one proves a change was *intended*. The v0 profile is the engine's identity, so it moves
    /// only in a commit that says so — the same discipline Ethos applies to its own profile
    /// artifact.
    #[test]
    fn the_default_profile_is_pinned() {
        let bytes = Profile::default().canonical_bytes().unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            r#"{"backend":{"name":"lopdf","version":"0.44.0"},"capabilities":{"char_offsets":false,"measured_ink_boxes":true,"multi_column_reading_order":false,"spans":true,"structural_locators":false,"tables":false},"classify_sample_pages":8,"cmap_data_version":"annex-d-encodings-1","coordinate_system":{"origin":"top-left","unit":"centipoint"},"page_budget":{"mode":"unlimited"},"parser_version":"0.0.0","quantum_per_point":100,"reading_order_rule":"single-column-v1"}"#,
            "the v0 profile changed. Expected causes: a crate version bump (parser_version is \
             part of identity, so a new build IS a new profile — that is by design), or a new \
             field. Update this vector and say why in the commit. Unexpected cause: something \
             added an output-affecting knob by accident.\n\n\
             Moved deliberately at M4, twice: `capabilities.char_offsets` true -> false (v0 \
             emits no element/span hierarchy for an offset to index into; it lands at M5), and \
             the new `page_budget` knob. Artifacts from before and after are correctly \
             non-comparable, because the profile that produced them really did change."
        );
        assert_eq!(
            Profile::default().profile_sha256().unwrap().to_string(),
            "sha256:f34be6328f858e09c241cb51c7b0dbb0fe065ecbf5f00e9942cd6bbcdd6faf1e"
        );
    }

    #[test]
    fn the_hash_is_stable_across_calls_and_clones() {
        let p = Profile::default();
        assert_eq!(hash(&p), hash(&p));
        assert_eq!(hash(&p), hash(&p.clone()));
    }

    #[test]
    fn the_hash_is_well_formed() {
        let d = Profile::default().profile_sha256().unwrap();
        assert!(d.as_str().starts_with("sha256:"));
        assert_eq!(d.hex().len(), 64);
        assert!(d
            .hex()
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    }

    #[test]
    fn the_free_function_matches_the_method() {
        let p = Profile::default();
        assert_eq!(profile_sha256(&p).unwrap(), p.profile_sha256().unwrap());
    }

    #[test]
    fn v0_capabilities_are_honest_about_what_is_missing() {
        let c = Capabilities::V0;
        assert!(!c.tables, "tables are not v0 scope");
        assert!(
            !c.multi_column_reading_order,
            "v0 reads single-column and declares the limitation"
        );
        assert!(
            !c.char_offsets,
            "v0 emits runs with no element/span hierarchy, so there is nothing an offset could \
             index into. M5 built the record and left this false: with no line grouping an \
             element and a span are the same object, so an offset would always be 0..len. It \
             flips at v1 with grouping — and with a test"
        );
        assert!(
            !c.structural_locators,
            "an `mcid` captured from BDC is not a structural address: no role path, and an \
             absent id is not evidence the document is untagged"
        );
    }

    #[test]
    fn the_default_budget_is_unlimited_and_says_so_on_the_wire() {
        assert_eq!(Profile::default().page_budget, PageBudget::Unlimited);
        assert_eq!(PageBudget::Unlimited.max_pages_to_process(), None);
        assert_eq!(PageBudget::AtMost(3).max_pages_to_process(), Some(3));

        // Declared, not omitted: a reader must not have to infer "unbounded" from a missing key.
        let s = String::from_utf8(Profile::default().canonical_bytes().unwrap()).unwrap();
        assert!(s.contains(r#""page_budget":{"mode":"unlimited"}"#), "{s}");
    }

    #[test]
    fn a_budget_admits_pages_up_to_its_bound_and_no_further() {
        assert!(PageBudget::Unlimited.admits(1));
        assert!(PageBudget::Unlimited.admits(u32::MAX));

        let b = PageBudget::AtMost(2);
        assert!(b.admits(1), "pages are 1-based");
        assert!(b.admits(2));
        assert!(!b.admits(3));

        // Zero admits nothing — the only budget that bites on a one-page document.
        assert!(!PageBudget::AtMost(0).admits(1));
    }

    #[test]
    fn canonical_bytes_contain_no_host_varying_data() {
        let s = String::from_utf8(Profile::default().canonical_bytes().unwrap()).unwrap();
        for forbidden in [
            "/Users",
            "/home",
            "/tmp",
            "timestamp",
            "elapsed",
            "hostname",
        ] {
            assert!(
                !s.contains(forbidden),
                "profile must not carry host-varying data, found {forbidden}: {s}"
            );
        }
    }

    /// An unknown field is refused rather than dropped.
    ///
    /// The failure this prevents: a profile from a newer engine deserializes with its unknown
    /// knob silently discarded, then re-hashes to a *different* digest than it arrived with —
    /// so an artifact claims comparability it does not have.
    #[test]
    fn an_unknown_profile_field_fails_closed() {
        let mut v = serde_json::to_value(Profile::default()).unwrap();
        v.as_object_mut()
            .unwrap()
            .insert("future_knob".into(), serde_json::Value::from(1));
        let bytes = crate::c14n::c14n_bytes(&v).unwrap();

        let parsed: Result<Profile, _> = serde_json::from_slice(&bytes);
        assert!(
            parsed.is_err(),
            "a profile carrying an unknown field must be refused, not silently truncated"
        );
    }

    /// Every nested object in `Profile`, not just the ones someone remembered.
    ///
    /// `deny_unknown_fields` is **not recursive**. This test originally covered `capabilities`
    /// and `backend` and missed `coordinate_system`, which left the exact hole it was written to
    /// prevent: a profile carrying `coordinate_system.future_knob` parsed cleanly and re-hashed
    /// to the unmodified default digest. The list below is derived from the serialized value, so
    /// a nested object added later is covered automatically rather than by memory.
    #[test]
    fn an_unknown_nested_field_also_fails_closed() {
        let default = serde_json::to_value(Profile::default()).unwrap();
        let nested: Vec<String> = default
            .as_object()
            .expect("profile is an object")
            .iter()
            .filter(|(_, v)| v.is_object())
            .map(|(k, _)| k.clone())
            .collect();

        assert!(
            nested.len() >= 4,
            "expected at least backend, capabilities, coordinate_system and page_budget as \
             nested objects; found {nested:?}"
        );
        for required in [
            "backend",
            "capabilities",
            "coordinate_system",
            "page_budget",
        ] {
            assert!(
                nested.iter().any(|n| n == required),
                "`{required}` must be among the nested objects under test; found {nested:?}"
            );
        }

        for path in &nested {
            let mut v = serde_json::to_value(Profile::default()).unwrap();
            v[path]
                .as_object_mut()
                .unwrap()
                .insert("future_knob".into(), serde_json::Value::from(true));
            let parsed: Result<Profile, _> = serde_json::from_value(v);
            assert!(
                parsed.is_err(),
                "an unknown field inside `{path}` must be refused; accepting it means the knob \
                 is dropped and the profile re-hashes as though it never existed"
            );
        }
    }

    /// The failure that makes nested unknown fields a correctness bug, not a tidiness one.
    ///
    /// If a truncating parse were ever allowed, the reconstructed profile would produce a digest
    /// identical to the default — so an artifact would claim comparability with a profile it does
    /// not match. This asserts the parse is refused *and* records why it has to be.
    #[test]
    fn a_truncated_profile_can_never_reproduce_the_default_digest() {
        let baseline = Profile::default().profile_sha256().unwrap();

        for path in [
            "backend",
            "capabilities",
            "coordinate_system",
            "page_budget",
        ] {
            let mut v = serde_json::to_value(Profile::default()).unwrap();
            v[path]
                .as_object_mut()
                .unwrap()
                .insert("future_knob".into(), serde_json::Value::from(7));

            match serde_json::from_value::<Profile>(v) {
                Err(_) => {} // correct: refused before it could be re-hashed
                Ok(truncated) => panic!(
                    "a profile with an unknown field in `{path}` parsed, and re-hashed to {} \
                     (default is {baseline}). Comparability is now a lie.",
                    truncated.profile_sha256().unwrap()
                ),
            }
        }
    }

    #[test]
    fn the_profile_round_trips() {
        let p = Profile::default();
        let bytes = p.canonical_bytes().unwrap();
        let back: Profile = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, p);
        assert_eq!(back.profile_sha256().unwrap(), p.profile_sha256().unwrap());
    }
}
