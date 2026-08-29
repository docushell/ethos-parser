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

//! The content-stream operator set, enumerated (`docs/05-MILESTONES.md` M3).
//!
//! # Why an enum and not a string match
//!
//! Every operator PDF 32000-1 Table A.1 defines has a variant here. Dispatch is an **exhaustive
//! match over this enum**, so adding a variant without handling it fails to compile, and a token
//! that maps to no variant is a **hard error** rather than a skip.
//!
//! That is the whole point. pdf-inspector's content-stream match omits the `"` show-text
//! operator: its text vanishes silently, surrounding runs merge with corrupt geometry, and the
//! output still looks well-formed — undetectable downstream (parity checklist P6). `Tz` appears
//! nowhere in its tree either, so advance widths are wrong on any document that uses it. Both are
//! the same defect: a permissive default.
//!
//! # Three kinds of arm, no fourth
//!
//! - **Interpreted** — the operator changes text state or emits text.
//! - **Acknowledged no-op** — a standard operator this profile does not need for text. It is
//!   *named*, its operands are consumed, and it is skipped **deliberately**. Graphics state,
//!   colour, path painting: none of it moves a glyph origin.
//! - **Unknown** — not in the table. Refused, naming the token.
//!
//! There is no "ignore and continue".

use serde::{Deserialize, Serialize};

/// Every operator this interpreter recognises.
///
/// Grouped by PDF 32000-1 Table A.1's own categories so the list can be checked against the
/// specification rather than against what happened to appear in a corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Operator {
    // --- Graphics state ---
    /// `q` — save graphics state.
    SaveState,
    /// `Q` — restore graphics state.
    RestoreState,
    /// `cm` — concatenate matrix to the CTM.
    ConcatMatrix,
    /// `w` — line width.
    LineWidth,
    /// `J` — line cap.
    LineCap,
    /// `j` — line join.
    LineJoin,
    /// `M` — miter limit.
    MiterLimit,
    /// `d` — dash pattern.
    DashPattern,
    /// `ri` — rendering intent.
    RenderingIntent,
    /// `i` — flatness tolerance.
    Flatness,
    /// `gs` — apply a named graphics-state dictionary.
    ExtGState,

    // --- Path construction ---
    /// `m` — begin subpath.
    MoveTo,
    /// `l` — line to.
    LineTo,
    /// `c` — cubic Bézier, three control points.
    CurveTo,
    /// `v` — cubic Bézier, initial point replicated.
    CurveToV,
    /// `y` — cubic Bézier, final point replicated.
    CurveToY,
    /// `h` — close subpath.
    ClosePath,
    /// `re` — append rectangle.
    Rectangle,

    // --- Path painting ---
    /// `S` — stroke.
    Stroke,
    /// `s` — close and stroke.
    CloseStroke,
    /// `f` — fill, nonzero winding.
    Fill,
    /// `F` — fill (obsolete synonym for `f`).
    FillObsolete,
    /// `f*` — fill, even-odd.
    FillEvenOdd,
    /// `B` — fill then stroke.
    FillStroke,
    /// `B*` — fill then stroke, even-odd.
    FillStrokeEvenOdd,
    /// `b` — close, fill, stroke.
    CloseFillStroke,
    /// `b*` — close, fill, stroke, even-odd.
    CloseFillStrokeEvenOdd,
    /// `n` — end path with no painting.
    EndPath,

    // --- Clipping ---
    /// `W` — clip, nonzero winding.
    Clip,
    /// `W*` — clip, even-odd.
    ClipEvenOdd,

    // --- Text objects ---
    /// `BT` — begin text object.
    BeginText,
    /// `ET` — end text object.
    EndText,

    // --- Text state ---
    /// `Tc` — character spacing.
    CharSpacing,
    /// `Tw` — word spacing.
    WordSpacing,
    /// `Tz` — horizontal scaling, in percent.
    ///
    /// **Implemented.** pdf-inspector has no `Tz` anywhere, so its advance widths are wrong on
    /// every document that uses it.
    HorizontalScale,
    /// `TL` — leading.
    Leading,
    /// `Tf` — font and size.
    SelectFont,
    /// `Tr` — rendering mode.
    RenderMode,
    /// `Ts` — rise.
    Rise,

    // --- Text positioning ---
    /// `Td` — move to the start of the next line, offset from the current line start.
    NextLine,
    /// `TD` — as `Td`, and set leading to `-ty`.
    NextLineSetLeading,
    /// `Tm` — set the text matrix and the text line matrix.
    SetTextMatrix,
    /// `T*` — move to the start of the next line.
    NextLineByLeading,

    // --- Text showing ---
    /// `Tj` — show a string.
    ShowText,
    /// `TJ` — show strings with individual glyph positioning.
    ShowTextAdjusted,
    /// `'` — move to the next line and show a string.
    NextLineShowText,
    /// `"` — set word and character spacing, move to the next line, show a string.
    ///
    /// **Implemented.** Its absence from pdf-inspector's match is the disqualifying defect:
    /// the text vanishes and the output still looks well-formed.
    NextLineShowTextSpacing,

    // --- Type 3 glyph metrics ---
    /// `d0` — glyph width in a Type 3 font.
    Type3Width,
    /// `d1` — glyph width and bounding box in a Type 3 font.
    Type3WidthBBox,

    // --- Colour ---
    /// `CS` — stroking colour space.
    StrokeColorSpace,
    /// `cs` — non-stroking colour space.
    FillColorSpace,
    /// `SC` — stroking colour.
    StrokeColor,
    /// `SCN` — stroking colour, extended.
    StrokeColorN,
    /// `sc` — non-stroking colour.
    FillColor,
    /// `scn` — non-stroking colour, extended.
    FillColorN,
    /// `G` — stroking grey.
    StrokeGray,
    /// `g` — non-stroking grey.
    FillGray,
    /// `RG` — stroking RGB.
    StrokeRgb,
    /// `rg` — non-stroking RGB.
    FillRgb,
    /// `K` — stroking CMYK.
    StrokeCmyk,
    /// `k` — non-stroking CMYK.
    FillCmyk,

    // --- Shading, XObjects, images ---
    /// `sh` — paint a shading.
    Shading,
    /// `Do` — draw an XObject.
    XObject,
    /// `BI` — begin inline image.
    BeginInlineImage,
    /// `ID` — inline image data.
    InlineImageData,
    /// `EI` — end inline image.
    EndInlineImage,

    // --- Marked content ---
    /// `MP` — marked-content point.
    MarkedPoint,
    /// `DP` — marked-content point with a property list.
    MarkedPointProps,
    /// `BMC` — begin marked-content sequence.
    BeginMarkedContent,
    /// `BDC` — begin marked-content sequence with a property list.
    ///
    /// Carries `/MCID`, the honest bridge from a text run to the tagged-structure tree.
    BeginMarkedContentProps,
    /// `EMC` — end marked-content sequence.
    EndMarkedContent,

    // --- Compatibility ---
    /// `BX` — begin compatibility section.
    BeginCompat,
    /// `EX` — end compatibility section.
    EndCompat,
}

