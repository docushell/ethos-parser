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

//! Content-based format detection (`docs/03-V0-SCOPE.md` §1 item 12).
//!
//! **The extension is not authority.** A file named `.pdf` containing HTML is HTML, and a
//! classifier that trusts the name reports on a document it never read. Taken from Anydoc
//! (parity checklist A4).

use engine_core::EngineError;

/// The PDF header, per PDF 32000-1 §7.5.2.
const PDF_MAGIC: &[u8] = b"%PDF-";

/// How far into the file the header may start.
///
/// Zero. The specification puts `%PDF-` at byte 0, and readers that scan for it are the reason
/// documents with prepended junk circulate at all. Refusing is a declared limitation, not an
/// oversight: `docs/01-CONTRACT.md` §8 prefers a named error over a repair nobody recorded.
const MAX_HEADER_OFFSET: usize = 0;

/// Confirm the bytes are a PDF by content.
///
/// # Errors
///
/// [`EngineError::Unsupported`] when the magic is absent — the format is something this profile
/// does not handle, which is different from a PDF that is broken ([`EngineError::Malformed`]).
/// Both exit 2, but a caller routing on the variant can tell "not a PDF" from "a bad PDF".
pub fn check_pdf_magic(bytes: &[u8]) -> Result<(), EngineError> {
    debug_assert_eq!(
        MAX_HEADER_OFFSET, 0,
        "header scanning is deliberately absent"
    );

    if bytes.len() < PDF_MAGIC.len() {
        return Err(EngineError::Unsupported {
            what: "media type".into(),
            detail: format!(
                "file is {} bytes, shorter than the {}-byte PDF header",
                bytes.len(),
                PDF_MAGIC.len()
            ),
        });
    }

    if &bytes[..PDF_MAGIC.len()] != PDF_MAGIC {
        return Err(EngineError::Unsupported {
            what: "media type".into(),
            detail: format!(
                "expected a PDF header (%PDF-) at byte 0, found {:?}",
                String::from_utf8_lossy(&bytes[..PDF_MAGIC.len()])
            ),
        });
    }

    Ok(())
}

/// Whether these bytes were **aimed at this reader**, so that [`check_pdf_magic`]'s message is the
/// right one for them (v2-S10).
///
/// This is not a detector and it decides nothing about what the bytes *are*. It answers one
/// routing question: if the caller handed the PDF reader something, is a message about a PDF
/// header the honest cause of the refusal, or is it the wrong cause?
///
/// True in exactly two cases:
///
/// 1. The bytes **start with** the header. Whatever follows may be broken, and
///    [`check_pdf_magic`] and the reader behind it will say how.
/// 2. The bytes are a **proper prefix** of the header, the empty file included. A PDF truncated in
///    transit still aimed here, and *"file is 3 bytes, shorter than the 5-byte PDF header"* names
///    its real cause. A zero-byte file is the sharpest case: nothing about it says PDF, and
///    nothing about it says anything else either, so the reader the caller reached is the one that
///    should answer.
///
/// False for everything else — a `.csv`, a letter, a log line, a PNG — and that is the whole point.
/// Those bytes state **no** format, and telling them they are a broken PDF names a cause they never
/// had. `engine extract` refuses them without opening a reader; see `docs/15-V2-MILESTONES.md` S10.
///
/// [`MAX_HEADER_OFFSET`] is still zero here: nothing is scanned for at any other offset.
pub fn aims_at_the_pdf_reader(bytes: &[u8]) -> bool {
    debug_assert_eq!(
        MAX_HEADER_OFFSET, 0,
        "header scanning is deliberately absent"
    );

    if bytes.len() >= PDF_MAGIC.len() {
        bytes.starts_with(PDF_MAGIC)
    } else {
        PDF_MAGIC.starts_with(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_pdf_header_passes() {
        assert!(check_pdf_magic(b"%PDF-1.7\n...").is_ok());
        assert!(check_pdf_magic(b"%PDF-1.4").is_ok());
    }

    #[test]
    fn the_extension_is_not_authority() {
        // Every one of these could arrive named `document.pdf`.
        for (label, bytes) in [
            ("html", &b"<!DOCTYPE html><html>"[..]),
            ("png", &b"\x89PNG\r\n\x1a\n"[..]),
            ("zip/docx", &b"PK\x03\x04"[..]),
            ("the invalid-header fixture", &b"not a pdf\n"[..]),
            ("empty", &b""[..]),
            ("truncated magic", &b"%PD"[..]),
        ] {
            let e = check_pdf_magic(bytes).unwrap_err_or_else(|| panic!("{label} must be refused"));
            assert_eq!(
                e.code(),
                "unsupported",
                "{label} is a format gap, not a broken PDF"
            );
        }
    }

    /// **v2-S10.** The two questions are different, and the difference is the whole slice.
    ///
    /// `check_pdf_magic` answers *"is this a PDF"* and refuses everything that is not.
    /// `aims_at_the_pdf_reader` answers *"is a message about a PDF header the honest cause"* — and
    /// the truncated cases are exactly where the two part company: a three-byte `%PD` is not a PDF
    /// and is still a caller's PDF, arriving short.
    #[test]
    fn a_truncated_header_still_aims_at_this_reader_and_nothing_else_does() {
        for aimed in [&b""[..], b"%", b"%P", b"%PD", b"%PDF"] {
            assert!(
                aims_at_the_pdf_reader(aimed),
                "{:?} is a proper prefix of the header",
                String::from_utf8_lossy(aimed)
            );
            assert!(
                check_pdf_magic(aimed).is_err(),
                "and is still not a PDF, which is the other question"
            );
        }

        assert!(aims_at_the_pdf_reader(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3"));
        // Broken past the header is still aimed here — the reader behind this says how.
        assert!(aims_at_the_pdf_reader(b"%PDF-"));

        for elsewhere in [
            &b"name,role\nAda,engineer\n"[..],
            b"Dear Ada,\n\nPlease pick up milk, eggs, and bread.\n",
            b"Prose with no punctuation of that kind at all\n",
            b"PK\x03\x04",
            b"\x89PNG\r\n\x1a\n",
            b"{\\rtf1",
            // One byte short of the header, and not a prefix of it.
            b"%PDX",
            // The header, but not at byte 0. `MAX_HEADER_OFFSET` is zero.
            b"\n%PDF-1.7",
        ] {
            assert!(
                !aims_at_the_pdf_reader(elsewhere),
                "{:?} states no PDF header and was never aimed here",
                String::from_utf8_lossy(elsewhere)
            );
        }
    }

    #[test]
    fn the_header_must_be_at_byte_zero() {
        // Leading junk is refused rather than scanned past. A reader that hunts for the header
        // accepts documents whose byte 0 nobody has accounted for.
        assert!(check_pdf_magic(b"\n%PDF-1.7").is_err());
        assert!(check_pdf_magic(b"   %PDF-1.7").is_err());
    }

    /// `Result::unwrap_err` needs `T: Debug`; this keeps the message useful without that bound.
    trait UnwrapErrOrElse<E> {
        fn unwrap_err_or_else(self, f: impl FnOnce() -> E) -> E;
    }
    impl<T, E> UnwrapErrOrElse<E> for Result<T, E> {
        fn unwrap_err_or_else(self, f: impl FnOnce() -> E) -> E {
            match self {
                Err(e) => e,
                Ok(_) => f(),
            }
        }
    }
}
