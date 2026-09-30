//! PDFDocEncoding, code to Unicode scalar: the encoding of a PDF text string that is not
//! UTF-16BE (PDF 32000-1 §7.9.2.2, Annex D.2).
//!
//! **Generated. Do not edit by hand.** Regenerate with
//! `python3 vendor/generate-pdfdoc-encoding.py <pdfjs-dist dir> <ghostscript lib dir>`, which
//! emits a code only where three independent decoders read it as the same scalar — qpdf
//! 12.3.2's text-string decoder, pdf.js 5.7.284's outline titles and Ghostscript 10.06.0's
//! `doc-to-ucode` table.
//!
//! **Why derived rather than transcribed.**
//! [`21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](../../../docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md)
//! §4 refused a hand-typed Annex D table, and §5 named the condition that reopens one: a derived
//! table, every entry cross-validated. `WIN_ANSI` cannot be the cross-check here — 0x80–0x9F is
//! exactly where it and PDFDocEncoding disagree — so three PDF implementations are.
//! [`29-OUTLINES-SCOPE.md`](../../../docs/29-OUTLINES-SCOPE.md) §9 `S-ENC` is the slice.
//!
//! **4 codes are `None`, because the three disagree:**
//! - `0x1B`: qpdf U+02D9, pdf.js nothing, Ghostscript U+02D9.
//! - `0x7F`: qpdf U+FFFD, pdf.js U+007F, Ghostscript U+007F.
//! - `0x9F`: qpdf U+FFFD, pdf.js U+009F, Ghostscript 0, its mark for no character.
//! - `0xAD`: qpdf U+FFFD, pdf.js U+00AD, Ghostscript U+00AD.
//!
//! A `None` is refused by the strict decoders and becomes U+FFFD in the lenient one
//! (`crate::forms`).

/// Code to scalar, and `None` where the three decoders disagree — see the module note.
pub(crate) static PDFDOC: [Option<char>; 256] = build();

const fn build() -> [Option<char>; 256] {
    let mut t: [Option<char>; 256] = [None; 256];
    t = latin1(t, 0x00, 0x17);
    t[0x18] = Some('\u{02D8}');
    t[0x19] = Some('\u{02C7}');
    t[0x1A] = Some('\u{02C6}');
    t[0x1C] = Some('\u{02DD}');
    t[0x1D] = Some('\u{02DB}');
    t[0x1E] = Some('\u{02DA}');
    t[0x1F] = Some('\u{02DC}');
    t = latin1(t, 0x20, 0x7E);
    t[0x80] = Some('\u{2022}');
    t[0x81] = Some('\u{2020}');
    t[0x82] = Some('\u{2021}');
    t[0x83] = Some('\u{2026}');
    t[0x84] = Some('\u{2014}');
    t[0x85] = Some('\u{2013}');
    t[0x86] = Some('\u{0192}');
    t[0x87] = Some('\u{2044}');
    t[0x88] = Some('\u{2039}');
    t[0x89] = Some('\u{203A}');
    t[0x8A] = Some('\u{2212}');
    t[0x8B] = Some('\u{2030}');
    t[0x8C] = Some('\u{201E}');
    t[0x8D] = Some('\u{201C}');
    t[0x8E] = Some('\u{201D}');
    t[0x8F] = Some('\u{2018}');
    t[0x90] = Some('\u{2019}');
    t[0x91] = Some('\u{201A}');
    t[0x92] = Some('\u{2122}');
    t[0x93] = Some('\u{FB01}');
    t[0x94] = Some('\u{FB02}');
    t[0x95] = Some('\u{0141}');
    t[0x96] = Some('\u{0152}');
    t[0x97] = Some('\u{0160}');
    t[0x98] = Some('\u{0178}');
    t[0x99] = Some('\u{017D}');
    t[0x9A] = Some('\u{0131}');
    t[0x9B] = Some('\u{0142}');
    t[0x9C] = Some('\u{0153}');
    t[0x9D] = Some('\u{0161}');
    t[0x9E] = Some('\u{017E}');
    t[0xA0] = Some('\u{20AC}');
    t = latin1(t, 0xA1, 0xAC);
    t = latin1(t, 0xAE, 0xFF);
    t
}

/// `first..=last`, each read as the Latin-1 scalar of the same value.
const fn latin1(mut t: [Option<char>; 256], first: u8, last: u8) -> [Option<char>; 256] {
    let mut code = first;
    loop {
        t[code as usize] = Some(code as char);
        if code == last {
            return t;
        }
        code += 1;
    }
}