impl Operator {
    /// Map a content-stream token to its operator, or `None` if it is not in Table A.1.
    ///
    /// `None` is a **hard error** at the call site, never a skip.
    pub fn from_token(token: &str) -> Option<Self> {
        use Operator::*;
        Some(match token {
            "q" => SaveState,
            "Q" => RestoreState,
            "cm" => ConcatMatrix,
            "w" => LineWidth,
            "J" => LineCap,
            "j" => LineJoin,
            "M" => MiterLimit,
            "d" => DashPattern,
            "ri" => RenderingIntent,
            "i" => Flatness,
            "gs" => ExtGState,

            "m" => MoveTo,
            "l" => LineTo,
            "c" => CurveTo,
            "v" => CurveToV,
            "y" => CurveToY,
            "h" => ClosePath,
            "re" => Rectangle,

            "S" => Stroke,
            "s" => CloseStroke,
            "f" => Fill,
            "F" => FillObsolete,
            "f*" => FillEvenOdd,
            "B" => FillStroke,
            "B*" => FillStrokeEvenOdd,
            "b" => CloseFillStroke,
            "b*" => CloseFillStrokeEvenOdd,
            "n" => EndPath,

            "W" => Clip,
            "W*" => ClipEvenOdd,

            "BT" => BeginText,
            "ET" => EndText,

            "Tc" => CharSpacing,
            "Tw" => WordSpacing,
            "Tz" => HorizontalScale,
            "TL" => Leading,
            "Tf" => SelectFont,
            "Tr" => RenderMode,
            "Ts" => Rise,

            "Td" => NextLine,
            "TD" => NextLineSetLeading,
            "Tm" => SetTextMatrix,
            "T*" => NextLineByLeading,

            "Tj" => ShowText,
            "TJ" => ShowTextAdjusted,
            "'" => NextLineShowText,
            "\"" => NextLineShowTextSpacing,

            "d0" => Type3Width,
            "d1" => Type3WidthBBox,

            "CS" => StrokeColorSpace,
            "cs" => FillColorSpace,
            "SC" => StrokeColor,
            "SCN" => StrokeColorN,
            "sc" => FillColor,
            "scn" => FillColorN,
            "G" => StrokeGray,
            "g" => FillGray,
            "RG" => StrokeRgb,
            "rg" => FillRgb,
            "K" => StrokeCmyk,
            "k" => FillCmyk,

            "sh" => Shading,
            "Do" => XObject,
            "BI" => BeginInlineImage,
            "ID" => InlineImageData,
            "EI" => EndInlineImage,

            "MP" => MarkedPoint,
            "DP" => MarkedPointProps,
            "BMC" => BeginMarkedContent,
            "BDC" => BeginMarkedContentProps,
            "EMC" => EndMarkedContent,

            "BX" => BeginCompat,
            "EX" => EndCompat,

            _ => return None,
        })
    }

