//! Standard-14 font metrics, read from the pristine Adobe AFM files in `vendor/afm/`.
//!
//! **Why this is reading and not guessing.** PDF 32000-1 §9.6.2.2 lets a font dictionary name one
//! of the standard 14 fonts with no `/Widths` and no `/FontDescriptor` *because* a conforming
//! reader is expected to hold these metrics. The values are therefore **known and merely absent
//! from the file**, which is the same standing [`crate::fonts`] already gives a composite font's
//! `/DW`: *"Reading a normative default is reading the document, not guessing at it"*. Decision
//! #22 of `docs/00-NORTH-STAR.md` is where that argument was accepted.
//!
//! **The line this module does not cross.** These metrics are supplied only for a font the
//! document itself names — Helvetica's metrics for `/BaseFont /Helvetica`. Supplying them for
//! `Arial` is a metric *substitution* and stays refused, so [`for_base_font`] matches the Core-14
//! names exactly and nothing else. On the corpus decision #22 measured, 1 867 nodes sit on
//! `Arial` and `TimesNewRoman` and correctly recover nothing.
//!
//! **What is deliberately partial.** A width is found by asking the font's own decoder what a code
//! means and looking that up, so coverage stops where this profile's encoding tables stop — the
//! ASCII range of `StandardEncoding` plus the glyph names [`crate::encoding`] carries. A code
//! outside that reports **no advance**, exactly as it did before this module existed. Widening it
//! needs the Annex D glyph-name column, which is its own measurement rather than a guess bolted on
//! here.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::encoding::{glyph_name_to_str, standard_code_to_str};

/// Glyph-space units per em, the AFM's own unit and the PDF default.
const GLYPH_SPACE_UNITS: f64 = 1000.0;

/// Ascent and descent came from the AFM's `Ascender` and `Descender` keys.
pub const SOURCE_ASCENDER: &str = "standard-14-afm-ascender-descender";

/// Ascent and descent came from `FontBBox`, because this face declares no `Ascender`.
///
/// Only `Symbol` and `ZapfDingbats` reach this. Decision #22 measured that fallback at **4 nodes**
/// on its corpus, with `ZapfDingbats` at zero — a footnote, and published as one.
pub const SOURCE_FONT_BBOX: &str = "standard-14-afm-font-bbox";

/// One Core-14 face's metrics.
///
/// `Debug` is written by hand: the derived form would dump 256 width slots and a text map on
/// every trace of a `WidthSource`, which is noise rather than evidence. `PartialEq` compares the
/// vendored data itself, so two references to the same face compare equal.
pub struct Core14 {
    /// The Core-14 name this face was matched under, e.g. `Helvetica`.
    pub face: &'static str,
    /// Highest ink above the baseline, in glyph space.
    pub ascent: f64,
    /// Lowest ink below the baseline, in glyph space. Negative.
    pub descent: f64,
    /// Which AFM key the vertical envelope came from.
    pub vertical_source: &'static str,
    /// Widths keyed by the AFM's own `C` code.
    ///
    /// For the twelve text faces those codes are `StandardEncoding`; for `Symbol` and
    /// `ZapfDingbats` they are the face's built-in encoding. Only consulted when the document
    /// names no base encoding, so the two cases never mix.
    by_code: [Option<f64>; 256],
    /// Widths keyed by the text a code decodes to.
    ///
    /// Empty for `Symbol` and `ZapfDingbats`: their `C` codes are not `StandardEncoding`, so
    /// resolving them through it would attach one glyph's width to another's character.
    by_text: BTreeMap<&'static str, f64>,
}

impl std::fmt::Debug for Core14 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Core14")
            .field("face", &self.face)
            .field("ascent", &self.ascent)
            .field("descent", &self.descent)
            .field("vertical_source", &self.vertical_source)
            .field("text_keys", &self.by_text.len())
            .finish()
    }
}

impl PartialEq for Core14 {
    fn eq(&self, other: &Self) -> bool {
        self.face == other.face
            && self.ascent == other.ascent
            && self.descent == other.descent
            && self.vertical_source == other.vertical_source
            && self.by_text == other.by_text
            && self.by_code == other.by_code
    }
}

