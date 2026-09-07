//! `WinAnsiEncoding`'s glyph-name column, code to Adobe glyph name.
//!
//! **Generated. Do not edit by hand.** Regenerate with
//! `python3 vendor/generate-winansi-glyph-names.py <glyphlist.txt>`, which refuses to emit an
//! entry that three independent sources do not agree on — this repository's own `WIN_ANSI`
//! code-to-text table, Adobe's Glyph List, and the glyph repertoire of the vendored Core-14 AFMs.
//!
//! **Why this exists as a derived table rather than a transcribed one.**
//! [`21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](../../../docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md)
//! refused the transcribed version: 224 entries typed by hand, carrying the transcription risk
//! that `docs/20` §4 used to reject pdf.js's metrics, for a seven-node return. §5 of that document
//! named the condition that reopens it, and this meets it — no entry here was typed, and
//! [`super::encoding`] re-checks the whole table against `WIN_ANSI` at test time with no external
//! source.
//!
//! **What it is for.** A standard-14 face's width lives under a glyph NAME in its AFM. A document
//! using `WinAnsiEncoding` addresses glyphs by CODE. This is the join, and it is the join for a
//! code above ASCII, where [`super::encoding`]'s `StandardEncoding` table stops.
//!
//! **Two codes are deliberately absent: 0xA0 and 0xAD.** Annex D notes that `WinAnsiEncoding`
//! also encodes `space` at 0xA0 and `hyphen` at 0xAD, but `WIN_ANSI` decodes those codes to
//! U+00A0 and U+00AD — no-break space and soft hyphen — which is right for TEXT and leaves the
//! generator with a codepoint no AFM glyph carries. The two readings are both real and they
//! disagree, so the generator emits neither rather than picking one: a width taken from the wrong
//! reading would be a plausible number for a glyph the document did not ask for. They cost
//! nothing measurable on the corpus of `21` — neither code appears in its seven-node residual.

