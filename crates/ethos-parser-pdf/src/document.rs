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

//! The opened-document handle (`docs/04-ARCHITECTURE.md` §2.1).
//!
//! # Single load
//!
//! A document is opened **once** and shared by every stage. This is a correctness rule before it
//! is a performance one: two loads can disagree, and a classifier that saw a different object
//! graph from the extractor is a silent divergence with no diagnostic. Borrowed from
//! pdf-inspector, which gets this right (parity checklist P11).
//!
//! `classify` takes `&Document` today; M3's `extract` will take the same `&Document`. Nothing
//! below the CLI opens a file.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use ethos_parser_core::{EngineError, Profile, Sha256Hex};

/// A PDF that has been read, validated as a PDF, and parsed once.
///
/// Holds the source bytes' digest alongside the parsed object graph, so every artifact derived
/// from this handle can bind to the exact bytes that produced it without re-reading the file.
pub struct Document {
    inner: lopdf::Document,
    source_sha256: Sha256Hex,
    byte_len: usize,
    /// Page object ids keyed by their **1-based** page number, as `lopdf` reports them.
    pages: Vec<(u32, lopdf::ObjectId)>,
    /// How many cross-reference entries the bounded repair padded, if it ran.
    ///
    /// `None` means the document parsed as written — the overwhelmingly common case, and the
    /// only one v0 had. `Some(n)` makes the repair visible to every stage so the artifact can
    /// declare it; a repaired open that produced an artifact indistinguishable from an
    /// unrepaired one would be exactly the silent repair `docs/01-CONTRACT.md` §12 forbids.
    xref_entries_padded: Option<u32>,
    /// Parsed fonts, keyed by `(font dictionary object id, resource name)`.
    ///
    /// Fonts are shared document-wide through inherited `/Resources`, and parsing one means
    /// inflating and reading its `/ToUnicode` CMap and embedded font program — so re-parsing
    /// per page multiplied that work by the page count on long documents. The resource name is
    /// part of the key because it is baked into a `Font`'s error strings and its widths-absent
    /// limitation prose: one object referenced under two names must not share those bytes, or
    /// a limitation recorded on a later page would carry an earlier page's name. A `Mutex`
    /// rather than a `RefCell` so the handle stays `Sync`.
    font_cache: std::sync::Mutex<BTreeMap<(lopdf::ObjectId, String), Arc<crate::fonts::Font>>>,
}

impl core::fmt::Debug for Document {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // `lopdf::Document` is large and its Debug output is not deterministic-friendly. A
        // handle's identity is its source digest and shape.
        f.debug_struct("Document")
            .field("source_sha256", &self.source_sha256.as_str())
            .field("byte_len", &self.byte_len)
            .field("page_count", &self.pages.len())
            .field("xref_entries_padded", &self.xref_entries_padded)
            .finish()
    }
}

impl Document {
    /// Open a PDF from disk.
    ///
    /// # Errors
    ///
    /// See [`Document::open_bytes`]; plus [`EngineError::Io`] if the file cannot be read.
    pub fn open(path: &Path, profile: &Profile) -> Result<Self, EngineError> {
        let bytes = std::fs::read(path).map_err(|e| EngineError::Io {
            detail: format!("{}: {e}", path.display()),
        })?;
        Self::open_bytes(&bytes, profile)
    }