impl Core14 {
    /// Advance for the text a code decoded to, in text space units per unit font size.
    pub fn advance_for_text(&self, text: &str) -> Option<f64> {
        self.by_text.get(text).map(|w| w / GLYPH_SPACE_UNITS)
    }

    /// Advance for a raw code, in text space units per unit font size.
    ///
    /// Valid only where the document names no base encoding, so the code is the face's built-in
    /// one — which is what the AFM's `C` column holds.
    pub fn advance_for_builtin_code(&self, code: u32) -> Option<f64> {
        let idx = usize::try_from(code).ok()?;
        self.by_code.get(idx)?.map(|w| w / GLYPH_SPACE_UNITS)
    }
}

/// The 14 files, embedded verbatim. The bytes in the binary are the bytes in `vendor/afm/`.
const FILES: [(&str, &str); 14] = [
    ("Courier", include_str!("../../../vendor/afm/Courier.afm")),
    (
        "Courier-Bold",
        include_str!("../../../vendor/afm/Courier-Bold.afm"),
    ),
    (
        "Courier-BoldOblique",
        include_str!("../../../vendor/afm/Courier-BoldOblique.afm"),
    ),
    (
        "Courier-Oblique",
        include_str!("../../../vendor/afm/Courier-Oblique.afm"),
    ),
    (
        "Helvetica",
        include_str!("../../../vendor/afm/Helvetica.afm"),
    ),
    (
        "Helvetica-Bold",
        include_str!("../../../vendor/afm/Helvetica-Bold.afm"),
    ),
    (
        "Helvetica-BoldOblique",
        include_str!("../../../vendor/afm/Helvetica-BoldOblique.afm"),
    ),
    (
        "Helvetica-Oblique",
        include_str!("../../../vendor/afm/Helvetica-Oblique.afm"),
    ),
    (
        "Times-Roman",
        include_str!("../../../vendor/afm/Times-Roman.afm"),
    ),
    (
        "Times-Bold",
        include_str!("../../../vendor/afm/Times-Bold.afm"),
    ),
    (
        "Times-BoldItalic",
        include_str!("../../../vendor/afm/Times-BoldItalic.afm"),
    ),
    (
        "Times-Italic",
        include_str!("../../../vendor/afm/Times-Italic.afm"),
    ),
    ("Symbol", include_str!("../../../vendor/afm/Symbol.afm")),
    (
        "ZapfDingbats",
        include_str!("../../../vendor/afm/ZapfDingbats.afm"),
    ),
];

/// The two faces whose `C` codes are their own built-in encoding rather than `StandardEncoding`.
const SYMBOLIC: [&str; 2] = ["Symbol", "ZapfDingbats"];

static TABLE: LazyLock<BTreeMap<&'static str, Core14>> = LazyLock::new(|| {
    FILES
        .iter()
        .filter_map(|(name, text)| parse(name, text).map(|m| (*name, m)))
        .collect()
});

/// Metrics for a base font name, or `None` when it is not one of the standard 14.
///
/// The name is matched **exactly**, after any subset prefix is stripped. `Arial` is not
/// `Helvetica` and gets nothing.
pub fn for_base_font(base_font: &str) -> Option<&'static Core14> {
    let name = base_font
        .split_once('+')
        .map_or(base_font, |(_, rest)| rest);
    TABLE.get(name)
}