/// Code to Adobe glyph name, for `WinAnsiEncoding`.
///
/// `None` where `WIN_ANSI` itself carries no character for the code, and at
/// 0xA0 and 0xAD — see the module note.
pub(crate) static WIN_ANSI_NAMES: &[Option<&'static str>; 256] = &build();

const fn build() -> [Option<&'static str>; 256] {
    let mut t: [Option<&'static str>; 256] = [None; 256];
    t[0x20] = Some("space");
    t[0x21] = Some("exclam");
    t[0x22] = Some("quotedbl");
    t[0x23] = Some("numbersign");
    t[0x24] = Some("dollar");
    t[0x25] = Some("percent");
    t[0x26] = Some("ampersand");
    t[0x27] = Some("quotesingle");
    t[0x28] = Some("parenleft");
    t[0x29] = Some("parenright");
    t[0x2A] = Some("asterisk");
    t[0x2B] = Some("plus");
    t[0x2C] = Some("comma");
    t[0x2D] = Some("hyphen");
    t[0x2E] = Some("period");
    t[0x2F] = Some("slash");
    t[0x30] = Some("zero");
    t[0x31] = Some("one");
    t[0x32] = Some("two");
    t[0x33] = Some("three");
    t[0x34] = Some("four");
    t[0x35] = Some("five");
    t[0x36] = Some("six");
    t[0x37] = Some("seven");
    t[0x38] = Some("eight");
    t[0x39] = Some("nine");
    t[0x3A] = Some("colon");
    t[0x3B] = Some("semicolon");
    t[0x3C] = Some("less");
    t[0x3D] = Some("equal");
    t[0x3E] = Some("greater");
    t[0x3F] = Some("question");
    t[0x40] = Some("at");
    t[0x41] = Some("A");
    t[0x42] = Some("B");
    t[0x43] = Some("C");
    t[0x44] = Some("D");
    t[0x45] = Some("E");
    t[0x46] = Some("F");
    t[0x47] = Some("G");
    t[0x48] = Some("H");
    t[0x49] = Some("I");
    t[0x4A] = Some("J");
    t[0x4B] = Some("K");
    t[0x4C] = Some("L");
    t[0x4D] = Some("M");
    t[0x4E] = Some("N");
    t[0x4F] = Some("O");
    t[0x50] = Some("P");
    t[0x51] = Some("Q");
    t[0x52] = Some("R");
    t[0x53] = Some("S");
    t[0x54] = Some("T");
    t[0x55] = Some("U");
    t[0x56] = Some("V");
    t[0x57] = Some("W");
    t[0x58] = Some("X");
    t[0x59] = Some("Y");
    t[0x5A] = Some("Z");
    t[0x5B] = Some("bracketleft");
    t[0x5C] = Some("backslash");
    t[0x5D] = Some("bracketright");
    t[0x5E] = Some("asciicircum");
    t[0x5F] = Some("underscore");
    t[0x60] = Some("grave");
    t[0x61] = Some("a");
    t[0x62] = Some("b");
    t[0x63] = Some("c");
    t[0x64] = Some("d");
    t[0x65] = Some("e");
    t[0x66] = Some("f");
    t[0x67] = Some("g");
    t[0x68] = Some("h");
    t[0x69] = Some("i");
    t[0x6A] = Some("j");
    t[0x6B] = Some("k");
    t[0x6C] = Some("l");
    t[0x6D] = Some("m");
    t[0x6E] = Some("n");
    t[0x6F] = Some("o");
    t[0x70] = Some("p");
    t[0x71] = Some("q");
    t[0x72] = Some("r");
    t[0x73] = Some("s");
    t[0x74] = Some("t");
    t[0x75] = Some("u");
    t[0x76] = Some("v");
    t[0x77] = Some("w");
    t[0x78] = Some("x");
    t[0x79] = Some("y");
    t[0x7A] = Some("z");
    t[0x7B] = Some("braceleft");
    t[0x7C] = Some("bar");
    t[0x7D] = Some("braceright");
    t[0x7E] = Some("asciitilde");
    t[0x80] = Some("Euro");
    t[0x82] = Some("quotesinglbase");
    t[0x83] = Some("florin");
    t[0x84] = Some("quotedblbase");
    t[0x85] = Some("ellipsis");
    t[0x86] = Some("dagger");
    t[0x87] = Some("daggerdbl");
    t[0x88] = Some("circumflex");
    t[0x89] = Some("perthousand");
    t[0x8A] = Some("Scaron");
    t[0x8B] = Some("guilsinglleft");
    t[0x8C] = Some("OE");
    t[0x8E] = Some("Zcaron");
    t[0x91] = Some("quoteleft");
    t[0x92] = Some("quoteright");
    t[0x93] = Some("quotedblleft");
    t[0x94] = Some("quotedblright");
    t[0x95] = Some("bullet");
    t[0x96] = Some("endash");
    t[0x97] = Some("emdash");
    t[0x98] = Some("tilde");
    t[0x99] = Some("trademark");
    t[0x9A] = Some("scaron");
    t[0x9B] = Some("guilsinglright");
    t[0x9C] = Some("oe");
    t[0x9E] = Some("zcaron");
    t[0x9F] = Some("Ydieresis");
    t[0xA1] = Some("exclamdown");
    t[0xA2] = Some("cent");
    t[0xA3] = Some("sterling");
    t[0xA4] = Some("currency");
    t[0xA5] = Some("yen");
    t[0xA6] = Some("brokenbar");
    t[0xA7] = Some("section");
    t[0xA8] = Some("dieresis");
    t[0xA9] = Some("copyright");
    t[0xAA] = Some("ordfeminine");
    t[0xAB] = Some("guillemotleft");
    t[0xAC] = Some("logicalnot");
    t[0xAE] = Some("registered");
    t[0xAF] = Some("macron");
    t[0xB0] = Some("degree");
    t[0xB1] = Some("plusminus");
    t[0xB2] = Some("twosuperior");
    t[0xB3] = Some("threesuperior");
    t[0xB4] = Some("acute");
    t[0xB5] = Some("mu");
    t[0xB6] = Some("paragraph");
    t[0xB7] = Some("periodcentered");
    t[0xB8] = Some("cedilla");
    t[0xB9] = Some("onesuperior");
    t[0xBA] = Some("ordmasculine");
    t[0xBB] = Some("guillemotright");
    t[0xBC] = Some("onequarter");
    t[0xBD] = Some("onehalf");
    t[0xBE] = Some("threequarters");
    t[0xBF] = Some("questiondown");
    t[0xC0] = Some("Agrave");
    t[0xC1] = Some("Aacute");
    t[0xC2] = Some("Acircumflex");
    t[0xC3] = Some("Atilde");
    t[0xC4] = Some("Adieresis");
    t[0xC5] = Some("Aring");
    t[0xC6] = Some("AE");
    t[0xC7] = Some("Ccedilla");
    t[0xC8] = Some("Egrave");
    t[0xC9] = Some("Eacute");
    t[0xCA] = Some("Ecircumflex");
    t[0xCB] = Some("Edieresis");
    t[0xCC] = Some("Igrave");
    t[0xCD] = Some("Iacute");
    t[0xCE] = Some("Icircumflex");
    t[0xCF] = Some("Idieresis");
    t[0xD0] = Some("Eth");
    t[0xD1] = Some("Ntilde");
    t[0xD2] = Some("Ograve");
    t[0xD3] = Some("Oacute");
    t[0xD4] = Some("Ocircumflex");
    t[0xD5] = Some("Otilde");
    t[0xD6] = Some("Odieresis");
    t[0xD7] = Some("multiply");
    t[0xD8] = Some("Oslash");
    t[0xD9] = Some("Ugrave");
    t[0xDA] = Some("Uacute");
    t[0xDB] = Some("Ucircumflex");
    t[0xDC] = Some("Udieresis");
    t[0xDD] = Some("Yacute");
    t[0xDE] = Some("Thorn");
    t[0xDF] = Some("germandbls");
    t[0xE0] = Some("agrave");
    t[0xE1] = Some("aacute");
    t[0xE2] = Some("acircumflex");
    t[0xE3] = Some("atilde");
    t[0xE4] = Some("adieresis");
    t[0xE5] = Some("aring");
    t[0xE6] = Some("ae");
    t[0xE7] = Some("ccedilla");
    t[0xE8] = Some("egrave");
    t[0xE9] = Some("eacute");
    t[0xEA] = Some("ecircumflex");
    t[0xEB] = Some("edieresis");
    t[0xEC] = Some("igrave");
    t[0xED] = Some("iacute");
    t[0xEE] = Some("icircumflex");
    t[0xEF] = Some("idieresis");
    t[0xF0] = Some("eth");
    t[0xF1] = Some("ntilde");
    t[0xF2] = Some("ograve");
    t[0xF3] = Some("oacute");
    t[0xF4] = Some("ocircumflex");
    t[0xF5] = Some("otilde");
    t[0xF6] = Some("odieresis");
    t[0xF7] = Some("divide");
    t[0xF8] = Some("oslash");
    t[0xF9] = Some("ugrave");
    t[0xFA] = Some("uacute");
    t[0xFB] = Some("ucircumflex");
    t[0xFC] = Some("udieresis");
    t[0xFD] = Some("yacute");
    t[0xFE] = Some("thorn");
    t[0xFF] = Some("ydieresis");
    t
}