    /// Open a PDF from memory.
    ///
    /// # Errors
    ///
    /// - [`EngineError::Unsupported`] — the bytes are not a PDF (magic check; the extension is
    ///   never consulted)
    /// - [`EngineError::Encrypted`] — the document is password-protected. **Its own variant**,
    ///   never folded into "malformed" or "complex": a human can supply a password, and no amount
    ///   of retrying fixes it otherwise. This check is explicit because `lopdf` *succeeds* on an
    ///   encrypted document and reports zero pages, which would otherwise read as an empty
    ///   document rather than a locked one.
    /// - [`EngineError::Malformed`] — the document violates the PDF specification. Includes the
    ///   known-hostile `synthetic/table-regular-grid`, whose xref entries are 19 bytes where PDF
    ///   32000-1 §7.5.4 requires 20.
    /// - [`EngineError::MissingPart`] — a required structure is absent.
    pub fn open_bytes(bytes: &[u8], profile: &Profile) -> Result<Self, EngineError> {
        // Magic first, always. An encrypted or wrong-magic file is refused here and never
        // reaches the repair below — repairing either was never on the table
        // (`docs/01-CONTRACT.md` §12).
        crate::magic::check_pdf_magic(bytes)?;

        // The document as written, first. The repair is a fallback and nothing else: a
        // well-formed document never goes near it, so the common path is byte-for-byte the v0
        // path and cannot have changed behaviour.
        let (inner, xref_entries_padded) = match lopdf::Document::load_mem(bytes) {
            Ok(doc) => (doc, None),
            Err(original) => {
                let original = map_lopdf_error(original);
                match Self::repair_and_reload(bytes, profile, &original) {
                    Some((doc, padded)) => (doc, Some(padded)),
                    // The ORIGINAL error, not one about the repair. A caller asked why this
                    // document failed; "the repair did not apply" answers a question nobody
                    // asked and hides the one they did.
                    None => return Err(original),
                }
            }
        };

        // Explicit, and before anything reads the page tree. `lopdf` returns Ok for an encrypted
        // document and then reports zero pages — so without this check a locked file classifies
        // as a zero-page document and exits 1 instead of 2.
        if inner.is_encrypted() {
            return Err(EngineError::Encrypted {
                detail: "document is encrypted; a password is required to read it".into(),
            });
        }

        let pages: Vec<(u32, lopdf::ObjectId)> = inner.get_pages().into_iter().collect();

        Ok(Self {
            // **The ORIGINAL bytes**, deliberately, even after a repair. The artifact must bind
            // to the file the caller actually has: a digest over the repaired bytes would match
            // nothing on disk, and `grounding-check --source-artifact` would report `mismatched`
            // against the very document that produced the artifact.
            source_sha256: Sha256Hex::from_hex(&ethos_parser_core::sha256_hex_bytes(bytes))
                .expect("sha256 hex is always well formed"),
            byte_len: bytes.len(),
            pages,
            inner,
            xref_entries_padded,
            font_cache: std::sync::Mutex::new(BTreeMap::new()),
        })
    }

    /// A cached parse of the font at `oid` under resource name `id`, if any page loaded it.
    pub(crate) fn cached_font(
        &self,
        oid: lopdf::ObjectId,
        id: &str,
    ) -> Option<Arc<crate::fonts::Font>> {
        self.font_cache
            .lock()
            .expect("font cache lock is never poisoned: no panics while held")
            .get(&(oid, id.to_string()))
            .cloned()
    }

    /// Record a parsed font for reuse by later pages.
    pub(crate) fn cache_font(&self, oid: lopdf::ObjectId, id: &str, font: Arc<crate::fonts::Font>) {
        self.font_cache
            .lock()
            .expect("font cache lock is never poisoned: no panics while held")
            .insert((oid, id.to_string()), font);
    }

    /// The one bounded repair, attempted only after a normal parse failed.
    ///
    /// Returns `None` — leaving the caller to report the original error — when the profile
    /// disables the repair, when the failure is not a structural one, or when any of
    /// [`crate::xref::repair_xref`]'s preconditions does not hold.
    fn repair_and_reload(
        bytes: &[u8],
        profile: &Profile,
        original: &EngineError,
    ) -> Option<(lopdf::Document, u32)> {
        if !profile.xref_repair.is_enabled() {
            return None;
        }
        // Only a structural failure is a repair candidate. An encrypted document reports
        // `encrypted`, and a missing part reports `missing_part`; neither is a 19-byte xref
        // table, and trying anyway would be the start of general recovery.
        if original.code() != "malformed" {
            return None;
        }

        let repair = crate::xref::repair_xref(bytes).ok()?;
        // The repaired bytes must parse cleanly. If they do not, the document had something else
        // wrong with it and the original error is still the honest answer.
        let doc = lopdf::Document::load_mem(&repair.bytes).ok()?;
        Some((doc, repair.entries_padded))
    }

    /// How many cross-reference entries the bounded repair padded, or `None` if it did not run.
    pub fn xref_entries_padded(&self) -> Option<u32> {
        self.xref_entries_padded
    }

