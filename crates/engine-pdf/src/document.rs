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

use std::path::Path;

use engine_core::{EngineError, Profile, Sha256Hex};

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
}

impl core::fmt::Debug for Document {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // `lopdf::Document` is large and its Debug output is not deterministic-friendly. A
        // handle's identity is its source digest and shape.
        f.debug_struct("Document")
            .field("source_sha256", &self.source_sha256.as_str())
            .field("byte_len", &self.byte_len)
            .field("page_count", &self.pages.len())
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
    pub fn open_bytes(bytes: &[u8], _profile: &Profile) -> Result<Self, EngineError> {
        crate::magic::check_pdf_magic(bytes)?;

        let inner = lopdf::Document::load_mem(bytes).map_err(map_lopdf_error)?;

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
            source_sha256: Sha256Hex::from_hex(&engine_core::sha256_hex_bytes(bytes))
                .expect("sha256 hex is always well formed"),
            byte_len: bytes.len(),
            pages,
            inner,
        })
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

    #[test]
    fn the_known_hostile_xref_fixture_is_malformed() {
        // 19-byte xref entries where PDF 32000-1 §7.5.4 requires 20. Refusing is correct; the
        // rate is a declared limitation (docs/03-V0-SCOPE.md §4).
        let bytes = conformance_fixture("synthetic/table-regular-grid/document.pdf");
        let e = Document::open_bytes(&bytes, &Profile::default()).unwrap_err();
        assert_eq!(e.code(), "malformed", "got {e}");
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
            engine_core::sha256_hex_bytes(&bytes)
        );
        assert_eq!(doc.byte_len(), bytes.len());
    }
}
