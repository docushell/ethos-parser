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

//! What reading one document may cost, as ceilings that depend only on the document (review
//! 2026-09-25 F08).
//!
//! Deflate expands up to about 1,032:1, and a content operation costs about 620 bytes once decoded
//! — the reader's own token and `lopdf`'s operation, both alive while a page is read. So 4.7 KB of
//! `0 w` reached 651 MiB on one page, 17 KB reached 2.6 GB, and pages read in parallel multiplied
//! either by the host's cores: four pages sharing one stream took 788 MiB on one thread and 2,893
//! MiB on four. Every limit below is a named refusal where that was an out-of-memory kill.
//!
//! **Set from a census of 1,294 real PDFs** (3,370 pages: the gate set, gate-zero, the engine
//! fixtures, opendataloader-bench and OmniDocBench `v1_0`), each far above what the census holds:
//!
//! | ceiling | the census's largest | margin |
//! |---|---:|---:|
//! | [`MAX_DECODED_BYTES`], one stream or one page's content | 4.2 MB | 16x |
//! | [`MAX_PAGE_OPERATIONS`] | 166,960 | 6x |
//! | [`MAX_DOCUMENT_OPERATIONS`] | 5,737,203 | 6x |
//! | [`MAX_DOCUMENT_CONTENT_BYTES`] | 58 MB | 18x |
//!
//! Constants, not profile fields: a caller cannot move them, so `parser_version` is their identity,
//! as it is `zip::MAX_INFLATED_BYTES`'s in the office reader.

use std::sync::atomic::{AtomicU64, Ordering};

use ethos_parser_core::EngineError;

/// A ceiling on the bytes one stream decodes to, and on one page's content streams together.
pub(crate) const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;

/// A ceiling on the operations one page's content holds, counted as they are tokenised, before
/// `lopdf` builds its larger copy. The densest real page read here holds 166,960 and costs 162 MiB
/// to extract; a page at the ceiling costs about 650 MiB.
pub(crate) const MAX_PAGE_OPERATIONS: usize = 1 << 20;

/// Pages holding more operations than this are not read in parallel with the others: they are
/// left, and read afterwards one at a time, so a document of dense pages costs one dense page
/// whatever the host's core count. Three of the census's 3,370 pages are this dense.
pub(crate) const HEAVY_PAGE_OPERATIONS: usize = 1 << 16;

/// A ceiling on the operations one reading of a document's pages holds, every page together.
/// Pages may share one content stream, so without it a 5 KB file of a thousand pages each drawing
/// one dense stream is read a thousand times.
pub(crate) const MAX_DOCUMENT_OPERATIONS: u64 = 1 << 25;

/// A ceiling on the content bytes one reading of a document's pages decodes, every page together.
pub(crate) const MAX_DOCUMENT_CONTENT_BYTES: u64 = 1 << 30;

/// Why a stream's bytes were not decoded.
#[derive(Debug)]
pub(crate) enum NotDecoded {
    /// The stream decodes to more than the limit it was given.
    PastLimit,
    /// Its filters failed, as `lopdf`'s own decoder reports.
    Failed,
}

/// `stream`'s bytes as `lopdf`'s `decompressed_content` gives them, refused past `limit`, which
/// callers take from [`MAX_DECODED_BYTES`].
pub(crate) fn decoded(stream: &lopdf::Stream, limit: usize) -> Result<Vec<u8>, NotDecoded> {
    stream
        .decompressed_content_with_limit(limit)
        .map_err(|e| match e {
            lopdf::Error::Decompress(lopdf::DecompressError::MemoryLimitExceeded { .. }) => {
                NotDecoded::PastLimit
            }
            _ => NotDecoded::Failed,
        })
}

/// The refusal for a stream that decodes to more than [`MAX_DECODED_BYTES`].
pub(crate) fn stream_past_ceiling(what: String) -> EngineError {
    EngineError::ResourceLimit {
        limit: format!("decoded bytes of {what}"),
        configured: MAX_DECODED_BYTES.to_string(),
    }
}

/// The refusal for a page whose content holds more than [`MAX_PAGE_OPERATIONS`] operations.
pub(crate) fn page_operations_past_ceiling(page_number: u32) -> EngineError {
    EngineError::ResourceLimit {
        limit: format!("content operations on page {page_number}"),
        configured: MAX_PAGE_OPERATIONS.to_string(),
    }
}