/// Parse one AFM.
///
/// Returns `None` if the file carries no usable vertical envelope, which would make every ink box
/// built from it degenerate. No Core-14 file is in that state; the check is here so a damaged
/// vendored file fails closed instead of emitting zero-height boxes.
fn parse(name: &'static str, text: &str) -> Option<Core14> {
    let symbolic = SYMBOLIC.contains(&name);
    let mut ascender = None;
    let mut descender = None;
    let mut bbox_top = None;
    let mut bbox_bottom = None;
    let mut by_code = [None; 256];
    let mut by_text: BTreeMap<&'static str, f64> = BTreeMap::new();
    // Keys whose width is contested by two glyphs. Dropped rather than guessed between.
    let mut ambiguous: Vec<&'static str> = Vec::new();

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(v) = line.strip_prefix("Ascender ") {
            ascender = v.trim().parse::<f64>().ok();
        } else if let Some(v) = line.strip_prefix("Descender ") {
            descender = v.trim().parse::<f64>().ok();
        } else if let Some(v) = line.strip_prefix("FontBBox ") {
            let nums: Vec<f64> = v
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            if let [_, lly, _, ury] = nums[..] {
                bbox_bottom = Some(lly);
                bbox_top = Some(ury);
            }
        } else if line.starts_with("C ") {
            let (code, width, glyph) = char_metric(line)?;
            if let (Ok(idx), true) = (usize::try_from(code), (0..256).contains(&code)) {
                by_code[idx] = Some(width);
            }
            if symbolic {
                continue;
            }
            // Prefer the glyph name this profile knows; fall back to what StandardEncoding says
            // the AFM's own code means. Both land on the text a decoder would produce.
            let key = glyph_name_to_str(glyph).or_else(|| {
                u8::try_from(code)
                    .ok()
                    .and_then(|c| standard_code_to_str(c))
            });
            if let Some(k) = key {
                match by_text.get(k) {
                    Some(existing) if (*existing - width).abs() > f64::EPSILON => {
                        ambiguous.push(k);
                    }
                    _ => {
                        by_text.insert(k, width);
                    }
                }
            }
        }
    }

    for k in ambiguous {
        by_text.remove(k);
    }

    let (ascent, descent, vertical_source) = match (ascender, descender) {
        (Some(a), Some(d)) => (a, d, SOURCE_ASCENDER),
        // Symbol and ZapfDingbats declare neither.
        _ => (bbox_top?, bbox_bottom?, SOURCE_FONT_BBOX),
    };
    if ascent <= descent {
        return None;
    }

    Some(Core14 {
        face: name,
        ascent,
        descent,
        vertical_source,
        by_code,
        by_text,
    })
}

/// Pull `C`, `WX` and `N` out of one `StartCharMetrics` line.
///
/// The AFM grammar is `key value ;` segments in any order, so this reads them by name rather than
/// by position.
fn char_metric(line: &str) -> Option<(i64, f64, &str)> {
    let mut code = None;
    let mut width = None;
    let mut glyph = None;
    for field in line.split(';') {
        let mut parts = field.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("C"), Some(v)) => code = v.parse::<i64>().ok(),
            (Some("WX"), Some(v)) => width = v.parse::<f64>().ok(),
            (Some("N"), Some(v)) => glyph = Some(v),
            _ => {}
        }
    }
    Some((code?, width?, glyph?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_core14_file_parses() {
        assert_eq!(TABLE.len(), 14, "all 14 vendored AFMs must parse");
    }

    #[test]
    fn helvetica_matches_the_published_metrics() {
        let m = for_base_font("Helvetica").expect("Helvetica is a standard-14 face");
        // Adobe's own values, read from vendor/afm/Helvetica.afm.
        assert_eq!(m.ascent, 718.0);
        assert_eq!(m.descent, -207.0);
        assert_eq!(m.vertical_source, SOURCE_ASCENDER);
        // `space` is 278/1000 em in Helvetica.
        assert_eq!(m.advance_for_text(" "), Some(0.278));
    }

    #[test]
    fn subset_prefix_is_stripped() {
        assert!(for_base_font("ABCDEF+Helvetica").is_some());
    }

    #[test]
    fn a_font_the_document_did_not_name_gets_nothing() {
        // The substitution decision #22 refuses: Arial is not Helvetica.
        assert!(for_base_font("Arial").is_none());
        assert!(for_base_font("TimesNewRomanPSMT").is_none());
        assert!(for_base_font("Helvetica-Condensed").is_none());
    }

    #[test]
    fn symbolic_faces_fall_back_to_the_bounding_box() {
        for name in SYMBOLIC {
            let m = for_base_font(name).expect("vendored");
            assert_eq!(
                m.vertical_source, SOURCE_FONT_BBOX,
                "{name} declares no Ascender"
            );
            // Their C codes are their own encoding, so no text keys are built.
            assert!(m.advance_for_text("a").is_none());
            assert!(m.by_text.is_empty(), "{name} must build no text keys");
        }
    }

    #[test]
    fn symbolic_widths_are_reachable_by_builtin_code() {
        let m = for_base_font("Symbol").expect("vendored");
        // C 97 in Symbol.afm is `alpha`, WX 631.
        assert_eq!(m.advance_for_builtin_code(97), Some(0.631));
    }
}
