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
    /// The `xref_repair` this document was opened under, which decided whether the repair could
    /// run. `extract` refuses a profile naming another (review 2026-09-26 N55).
    opened_under: ethos_parser_core::XrefRepair,
    /// Whether the backend decrypted this document with the empty user password at load.
    ///
    /// `lopdf` authenticates the empty password itself, decrypts every object and removes
    /// `/Encrypt` from the trailer before `open_bytes`'s encryption check runs, so a document
    /// whose user password is empty — one carrying an owner password alone, say — reads exactly
    /// as an unencrypted one would and the check has nothing left to refuse. Every artifact from
    /// such an open declares it (`limitations::ENCRYPTED_EMPTY_USER_PASSWORD`), because a
    /// consumer cannot otherwise tell that the bytes it binds to are ciphertext.
    opened_encrypted: bool,
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
            .field("opened_encrypted", &self.opened_encrypted)
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

        // An object stream that the cross-reference table itself lists as a compressed object is
        // malformed (PDF 32000-1 §7.5.7 stores no stream inside an object stream), and lopdf
        // 0.44.0's encrypted loader resolves that conflict in `HashMap` iteration order — so the
        // same bytes could load as different objects in two runs. Refused before anything reads
        // them. The unencrypted loader resolves it in cross-reference order and needs no guard.
        if inner.was_encrypted() {
            if let Some((object, container)) = nested_object_stream(&inner) {
                return Err(EngineError::Malformed {
                    what: "object stream".into(),
                    detail: format!(
                        "object {object} is stored in object stream {container}, which the \
                         cross-reference table does not list as an uncompressed object"
                    ),
                });
            }
        }

        // `lopdf` drops an object it cannot parse, and leaves a stream whose `/Length` does not
        // resolve without its data. Every later lookup reads the loss as an absence: an
        // annotation, a field or a `/ToUnicode` gone without a word, and a writer copying the
        // document writes the loss out. Refused here, once, before anything reads the document.
        if let Some((id, why)) = unloaded_in_use(&inner) {
            return Err(EngineError::Malformed {
                what: "pdf object".into(),
                detail: format!("object {} {} R {why}", id.0, id.1),
            });
        }

        let pages = walk_page_tree(&inner)?;

        Ok(Self {
            // **The ORIGINAL bytes**, deliberately, even after a repair. The artifact must bind
            // to the file the caller actually has: a digest over the repaired bytes would match
            // nothing on disk, and `grounding-check --source-artifact` would report `mismatched`
            // against the very document that produced the artifact.
            source_sha256: Sha256Hex::from_hex(&ethos_parser_core::sha256_hex_bytes(bytes))
                .expect("sha256 hex is always well formed"),
            byte_len: bytes.len(),
            pages,
            opened_encrypted: inner.was_encrypted(),
            inner,
            xref_entries_padded,
            opened_under: profile.xref_repair,
            font_cache: std::sync::Mutex::new(BTreeMap::new()),
        })
    }

    /// The font cache, recovering the guard rather than propagating poison.
    ///
    /// These two call sites read `.expect("font cache lock is never poisoned: no panics while
    /// held")` until v2-S15. The invariant was true and nothing enforced it: it holds only because
    /// no caller currently panics inside the critical section, and it stops holding the first time
    /// someone widens one. The failure that follows is out of all proportion to the cause — under
    /// `panic = "unwind"` (every test and debug build) one panic while the lock is held poisons it
    /// permanently, so every subsequent page panics HERE, turning one bad page into a whole
    /// document of cascading failures with the blame on the cache.
    ///
    /// Poison carries no information this cache needs. The map is a memo of data that is
    /// re-derivable by construction — the values are `Arc<Font>` and the worst case from
    /// proceeding is parsing a font twice, which is already possible under the race two rayon
    /// workers can lose on a cold entry. So recover the guard and carry on.
    ///
    /// `parking_lot::Mutex` would remove poisoning outright; it is a dependency, and this buys the
    /// same thing for zero new crates (`docs/04-ARCHITECTURE.md` §5).
    fn fonts(
        &self,
    ) -> std::sync::MutexGuard<'_, BTreeMap<(lopdf::ObjectId, String), Arc<crate::fonts::Font>>>
    {
        self.font_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A cached parse of the font at `oid` under resource name `id`, if any page loaded it.
    pub(crate) fn cached_font(
        &self,
        oid: lopdf::ObjectId,
        id: &str,
    ) -> Option<Arc<crate::fonts::Font>> {
        self.fonts().get(&(oid, id.to_string())).cloned()
    }

    /// Record a parsed font for reuse by later pages.
    pub(crate) fn cache_font(&self, oid: lopdf::ObjectId, id: &str, font: Arc<crate::fonts::Font>) {
        self.fonts().insert((oid, id.to_string()), font);
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

    /// The `xref_repair` this document was opened under.
    pub(crate) fn opened_under(&self) -> ethos_parser_core::XrefRepair {
        self.opened_under
    }

    /// How many cross-reference entries the bounded repair padded, or `None` if it did not run.
    pub fn xref_entries_padded(&self) -> Option<u32> {
        self.xref_entries_padded
    }

    /// Whether the backend decrypted this document with the empty user password at load.
    pub fn opened_encrypted(&self) -> bool {
        self.opened_encrypted
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

/// The first compressed object whose container is not an uncompressed object, if any.
fn nested_object_stream(doc: &lopdf::Document) -> Option<(u32, u32)> {
    use lopdf::xref::XrefEntry;
    let entries = &doc.reference_table.entries;
    entries.iter().find_map(|(&object, entry)| match entry {
        XrefEntry::Compressed { container, .. }
            if !matches!(entries.get(container), Some(XrefEntry::Normal { .. })) =>
        {
            Some((object, *container))
        }
        _ => None,
    })
}

/// The first object the cross-reference table lists in use whose data `lopdf` did not load, and
/// why. `/Encrypt` is the exception: `lopdf` removes it on purpose after decrypting.
///
/// A stream whose `/Length` resolves read its data at load, an empty one included, so only one
/// whose `/Length` does not resolve can have lost it that way.
fn unloaded_in_use(doc: &lopdf::Document) -> Option<(lopdf::ObjectId, &'static str)> {
    use lopdf::xref::XrefEntry;
    let encrypt = doc
        .encryption_state
        .as_ref()
        .and_then(lopdf::EncryptionState::encrypt_object_id);
    for (&number, entry) in &doc.reference_table.entries {
        let id = match *entry {
            XrefEntry::Normal { generation, .. } => (number, generation),
            XrefEntry::Compressed { .. } => (number, 0),
            _ => continue,
        };
        let why = match doc.objects.get(&id) {
            _ if Some(id) == encrypt => continue,
            None => "is in use in the cross-reference table and did not load",
            Some(lopdf::Object::Stream(s))
                if s.content.is_empty()
                    && s.start_position.is_some()
                    && s.dict
                        .get(b"Length")
                        .and_then(|l| doc.dereference(l))
                        .and_then(|(_, l)| l.as_i64())
                        .is_err() =>
            {
                "is a stream whose /Length does not resolve, so its data did not load"
            }
            _ => continue,
        };
        return Some((id, why));
    }
    None
}

/// A `/Pages` node's `/Kids`, or a refusal naming the node.
fn kids_of(doc: &lopdf::Document, id: lopdf::ObjectId) -> Result<&[lopdf::Object], EngineError> {
    doc.get_dictionary(id)
        .and_then(|d| d.get_deref(b"Kids", doc))
        .and_then(lopdf::Object::as_array)
        .map(Vec::as_slice)
        .map_err(|e| EngineError::Malformed {
            what: "page tree".into(),
            detail: format!("/Pages {} {} R has no /Kids array: {e}", id.0, id.1),
        })
}

/// The pages the document's own tree lists, in the tree's order — refusing a tree that is not one.
///
/// `lopdf`'s `get_pages` keeps no visited set: a `/Kids` entry naming an ancestor or naming one
/// page twice yields pages the document does not contain, bounded only by how many objects the
/// file holds; and it skips, without a word, a kid that is neither `/Page` nor `/Pages`, a kid it
/// could not load, and a `/Pages` node beneath 256 pending sibling lists. Each of those is a page
/// count the artifact would state as `complete` while the document says otherwise. For a tree
/// that is a tree, this is the same depth-first order and the same numbering.
fn walk_page_tree(doc: &lopdf::Document) -> Result<Vec<(u32, lopdf::ObjectId)>, EngineError> {
    const MAX_DEPTH: usize = 256;
    let malformed = |detail: String| EngineError::Malformed {
        what: "page tree".into(),
        detail,
    };
    let root = doc
        .catalog()
        .and_then(|c| c.get(b"Pages"))
        .and_then(lopdf::Object::as_reference)
        .map_err(|_| EngineError::MissingPart {
            part: "/Pages".into(),
        })?;
    let mut seen = std::collections::BTreeSet::from([root]);
    let mut pages = Vec::new();
    let mut stack = vec![kids_of(doc, root)?.iter()];
    while let Some(level) = stack.last_mut() {
        let Some(kid) = level.next() else {
            stack.pop();
            continue;
        };
        // A level with no kid left holds no pending sibling. It is dropped before descending, so
        // every level under the top of the stack still holds one.
        let exhausted = level.len() == 0;
        let id = kid
            .as_reference()
            .map_err(|_| malformed("a /Kids entry is not an indirect reference".into()))?;
        if !seen.insert(id) {
            return Err(malformed(format!(
                "object {} {} R is reached twice, so the page tree is not a tree",
                id.0, id.1
            )));
        }
        let dict = doc.get_dictionary(id).map_err(|e| {
            malformed(format!(
                "kid {} {} R did not load as a dictionary: {e}",
                id.0, id.1
            ))
        })?;
        let kind = dict
            .get(b"Type")
            .and_then(lopdf::Object::as_name)
            .map_err(|_| {
                malformed(format!(
                    "kid {} {} R names no /Type, so it is neither /Page nor /Pages",
                    id.0, id.1
                ))
            })?;
        match kind {
            b"Page" => {
                let number = u32::try_from(pages.len() + 1)
                    .map_err(|_| malformed("more pages than a u32 counts".into()))?;
                pages.push((number, id));
            }
            b"Pages" => {
                // `get_pages` skips a `/Pages` node beneath 256 pending sibling lists, and those
                // lists are the levels under this one: the bound counts them, not levels.
                if stack.len() > MAX_DEPTH {
                    return Err(malformed(format!("/Pages nests past {MAX_DEPTH} levels")));
                }
                if exhausted {
                    stack.pop();
                }
                stack.push(kids_of(doc, id)?.iter());
            }
            other => {
                return Err(malformed(format!(
                    "kid {} {} R has /Type /{}, neither /Page nor /Pages",
                    id.0,
                    id.1,
                    String::from_utf8_lossy(other)
                )));
            }
        }
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{bench_fixture, conformance_fixture};

    /// **A poisoned font cache is still a usable font cache.**
    ///
    /// These two accessors carried `.expect("font cache lock is never poisoned")` until v2-S15.
    /// Nothing enforced that invariant, and the consequence of it breaking was disproportionate:
    /// one panic while the lock is held poisons it for the life of the `Document`, so under
    /// `panic = "unwind"` every LATER page panics inside the cache rather than wherever the real
    /// fault was.
    ///
    /// The poison here is induced the only way it can be — a panic while a guard is live, caught
    /// so the test itself survives. What is asserted is that the reads and writes afterwards work
    /// normally, because the cache is a memo of re-derivable data and poison tells it nothing.
    #[test]
    fn a_poisoned_font_cache_still_reads_and_writes() {
        let bytes = conformance_fixture("synthetic/simple-text/document.pdf");
        let doc = Document::open_bytes(&bytes, &Profile::default()).expect("opens");

        // Poison it: panic with the guard held.
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = doc.font_cache.lock().expect("first lock is clean");
            panic!("induced, to poison the cache");
        }));
        assert!(poisoned.is_err(), "the induced panic must have happened");
        assert!(
            doc.font_cache.is_poisoned(),
            "the cache must actually be poisoned, or this test proves nothing"
        );

        // A read must be a miss rather than a panic. Before v2-S15 this line aborted.
        assert!(
            doc.cached_font((7, 0), "F1").is_none(),
            "a miss on a poisoned cache is a miss, not a panic"
        );

        // And the real path — extraction loads and caches every font this document uses, so it
        // goes through both accessors. Asserting the artifact rather than the accessors is what
        // makes this a test of the engine surviving poison rather than of two one-line helpers.
        let artifact = crate::extract::extract(&doc, &Profile::default())
            .expect("a poisoned font cache must not fail an extract");
        assert!(
            !artifact.pages.is_empty(),
            "the extract must still produce pages"
        );
    }

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
