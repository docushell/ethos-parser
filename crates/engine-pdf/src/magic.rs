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