    /// Digest of the exact source bytes this handle was built from.
    pub fn source_sha256(&self) -> &Sha256Hex {
        &self.source_sha256
    }

    /// Size of the source in bytes.
    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Total pages in the document.
    ///
    /// Distinct from the number **sampled**: the whole point of bounded classification is that
    /// this number does not drive cost.
    pub fn page_count(&self) -> u32 {
        u32::try_from(self.pages.len()).unwrap_or(u32::MAX)
    }

    /// Page numbers and object ids in document order, **1-based**.
    ///
    /// `lopdf` already keys pages from 1; this preserves that rather than re-deriving it, so
    /// there is no place for an off-by-one to enter. pdf-inspector carries a 0-vs-1 inconsistency
    /// between two of its own result types; one indexing origin, established at the boundary, is
    /// how that is avoided.
    pub(crate) fn pages(&self) -> &[(u32, lopdf::ObjectId)] {
        &self.pages
    }

    /// The underlying parsed document, for stage implementations in this crate.
    pub(crate) fn inner(&self) -> &lopdf::Document {
        &self.inner
    }
}

/// Map `lopdf`'s failure modes onto the routable taxonomy.
///
/// The mapping is the point: a caller must be able to tell "encrypted" from "malformed" from
/// "unsupported" without parsing a message. LiteParse returns exit 1 for password-protected,
/// invalid-header, corrupt-header **and** a missing file, identically to "this document is
/// complex" (parity checklist L16) — failing closed with an indistinguishable signal.
fn map_lopdf_error(e: lopdf::Error) -> EngineError {
    use lopdf::Error as L;
    match e {
        L::Decryption(d) => EngineError::Encrypted {
            detail: d.to_string(),
        },
        L::InvalidPassword => EngineError::Encrypted {
            detail: "invalid password".into(),
        },
        L::IO(io) => EngineError::Io {
            detail: io.to_string(),
        },
        L::Unimplemented(what) => EngineError::Unsupported {
            what: "pdf feature".into(),
            detail: what.to_string(),
        },
        L::DictKey(key) => EngineError::MissingPart {
            part: format!("dictionary key {key}"),
        },
        L::MissingXrefEntry => EngineError::MissingPart {
            part: "xref entry".into(),
        },
        L::NoOutline => EngineError::MissingPart {
            part: "document outline".into(),
        },
        other => EngineError::Malformed {
            what: "pdf structure".into(),
            detail: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{bench_fixture, conformance_fixture};

    #[test]
    fn a_non_pdf_is_unsupported_not_malformed() {
        let e = Document::open_bytes(b"not a pdf\n", &Profile::default()).unwrap_err();
        assert_eq!(e.code(), "unsupported");
    }

    #[test]
    fn an_encrypted_document_is_encrypted_not_empty() {
        // The load succeeds and reports zero pages. Without the explicit check this would look
        // like a valid empty document.
        let bytes = conformance_fixture("failure/password-protected/document.pdf");
        let e = Document::open_bytes(&bytes, &Profile::default()).unwrap_err();
        assert_eq!(
            e.code(),
            "encrypted",
            "a locked document must never arrive as anything else; got {e}"
        );
    }

    /// **The v0.1 decision, on the fixture that forced it** (`docs/01-CONTRACT.md` §12).
    ///
    /// Through v0 this document exited 2: 19-byte xref entries where PDF 32000-1 §7.5.4 requires
    /// 20. v0.1 repairs that one class, so it now opens — and says so.
    #[test]
    fn the_known_hostile_xref_fixture_is_repaired_and_declares_it() {
        let bytes = conformance_fixture("synthetic/table-regular-grid/document.pdf");
        let doc = Document::open_bytes(&bytes, &Profile::default()).expect("v0.1 repairs this");

        assert_eq!(
            doc.xref_entries_padded(),
            Some(6),
            "the repair must be visible on the handle, or no artifact can declare it"
        );
        assert_eq!(doc.page_count(), 1);

        // The artifact binds to the file on disk, not to the repaired bytes. A digest over the
        // repaired copy would match nothing a caller holds.
        assert_eq!(
            doc.source_sha256().hex(),
            ethos_parser_core::sha256_hex_bytes(&bytes),
            "the source digest is over the ORIGINAL bytes"
        );
        assert_eq!(doc.byte_len(), bytes.len());
    }

    /// The repair is a knob, and turning it off restores v0's refusal exactly.
    #[test]
    fn a_profile_that_refuses_still_refuses() {
        let bytes = conformance_fixture("synthetic/table-regular-grid/document.pdf");
        let profile = Profile {
            xref_repair: ethos_parser_core::XrefRepair::Refuse,
            ..Profile::default()
        };
        let e = Document::open_bytes(&bytes, &profile).unwrap_err();
        assert_eq!(e.code(), "malformed", "got {e}");
    }

    /// A document with a *different* malformation is still refused, repair enabled or not.
    ///
    /// The whole risk of a repair is that it grows into general recovery. This is the test that
    /// says it did not: `corrupt-header-valid` fails for a reason that is not a 19-byte xref
    /// table, and the repair leaves it exactly as refused as before.
    #[test]
    fn a_different_malformation_is_not_swept_up_by_the_repair() {
        let bytes = conformance_fixture("failure/corrupt-header-valid/document.pdf");
        let e = Document::open_bytes(&bytes, &Profile::default()).unwrap_err();
        assert_eq!(e.code(), "malformed", "got {e}");
    }

    /// An encrypted document never reaches the repair.
    #[test]
    fn an_encrypted_document_is_never_repaired() {
        let bytes = conformance_fixture("failure/password-protected/document.pdf");
        let e = Document::open_bytes(&bytes, &Profile::default()).unwrap_err();
        assert_eq!(
            e.code(),
            "encrypted",
            "encryption is answered before any repair is considered; got {e}"
        );
    }

    #[test]
    fn a_corrupt_xref_is_malformed() {
        let bytes = conformance_fixture("failure/corrupt-header-valid/document.pdf");
        let e = Document::open_bytes(&bytes, &Profile::default()).unwrap_err();
        assert_eq!(e.code(), "malformed", "got {e}");
    }

    #[test]
    fn every_open_failure_mode_is_distinguishable() {
        // The LiteParse defect, asserted directly: these must not collapse to one signal.
        let cases = [
            (
                "not a pdf",
                Document::open_bytes(b"not a pdf\n", &Profile::default()),
            ),
            (
                "encrypted",
                Document::open_bytes(
                    &conformance_fixture("failure/password-protected/document.pdf"),
                    &Profile::default(),
                ),
            ),
            (
                "malformed",
                Document::open_bytes(
                    &conformance_fixture("failure/corrupt-header-valid/document.pdf"),
                    &Profile::default(),
                ),
            ),
        ];
        let codes: Vec<&str> = cases
            .iter()
            .map(|(label, r)| {
                r.as_ref()
                    .err()
                    .unwrap_or_else(|| panic!("{label} should fail"))
                    .code()
            })
            .collect();
        let mut unique = codes.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), codes.len(), "codes collapsed: {codes:?}");
    }

    #[test]
    fn pages_are_one_based_from_the_boundary() {
        let bytes = conformance_fixture("synthetic/two-lines/document.pdf");
        let doc = Document::open_bytes(&bytes, &Profile::default()).unwrap();
        assert_eq!(doc.page_count(), 1);
        assert_eq!(
            doc.pages()[0].0,
            1,
            "the first page is page 1, never page 0"
        );

        let bytes = bench_fixture("nist-sp-800-63b.pdf");
        let doc = Document::open_bytes(&bytes, &Profile::default()).unwrap();
        assert_eq!(doc.page_count(), 80);
        assert_eq!(doc.pages()[0].0, 1);
        assert_eq!(doc.pages().last().unwrap().0, 80);
    }

    #[test]
    fn the_source_digest_binds_to_the_exact_bytes() {
        let bytes = conformance_fixture("synthetic/simple-text/document.pdf");
        let doc = Document::open_bytes(&bytes, &Profile::default()).unwrap();
        assert_eq!(
            doc.source_sha256().hex(),
            ethos_parser_core::sha256_hex_bytes(&bytes)
        );
        assert_eq!(doc.byte_len(), bytes.len());
    }
}