    /// The token this operator is written as. Test-only.
    ///
    /// The parse direction is [`Operator::from_token`]; nothing in the engine writes a content
    /// stream, so the inverse exists to let `every_operator_round_trips_through_its_token` prove
    /// the table has no gaps or duplicates. `cfg(test)` since M7, when `ops` became private and
    /// the compiler could finally see that.
    #[cfg(test)]
    pub fn token(self) -> &'static str {
        use Operator::*;
        match self {
            SaveState => "q",
            RestoreState => "Q",
            ConcatMatrix => "cm",
            LineWidth => "w",
            LineCap => "J",
            LineJoin => "j",
            MiterLimit => "M",
            DashPattern => "d",
            RenderingIntent => "ri",
            Flatness => "i",
            ExtGState => "gs",
            MoveTo => "m",
            LineTo => "l",
            CurveTo => "c",
            CurveToV => "v",
            CurveToY => "y",
            ClosePath => "h",
            Rectangle => "re",
            Stroke => "S",
            CloseStroke => "s",
            Fill => "f",
            FillObsolete => "F",
            FillEvenOdd => "f*",
            FillStroke => "B",
            FillStrokeEvenOdd => "B*",
            CloseFillStroke => "b",
            CloseFillStrokeEvenOdd => "b*",
            EndPath => "n",
            Clip => "W",
            ClipEvenOdd => "W*",
            BeginText => "BT",
            EndText => "ET",
            CharSpacing => "Tc",
            WordSpacing => "Tw",
            HorizontalScale => "Tz",
            Leading => "TL",
            SelectFont => "Tf",
            RenderMode => "Tr",
            Rise => "Ts",
            NextLine => "Td",
            NextLineSetLeading => "TD",
            SetTextMatrix => "Tm",
            NextLineByLeading => "T*",
            ShowText => "Tj",
            ShowTextAdjusted => "TJ",
            NextLineShowText => "'",
            NextLineShowTextSpacing => "\"",
            Type3Width => "d0",
            Type3WidthBBox => "d1",
            StrokeColorSpace => "CS",
            FillColorSpace => "cs",
            StrokeColor => "SC",
            StrokeColorN => "SCN",
            FillColor => "sc",
            FillColorN => "scn",
            StrokeGray => "G",
            FillGray => "g",
            StrokeRgb => "RG",
            FillRgb => "rg",
            StrokeCmyk => "K",
            FillCmyk => "k",
            Shading => "sh",
            XObject => "Do",
            BeginInlineImage => "BI",
            InlineImageData => "ID",
            EndInlineImage => "EI",
            MarkedPoint => "MP",
            MarkedPointProps => "DP",
            BeginMarkedContent => "BMC",
            BeginMarkedContentProps => "BDC",
            EndMarkedContent => "EMC",
            BeginCompat => "BX",
            EndCompat => "EX",
        }
    }

    /// Every operator, for tests that must cover the whole table.
    #[cfg(test)]
    pub const ALL: [Self; 73] = {
        use Operator::*;
        [
            SaveState,
            RestoreState,
            ConcatMatrix,
            LineWidth,
            LineCap,
            LineJoin,
            MiterLimit,
            DashPattern,
            RenderingIntent,
            Flatness,
            ExtGState,
            MoveTo,
            LineTo,
            CurveTo,
            CurveToV,
            CurveToY,
            ClosePath,
            Rectangle,
            Stroke,
            CloseStroke,
            Fill,
            FillObsolete,
            FillEvenOdd,
            FillStroke,
            FillStrokeEvenOdd,
            CloseFillStroke,
            CloseFillStrokeEvenOdd,
            EndPath,
            Clip,
            ClipEvenOdd,
            BeginText,
            EndText,
            CharSpacing,
            WordSpacing,
            HorizontalScale,
            Leading,
            SelectFont,
            RenderMode,
            Rise,
            NextLine,
            NextLineSetLeading,
            SetTextMatrix,
            NextLineByLeading,
            ShowText,
            ShowTextAdjusted,
            NextLineShowText,
            NextLineShowTextSpacing,
            Type3Width,
            Type3WidthBBox,
            StrokeColorSpace,
            FillColorSpace,
            StrokeColor,
            StrokeColorN,
            FillColor,
            FillColorN,
            StrokeGray,
            FillGray,
            StrokeRgb,
            FillRgb,
            StrokeCmyk,
            FillCmyk,
            Shading,
            XObject,
            BeginInlineImage,
            InlineImageData,
            EndInlineImage,
            MarkedPoint,
            MarkedPointProps,
            BeginMarkedContent,
            BeginMarkedContentProps,
            EndMarkedContent,
            BeginCompat,
            EndCompat,
        ]
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_operator_round_trips_through_its_token() {
        for op in Operator::ALL {
            assert_eq!(
                Operator::from_token(op.token()),
                Some(op),
                "`{}` does not round-trip",
                op.token()
            );
        }
    }

    #[test]
    fn the_table_has_no_duplicate_tokens() {
        let mut tokens: Vec<&str> = Operator::ALL.iter().map(|o| o.token()).collect();
        let n = tokens.len();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(tokens.len(), n, "duplicate token in the operator table");
    }

    /// The four show-text operators, all present.
    ///
    /// pdf-inspector omits `"`. Its text vanishes silently and the surrounding runs merge with
    /// corrupt geometry (parity checklist P6).
    #[test]
    fn all_four_show_text_operators_exist() {
        assert_eq!(Operator::from_token("Tj"), Some(Operator::ShowText));
        assert_eq!(Operator::from_token("TJ"), Some(Operator::ShowTextAdjusted));
        assert_eq!(Operator::from_token("'"), Some(Operator::NextLineShowText));
        assert_eq!(
            Operator::from_token("\""),
            Some(Operator::NextLineShowTextSpacing),
            "the `\"` operator is the pdf-inspector disqualifier"
        );
    }

    #[test]
    fn horizontal_scaling_exists() {
        // pdf-inspector has no `Tz` anywhere in its tree, so its advances are wrong wherever a
        // document uses it.
        assert_eq!(Operator::from_token("Tz"), Some(Operator::HorizontalScale));
    }

    #[test]
    fn unknown_tokens_map_to_nothing() {
        for token in [
            "Xy",
            "TjTj",
            "",
            " ",
            "Q2",
            "UnknownOp",
            "tj",
            "bt",
            "\u{1F4A1}",
        ] {
            assert_eq!(
                Operator::from_token(token),
                None,
                "`{token}` must not resolve to an operator"
            );
        }
    }

    #[test]
    fn tokens_are_case_sensitive() {
        // `f` and `F` are different operators; `B` and `b` differ in whether the path closes.
        assert_ne!(Operator::from_token("f"), Operator::from_token("F"));
        assert_ne!(Operator::from_token("B"), Operator::from_token("b"));
        assert_ne!(Operator::from_token("W"), Operator::from_token("w"));
        assert_ne!(Operator::from_token("G"), Operator::from_token("g"));
    }

    #[test]
    fn star_variants_are_distinct_from_their_bases() {
        assert_ne!(Operator::from_token("f*"), Operator::from_token("f"));
        assert_ne!(Operator::from_token("B*"), Operator::from_token("B"));
        assert_ne!(Operator::from_token("W*"), Operator::from_token("W"));
        assert_ne!(Operator::from_token("T*"), Operator::from_token("Tj"));
    }

    #[test]
    fn the_declared_length_matches_the_table() {
        assert_eq!(Operator::ALL.len(), 73);
        // Every entry distinct.
        let mut seen = std::collections::BTreeSet::new();
        for op in Operator::ALL {
            assert!(seen.insert(op), "{op:?} appears twice in ALL");
        }
    }
}