/// What one reading of a document's pages has decoded and tokenised, every page together.
///
/// **One per reading, not one per document**: a library caller extracting twice from one
/// `Document` reads it twice, and is not charged for the first reading on the second.
///
/// **Charged when a page is read whole**, never in part, and a page that starts after the budget
/// is spent stops at once. Pages run in parallel, so which page crossed is timing; that the
/// document crosses is not — a page stops early only once the budget is already spent — so the
/// refusal, [`ContentBudget::refusal`], names the document and never a page.
#[derive(Debug, Default)]
pub(crate) struct ContentBudget {
    operations: AtomicU64,
    bytes: AtomicU64,
}

impl ContentBudget {
    /// Charge one page read whole: its decoded content bytes and its operations.
    pub(crate) fn charge(&self, bytes: usize, operations: usize) -> Result<(), EngineError> {
        let wide = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
        self.bytes.fetch_add(wide(bytes), Ordering::Relaxed);
        self.operations
            .fetch_add(wide(operations), Ordering::Relaxed);
        self.refusal()
    }

    /// The document's refusal, if its pages passed a ceiling between them: the same whichever
    /// page crossed.
    pub(crate) fn refusal(&self) -> Result<(), EngineError> {
        if self.operations.load(Ordering::Relaxed) > MAX_DOCUMENT_OPERATIONS {
            return Err(EngineError::ResourceLimit {
                limit: "content operations in the document's pages".into(),
                configured: MAX_DOCUMENT_OPERATIONS.to_string(),
            });
        }
        if self.bytes.load(Ordering::Relaxed) > MAX_DOCUMENT_CONTENT_BYTES {
            return Err(EngineError::ResourceLimit {
                limit: "decoded content bytes in the document's pages".into(),
                configured: MAX_DOCUMENT_CONTENT_BYTES.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit_of(e: EngineError) -> (String, String) {
        match e {
            EngineError::ResourceLimit { limit, configured } => (limit, configured),
            other => panic!("expected a resource limit, got {other}"),
        }
    }

    /// **Each document ceiling admits a reading exactly at it and refuses the first unit past.**
    ///
    /// The failure this catches: a ceiling never checked, one off by one, the two counts crossed,
    /// or a refusal naming the other limit.
    #[test]
    fn the_content_budget_admits_to_each_ceiling_and_refuses_past_it() {
        let wide = |n: u64| usize::try_from(n).expect("64-bit");

        let operations = ContentBudget::default();
        operations
            .charge(0, wide(MAX_DOCUMENT_OPERATIONS) - 1)
            .expect("under");
        operations.charge(0, 1).expect("exactly at the ceiling");
        assert_eq!(
            limit_of(operations.charge(0, 1).expect_err("one past it")),
            (
                "content operations in the document's pages".to_string(),
                MAX_DOCUMENT_OPERATIONS.to_string()
            )
        );
        assert!(operations.refusal().is_err(), "and it stays spent");

        let bytes = ContentBudget::default();
        bytes
            .charge(wide(MAX_DOCUMENT_CONTENT_BYTES), 0)
            .expect("exactly at the ceiling");
        assert_eq!(
            limit_of(bytes.charge(1, 0).expect_err("one past it")),
            (
                "decoded content bytes in the document's pages".to_string(),
                MAX_DOCUMENT_CONTENT_BYTES.to_string()
            )
        );
    }

    /// **A stream decodes to at most its limit, and past it is told from a filter that fails.**
    ///
    /// The failure this catches: an unbounded decode, a limit that truncates instead of refusing,
    /// or a size refusal reported as a broken filter, which would name the wrong cause.
    #[test]
    fn a_stream_decodes_within_its_limit_and_is_refused_past_it() {
        use std::io::Write;
        let plain = vec![b' '; 10_000];
        let mut deflated = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        deflated.write_all(&plain).expect("deflates");
        let mut dict = lopdf::Dictionary::new();
        dict.set("Filter", lopdf::Object::Name(b"FlateDecode".to_vec()));
        let flate = lopdf::Stream::new(dict, deflated.finish().expect("deflates"));

        assert_eq!(decoded(&flate, 10_000).expect("within"), plain);
        assert!(matches!(decoded(&flate, 9_999), Err(NotDecoded::PastLimit)));

        let raw = lopdf::Stream::new(lopdf::Dictionary::new(), plain.clone());
        assert_eq!(decoded(&raw, 10_000).expect("within"), plain);
        assert!(matches!(decoded(&raw, 9_999), Err(NotDecoded::PastLimit)));

        let mut dict = lopdf::Dictionary::new();
        dict.set("Filter", lopdf::Object::Name(b"NoSuchFilter".to_vec()));
        let broken = lopdf::Stream::new(dict, plain);
        assert!(matches!(
            decoded(&broken, MAX_DECODED_BYTES),
            Err(NotDecoded::Failed)
        ));
    }
}
