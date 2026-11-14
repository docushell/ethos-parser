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

//! A workbook number format, applied to a stored value (decision #135).
//!
//! `ethos_parser_core::XLSX_TEXT_CODE_RULE_V4` lists what is rendered. Everything is computed on the
//! stored decimal digits — the `<v>` string — so no float rounds a value the file wrote exactly;
//! a float is used only to compare a value with a section's condition. A code this module does not
//! render answers [`Shown::NotRendered`], and the caller shows the stored value and counts it.

/// What a number format makes of a stored value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Shown {
    /// The value as the format displays it.
    Rendered(String),
    /// `General` or `@`: the stored value is what shows.
    General,
    /// A format this build does not render. The stored value shows, counted.
    NotRendered,
}

/// Excel's own ceiling on a format code's length; a longer one is not one Excel wrote.
const MAX_CODE_CHARS: usize = 255;

/// The largest day serial the 1900 date system has: 9999-12-31.
const MAX_SERIAL_1900: u64 = 2_958_465;

/// The largest day serial the 1904 date system has: 9999-12-31.
const MAX_SERIAL_1904: u64 = 2_957_003;

/// The code a built-in `numFmtId` stands for, where ECMA-376 §18.8.30 states one this module
/// renders. 14 and 22, which Excel shows in the reader's short-date order, are written in ISO 8601.
pub(crate) fn builtin(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        9 => "0%",
        10 => "0.00%",
        14 => "yyyy-mm-dd",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "yyyy-mm-dd h:mm",
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        45 => "mm:ss",
        49 => "@",
        _ => return None,
    })
}

/// `stored` as `code` displays it, on the 1904 date system when `date1904`.
pub(crate) fn render(code: &str, stored: &str, date1904: bool) -> Shown {
    let trimmed = code.trim();
    if trimmed.eq_ignore_ascii_case("general") || trimmed == "@" {
        return Shown::General;
    }
    if code.chars().count() > MAX_CODE_CHARS {
        return Shown::NotRendered;
    }
    // A double holds no integer past 309 digits, so a longer one is not a value Excel stored. Excel
    // displays a number from its first fifteen significant digits, so `0.22499999999999998`, the
    // double nearest `0.225`, shows `0.23` under `0.00`.
    let Some(value) = Decimal::parse(stored.trim())
        .filter(|v| v.int.len() <= 309)
        .map(|v| v.significant(15))
    else {
        return Shown::NotRendered;
    };
    let Some(sections) = tokenize(code) else {
        return Shown::NotRendered;
    };
    match pick(&sections, &value) {
        // A negative serial is no date; Excel shows `####`.
        Some((section, _, _)) if value.negative && section.toks.iter().any(Tok::is_date) => {
            Shown::NotRendered
        }
        Some((section, magnitude, minus)) => {
            match section.render(&magnitude, minus, stored.trim(), date1904) {
                Some(text) if !text.is_empty() => Shown::Rendered(text),
                _ => Shown::NotRendered,
            }
        }
        None => Shown::NotRendered,
    }
}

// -------------------------------------------------------------------------------------------
// The stored value, as decimal digits
// -------------------------------------------------------------------------------------------

/// A stored decimal: sign, integer digits without leading zeros, fraction digits without trailing
/// zeros. Zero is two empty strings.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Decimal {
    negative: bool,
    int: String,
    frac: String,
}

impl Decimal {
    /// `-12.5`, `1E-3`, `4.6339E4`. `None` for anything else, or an exponent past 400.
    fn parse(s: &str) -> Option<Self> {
        let (negative, rest) = match s.as_bytes().first()? {
            b'-' => (true, &s[1..]),
            b'+' => (false, &s[1..]),
            _ => (false, s),
        };
        let (mantissa, exponent) = match rest.find(['e', 'E']) {
            Some(at) => (&rest[..at], rest[at + 1..].parse::<i32>().ok()?),
            None => (rest, 0),
        };
        if exponent.abs() > 400 {
            return None;
        }
        let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        if int.is_empty() && frac.is_empty()
            || !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit())
        {
            return None;
        }
        // All digits, and where the point falls in them once the exponent moves it.
        let digits = format!("{int}{frac}");
        let point = int.len() as i64 + exponent as i64;
        let (int, frac) = if point <= 0 {
            (
                String::new(),
                "0".repeat(point.unsigned_abs() as usize) + &digits,
            )
        } else if point as usize >= digits.len() {
            (
                digits.clone() + &"0".repeat(point as usize - digits.len()),
                String::new(),
            )
        } else {
            (
                digits[..point as usize].to_string(),
                digits[point as usize..].to_string(),
            )
        };
        let int = int.trim_start_matches('0').to_string();
        let frac = frac.trim_end_matches('0').to_string();
        Some(Self {
            negative: negative && !(int.is_empty() && frac.is_empty()),
            int,
            frac,
        })
    }

    /// Rounded half away from zero to `digits` significant digits.
    fn significant(&self, digits: usize) -> Self {
        let all = format!("{}{}", self.int, self.frac);
        let Some(first) = all.bytes().position(|b| b != b'0') else {
            return self.clone();
        };
        let cut = first + digits;
        if cut >= all.len() {
            return self.clone();
        }
        let mut kept: Vec<u8> = all.as_bytes()[..cut].to_vec();
        let mut point = self.int.len();
        if all.as_bytes()[cut] >= b'5' {
            let mut carry = true;
            for d in kept.iter_mut().rev() {
                if *d == b'9' {
                    *d = b'0';
                } else {
                    *d += 1;
                    carry = false;
                    break;
                }
            }
            if carry {
                kept.insert(0, b'1');
                point += 1;
            }
        }
        // The digits past the cut are zeros, so the integer part keeps its width.
        kept.resize(kept.len().max(point), b'0');
        let kept = String::from_utf8(kept).unwrap_or_default();
        Self {
            negative: self.negative,
            int: kept[..point].trim_start_matches('0').to_string(),
            frac: kept[point..].trim_end_matches('0').to_string(),
        }
    }

    fn is_zero(&self) -> bool {
        self.int.is_empty() && self.frac.is_empty()
    }

    fn abs(&self) -> Self {
        Self {
            negative: false,
            ..self.clone()
        }
    }

    /// For a condition only: the one place a float is used, to compare.
    fn approx(&self) -> f64 {
        let s = format!(
            "{}{}.{}0",
            if self.negative { "-" } else { "" },
            if self.int.is_empty() { "0" } else { &self.int },
            self.frac
        );
        s.parse().unwrap_or(f64::NAN)
    }

    /// Times 10^`places`, exactly.
    fn shifted(&self, places: usize) -> Self {
        let mut frac = self.frac.clone();
        let mut int = self.int.clone();
        for _ in 0..places {
            if frac.is_empty() {
                int.push('0');
            } else {
                int.push(frac.remove(0));
            }
        }
        Self {
            negative: self.negative,
            int: int.trim_start_matches('0').to_string(),
            frac,
        }
    }

    /// Rounded half away from zero to `places` fraction digits: `(int, frac)` with `frac` exactly
    /// `places` long and `int` without leading zeros.
    fn rounded(&self, places: usize) -> (String, String) {
        let mut frac: Vec<u8> = self.frac.bytes().collect();
        let up = frac.get(places).is_some_and(|d| *d >= b'5');
        frac.resize(places, b'0');
        let mut int: Vec<u8> = self.int.bytes().collect();
        if up {
            let mut carry = true;
            for d in frac.iter_mut().rev().chain(int.iter_mut().rev()) {
                if *d == b'9' {
                    *d = b'0';
                } else {
                    *d += 1;
                    carry = false;
                    break;
                }
            }
            if carry {
                int.insert(0, b'1');
            }
        }
        let int = String::from_utf8(int).unwrap_or_default();
        let frac = String::from_utf8(frac).unwrap_or_default();
        (int.trim_start_matches('0').to_string(), frac)
    }
}

// -------------------------------------------------------------------------------------------
// The code
// -------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Lit(String),
    /// `0` or `#`.
    Digit(u8),
    Point,
    Comma,
    Percent,
    General,
    Year(usize),
    /// An `m` run, month or minute decided by its neighbours.
    M(usize),
    Day(usize),
    Hour(usize),
    Second(usize),
    AmPm,
}

impl Tok {
    fn is_date(&self) -> bool {
        matches!(
            self,
            Tok::Year(_) | Tok::M(_) | Tok::Day(_) | Tok::Hour(_) | Tok::Second(_) | Tok::AmPm
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Section {
    toks: Vec<Tok>,
    condition: Option<(Op, f64)>,
    /// A non-English LCID, which month and day names cannot honour.
    foreign: bool,
}

/// The code's sections, or `None` where it holds anything this module does not render.
fn tokenize(code: &str) -> Option<Vec<Section>> {
    let chars: Vec<char> = code.chars().collect();
    let mut sections = vec![Section::default()];
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let section = sections.last_mut()?;
        let run = |ch: char| {
            chars[i..]
                .iter()
                .take_while(|x| x.eq_ignore_ascii_case(&ch))
                .count()
        };
        match c {
            ';' => {
                sections.push(Section::default());
                i += 1;
            }
            '"' => {
                let end = chars[i + 1..].iter().position(|x| *x == '"')? + i + 1;
                section
                    .toks
                    .push(Tok::Lit(chars[i + 1..end].iter().collect()));
                i = end + 1;
            }
            '\\' => {
                section.toks.push(Tok::Lit(chars.get(i + 1)?.to_string()));
                i += 2;
            }
            '_' => {
                chars.get(i + 1)?;
                section.toks.push(Tok::Lit(" ".into()));
                i += 2;
            }
            '[' => {
                let end = chars[i + 1..].iter().position(|x| *x == ']')? + i + 1;
                let inner: String = chars[i + 1..end].iter().collect();
                bracket(&inner, section)?;
                i = end + 1;
            }
            '0' | '#' => {
                section.toks.push(Tok::Digit(c as u8));
                i += 1;
            }
            '.' => {
                section.toks.push(Tok::Point);
                i += 1;
            }
            ',' => {
                section.toks.push(Tok::Comma);
                i += 1;
            }
            '%' => {
                section.toks.push(Tok::Percent);
                i += 1;
            }
            'y' | 'Y' => {
                let n = run('y');
                section.toks.push(Tok::Year(n));
                i += n;
            }
            'm' | 'M' => {
                let n = run('m');
                section.toks.push(Tok::M(n));
                i += n;
            }
            'd' | 'D' => {
                let n = run('d');
                section.toks.push(Tok::Day(n));
                i += n;
            }
            'h' | 'H' => {
                let n = run('h');
                section.toks.push(Tok::Hour(n));
                i += n;
            }
            's' | 'S' => {
                let n = run('s');
                section.toks.push(Tok::Second(n));
                i += n;
            }
            'a' | 'A' => {
                let word: String = chars[i..chars.len().min(i + 5)].iter().collect();
                if !word.eq_ignore_ascii_case("am/pm") {
                    return None;
                }
                section.toks.push(Tok::AmPm);
                i += 5;
            }
            'g' | 'G' => {
                let word: String = chars[i..chars.len().min(i + 7)].iter().collect();
                if !word.eq_ignore_ascii_case("general") {
                    return None;
                }
                section.toks.push(Tok::General);
                i += 7;
            }
            '$' | '-' | '+' | '/' | '(' | ')' | ':' | '!' | '^' | '&' | '\'' | '~' | '{' | '}'
            | '<' | '>' | '=' | ' ' => {
                section.toks.push(Tok::Lit(c.to_string()));
                i += 1;
            }
            // A currency sign or any other letter outside ASCII is shown as written.
            c if !c.is_ascii() => {
                section.toks.push(Tok::Lit(c.to_string()));
                i += 1;
            }
            // `?`, `*`, `E+`, `@` in a number, `B1`, digits 1–9 and any other ASCII letter.
            _ => return None,
        }
    }
    (sections.len() <= 4).then_some(sections)
}

/// One `[…]`: a colour (ignored), a condition, or `$text-lcid`. Anything else refuses the code.
fn bracket(inner: &str, section: &mut Section) -> Option<()> {
    const COLOURS: [&str; 8] = [
        "black", "blue", "cyan", "green", "magenta", "red", "white", "yellow",
    ];
    let lower = inner.to_ascii_lowercase();
    if COLOURS.contains(&lower.as_str())
        || lower
            .strip_prefix("color")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    {
        return Some(());
    }
    if let Some(rest) = inner.strip_prefix('$') {
        let (text, lcid) = match rest.rfind('-') {
            Some(at) => (&rest[..at], Some(&rest[at + 1..])),
            None => (rest, None),
        };
        if let Some(lcid) = lcid {
            let id = u32::from_str_radix(lcid, 16).ok()?;
            // A calendar or numeral system in the upper half, or the system date and time
            // formats `F800`/`F400`, are the locale's to supply.
            if id >> 16 != 0 || id & 0xF000 == 0xF000 {
                return None;
            }
            section.foreign |= id != 0 && id & 0x3FF != 0x09;
        }
        if !text.is_empty() {
            section.toks.push(Tok::Lit(text.to_string()));
        }
        return Some(());
    }
    let (op, number) = [
        ("<=", Op::Le),
        (">=", Op::Ge),
        ("<>", Op::Ne),
        ("<", Op::Lt),
        (">", Op::Gt),
        ("=", Op::Eq),
    ]
    .iter()
    .find_map(|(spelling, op)| inner.strip_prefix(spelling).map(|n| (*op, n)))?;
    let number: f64 = number.trim().parse().ok()?;
    if section.condition.replace((op, number)).is_some() {
        return None;
    }
    Some(())
}

fn holds(condition: (Op, f64), v: f64) -> bool {
    let (op, n) = condition;
    match op {
        Op::Lt => v < n,
        Op::Le => v <= n,
        Op::Gt => v > n,
        Op::Ge => v >= n,
        Op::Eq => v == n,
        Op::Ne => v != n,
    }
}

/// The section `value` shows in, the magnitude it shows, and whether a minus goes in front.
fn pick<'a>(sections: &'a [Section], value: &Decimal) -> Option<(&'a Section, Decimal, bool)> {
    let conditional = sections.iter().any(|s| s.condition.is_some());
    if conditional {
        // Excel's sign rules for conditional sections are not stated by the code; a negative value
        // under one is not rendered.
        if value.negative {
            return None;
        }
        let v = value.approx();
        for (i, s) in sections.iter().enumerate() {
            match s.condition {
                Some(c) if holds(c, v) => return Some((s, value.clone(), false)),
                Some(_) => {}
                // The first section with no condition takes the rest, once the conditions before
                // it have been tried.
                None if i > 0 => return Some((s, value.clone(), false)),
                None => return None,
            }
        }
        return None;
    }
    let magnitude = value.abs();
    Some(match (sections.len(), value.negative, value.is_zero()) {
        (1, negative, _) => (&sections[0], magnitude, negative),
        (_, true, _) => (&sections[1], magnitude, false),
        (n, false, true) if n >= 3 => (&sections[2], magnitude, false),
        _ => (&sections[0], magnitude, false),
    })
}

impl Section {
    fn render(&self, value: &Decimal, minus: bool, stored: &str, date1904: bool) -> Option<String> {
        if self.toks.iter().any(Tok::is_date) {
            if minus
                || self
                    .toks
                    .iter()
                    .any(|t| matches!(t, Tok::Digit(_) | Tok::Percent))
            {
                return None;
            }
            return self.date(value, date1904);
        }
        let out = self.number(value, stored)?;
        if minus {
            // A value that rounds to nothing keeps or drops its sign by rules the code does not
            // state; it is not rendered.
            if !out.bytes().any(|b| (b'1'..=b'9').contains(&b)) {
                return None;
            }
            return Some(format!("-{out}"));
        }
        Some(out)
    }

    fn number(&self, value: &Decimal, stored: &str) -> Option<String> {
        let toks = &self.toks;
        if toks.contains(&Tok::General) {
            if toks
                .iter()
                .any(|t| matches!(t, Tok::Digit(_) | Tok::Percent))
            {
                return None;
            }
            let shown = stored.trim_start_matches(['-', '+']);
            return Some(
                toks.iter()
                    .map(|t| match t {
                        Tok::Lit(s) => s.as_str(),
                        Tok::General => shown,
                        _ => "",
                    })
                    .collect(),
            );
        }
        let point = toks.iter().position(|t| *t == Tok::Point);
        if toks.iter().filter(|t| **t == Tok::Point).count() > 1 {
            return None;
        }
        let percents = toks.iter().filter(|t| **t == Tok::Percent).count();
        // Two percent signs scale twice, and a `/` among placeholders is a fraction.
        if percents > 1 || toks.contains(&Tok::Lit("/".into())) {
            return None;
        }
        let int_end = point.unwrap_or(toks.len());
        let digit_at: Vec<usize> = (0..int_end)
            .filter(|&i| matches!(toks[i], Tok::Digit(_)))
            .collect();
        if digit_at.is_empty() && toks.iter().any(|t| matches!(t, Tok::Digit(_))) {
            return None;
        }
        let fraction: Vec<u8> = toks[int_end..]
            .iter()
            .filter_map(|t| match t {
                Tok::Digit(d) => Some(*d),
                _ => None,
            })
            .collect();
        // Commas: grouping only between two integer placeholders; anywhere else (a trailing comma
        // scales by a thousand) the code is not rendered.
        let mut group_widths: Vec<usize> = Vec::new();
        for (i, t) in toks.iter().enumerate() {
            if *t != Tok::Comma {
                continue;
            }
            let right = digit_at.iter().filter(|&&d| d > i).count();
            if i > int_end || right == 0 || right == digit_at.len() {
                return None;
            }
            group_widths.push(right);
        }
        group_widths.sort_unstable();
        group_widths.dedup();
        let boundaries = boundaries(&group_widths);

        let value = if percents == 1 {
            value.shifted(2)
        } else {
            value.clone()
        };
        let (int, frac) = value.rounded(fraction.len());

        // The integer digits as the placeholders take them, right to left: the leftmost takes any
        // the others cannot hold, and a `0` with no digit shows one.
        let zero_floor = digit_at
            .iter()
            .rev()
            .enumerate()
            .filter(|(_, &i)| toks[i] == Tok::Digit(b'0'))
            .map(|(k, _)| k + 1)
            .max()
            .unwrap_or(0);
        let width = int.len().max(zero_floor);
        let int = format!("{}{int}", "0".repeat(width - int.len()));
        let placeholders = digit_at.len();
        let mut out = String::new();
        let mut written = 0usize; // integer digits written so far
        for (i, t) in toks[..int_end].iter().enumerate() {
            match t {
                Tok::Digit(_) => {
                    let from_right = digit_at.iter().filter(|&&d| d > i).count();
                    let take = if from_right + 1 == placeholders {
                        width.saturating_sub(from_right)
                    } else {
                        usize::from(from_right < width)
                    };
                    for _ in 0..take {
                        out.push(int.as_bytes()[written] as char);
                        written += 1;
                        let left = width - written;
                        if left > 0 && boundaries.contains(&left) {
                            out.push(',');
                        }
                    }
                }
                Tok::Comma => {}
                Tok::Lit(s) => out.push_str(s),
                Tok::Percent => out.push('%'),
                _ => return None,
            }
        }
        // The fraction: a trailing `#` drops a trailing zero, a `0` never does.
        let mut keep = fraction.len();
        while keep > 0 && fraction[keep - 1] == b'#' && frac.as_bytes()[keep - 1] == b'0' {
            keep -= 1;
        }
        let mut f = 0usize;
        for t in &toks[int_end..] {
            match t {
                Tok::Point => out.push('.'),
                Tok::Digit(_) => {
                    if f < keep {
                        out.push(frac.as_bytes()[f] as char);
                    }
                    f += 1;
                }
                Tok::Lit(s) => out.push_str(s),
                Tok::Percent => out.push('%'),
                _ => return None,
            }
        }
        Some(out)
    }

    fn date(&self, value: &Decimal, date1904: bool) -> Option<String> {
        // The day serial and the second of the day, rounded to the nearest second.
        let day: u64 = if value.int.is_empty() {
            0
        } else if value.int.len() > 9 {
            return None;
        } else {
            value.int.parse().ok()?
        };
        let frac: String = value.frac.chars().take(30).collect();
        let scale = frac.bytes().fold(1u128, |scale, _| scale * 10);
        let numerator: u128 = if frac.is_empty() {
            0
        } else {
            frac.parse().ok()?
        };
        // The whole seconds of the day and what is left over. A remainder within a ten-thousandth
        // of a second is the next second, whatever the code shows; past half a second it is the
        // next second only where the code shows a time, so `yyyy-mm-dd` never shows the next day
        // for a value a moment before midnight (as SheetJS SSF, which emulates Excel, reads it).
        let shown = numerator * 86_400;
        let (whole, rest) = (shown / scale, shown % scale);
        let has_time = self
            .toks
            .iter()
            .any(|t| matches!(t, Tok::Hour(_) | Tok::Second(_) | Tok::AmPm))
            || self
                .toks
                .iter()
                .zip(self.months())
                .any(|(t, month)| !month && matches!(t, Tok::M(_)));
        let up = rest * 10_000 > scale * 9_999 || has_time && rest * 2 >= scale;
        let mut seconds = whole as u64 + u64::from(up);
        let mut day = day;
        if seconds >= 86_400 {
            seconds -= 86_400;
            day += 1;
        }
        let has_date = self
            .toks
            .iter()
            .any(|t| matches!(t, Tok::Year(_) | Tok::Day(_)))
            || self.months().iter().any(|m| *m);
        let max = if date1904 {
            MAX_SERIAL_1904
        } else {
            MAX_SERIAL_1900
        };
        if day > max || has_date && !date1904 && day == 0 {
            return None;
        }
        let (y, mo, d, weekday) = civil(day, date1904);
        let names = self.toks.iter().zip(self.months()).any(|(t, month)| {
            matches!(t, Tok::Day(n) if *n >= 3) || month && matches!(t, Tok::M(n) if *n >= 3)
        });
        if names && self.foreign {
            return None;
        }
        let twelve = self.toks.contains(&Tok::AmPm);
        let (h, mi, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
        let mut out = String::new();
        for (t, month) in self.toks.iter().zip(self.months()) {
            match t {
                Tok::Lit(s) => out.push_str(s),
                Tok::Point => out.push('.'),
                Tok::Comma => out.push(','),
                Tok::Year(n) if *n <= 2 => out.push_str(&format!("{:02}", y % 100)),
                Tok::Year(_) => out.push_str(&format!("{y:04}")),
                Tok::M(n) if month => match n {
                    1 => out.push_str(&mo.to_string()),
                    2 => out.push_str(&format!("{mo:02}")),
                    3 => out.push_str(&MONTHS[mo as usize - 1][..3]),
                    4 => out.push_str(MONTHS[mo as usize - 1]),
                    _ => out.push_str(&MONTHS[mo as usize - 1][..1]),
                },
                Tok::M(1) => out.push_str(&mi.to_string()),
                Tok::M(2) => out.push_str(&format!("{mi:02}")),
                Tok::Day(1) => out.push_str(&d.to_string()),
                Tok::Day(2) => out.push_str(&format!("{d:02}")),
                Tok::Day(3) => out.push_str(&DAYS[weekday][..3]),
                Tok::Day(_) => out.push_str(DAYS[weekday]),
                Tok::Hour(n) => {
                    let h = if twelve { (h + 11) % 12 + 1 } else { h };
                    if *n == 1 {
                        out.push_str(&h.to_string());
                    } else {
                        out.push_str(&format!("{h:02}"));
                    }
                }
                Tok::Second(1) => out.push_str(&s.to_string()),
                Tok::Second(_) => out.push_str(&format!("{s:02}")),
                Tok::AmPm => out.push_str(if h < 12 { "AM" } else { "PM" }),
                // Three or more `m` beside an hour or a second, `General`, or a digit placeholder.
                _ => return None,
            }
        }
        Some(out)
    }

    /// Whether each token is a month: an `m` run is a minute right after an hour or right before a
    /// second (literals between are passed over), a month everywhere else.
    fn months(&self) -> Vec<bool> {
        let timed: Vec<(usize, &Tok)> = self
            .toks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_date())
            .collect();
        let mut out = vec![false; self.toks.len()];
        for (k, (i, t)) in timed.iter().enumerate() {
            if let Tok::M(_) = t {
                let after_hour = k > 0 && matches!(timed[k - 1].1, Tok::Hour(_));
                let before_second = timed
                    .get(k + 1)
                    .is_some_and(|(_, n)| matches!(n, Tok::Second(_)));
                out[*i] = !(after_hour || before_second);
            }
        }
        out
    }
}

/// The digit counts from the right after which a grouping separator falls. Any comma between two
/// digit placeholders asks for grouping, by threes — except the Indian pattern, commas three and
/// then every two digits from the right (`#,##,##0` and its longer forms), which groups as it is
/// written: `12,34,567`, as Excel shows it on an Indian locale. `widths` are the placeholder counts
/// right of each comma, ascending. Bounded by the longest integer a stored value can hold.
fn boundaries(widths: &[usize]) -> Vec<usize> {
    if widths.is_empty() {
        return Vec::new();
    }
    let lakh =
        widths.len() >= 2 && widths[0] == 3 && widths.windows(2).all(|pair| pair[1] == pair[0] + 2);
    let mut out = Vec::new();
    let mut at = 3usize;
    while at < 1_000 {
        out.push(at);
        at += if lakh { 2 } else { 3 };
    }
    out
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Indexed by days since a Saturday.
const DAYS: [&str; 7] = [
    "Saturday",
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
];

/// A day serial as `(year, month, day, weekday)`, weekday counted from Saturday.
///
/// The 1900 system counts 29 February 1900, which the calendar does not have: serial 60 is that
/// day, serials before it are one day later than the calendar's count would make them, and Excel's
/// weekday for them follows its own count rather than the calendar's — serial 1 is a Sunday.
fn civil(serial: u64, date1904: bool) -> (i64, u32, u32, usize) {
    let serial = serial as i64;
    if date1904 {
        let days = days_from_civil(1904, 1, 1) + serial;
        let (y, m, d) = civil_from_days(days);
        // 1970-01-01, day 0, was a Thursday: five days after a Saturday.
        return (y, m, d, (days + 5).rem_euclid(7) as usize);
    }
    let weekday = serial.rem_euclid(7) as usize;
    if serial == 60 {
        return (1900, 2, 29, weekday);
    }
    let base = if serial < 60 {
        days_from_civil(1899, 12, 31)
    } else {
        days_from_civil(1899, 12, 30)
    };
    let (y, m, d) = civil_from_days(base + serial);
    (y, m, d, weekday)
}

/// Days since 1970-01-01 (H. Hinnant's algorithm).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(code: &str, stored: &str) -> String {
        match render(code, stored, false) {
            Shown::Rendered(s) => s,
            other => panic!("`{code}` on `{stored}`: {other:?}"),
        }
    }

    #[test]
    fn a_date_serial_shows_as_its_format_writes_it() {
        assert_eq!(shown(builtin(14).unwrap(), "46339"), "2026-11-13");
        assert_eq!(shown("dd-mm-yyyy", "46339"), "13-11-2026");
        assert_eq!(shown(builtin(15).unwrap(), "46339"), "13-Nov-26");
        assert_eq!(shown(builtin(16).unwrap(), "46339"), "13-Nov");
        assert_eq!(shown(builtin(17).unwrap(), "46339"), "Nov-26");
        assert_eq!(
            shown("dddd, mmmm d, yyyy", "46339"),
            "Friday, November 13, 2026"
        );
        assert_eq!(shown(builtin(22).unwrap(), "46339.5"), "2026-11-13 12:00");
        assert_eq!(shown("m/d/yy", "1"), "1/1/00");
    }

    #[test]
    fn the_1900_system_counts_the_day_the_calendar_does_not() {
        assert_eq!(shown("yyyy-mm-dd", "59"), "1900-02-28");
        assert_eq!(shown("yyyy-mm-dd", "60"), "1900-02-29");
        assert_eq!(shown("yyyy-mm-dd", "61"), "1900-03-01");
        assert_eq!(shown("ddd", "1"), "Sun");
        assert_eq!(shown("ddd", "61"), "Thu");
        assert_eq!(render("yyyy-mm-dd", "0", false), Shown::NotRendered);
    }

    #[test]
    fn the_1904_system_starts_on_its_own_day() {
        assert_eq!(
            render("yyyy-mm-dd ddd", "0", true),
            Shown::Rendered("1904-01-01 Fri".into())
        );
        assert_eq!(
            render("yyyy-mm-dd", "44877", true),
            Shown::Rendered("2026-11-13".into())
        );
    }

    #[test]
    fn times_take_minutes_beside_hours_and_seconds() {
        assert_eq!(shown(builtin(18).unwrap(), "0.75"), "6:00 PM");
        assert_eq!(shown(builtin(19).unwrap(), "0.5"), "12:00:00 PM");
        assert_eq!(
            shown(builtin(21).unwrap(), "0.0000115740740740741"),
            "0:00:01"
        );
        assert_eq!(shown(builtin(45).unwrap(), "0.0006944444444"), "01:00");
        assert_eq!(shown("h:mm", "0.99999999"), "0:00");
    }

    #[test]
    fn numbers_round_on_their_stored_digits() {
        assert_eq!(shown("0.00", "39000.5"), "39000.50");
        assert_eq!(shown("0.00", "2.675"), "2.68");
        assert_eq!(shown("0", "0.5"), "1");
        assert_eq!(shown("#,##0", "1234567"), "1,234,567");
        assert_eq!(shown("#,##0.00", "-1234.5"), "-1,234.50");
        assert_eq!(shown("0.0#", "1.5"), "1.5");
        assert_eq!(shown("0%", "0.175"), "18%");
        assert_eq!(shown("0.00", "4.6339E4"), "46339.00");
        assert_eq!(shown("0.000", "1E-3"), "0.001");
        assert_eq!(shown("000", "7"), "007");
        assert_eq!(shown(builtin(39).unwrap(), "-5"), "(5.00)");
        assert_eq!(shown(builtin(37).unwrap(), "5"), "5 ");
    }

    #[test]
    fn a_value_shows_from_its_first_fifteen_significant_digits() {
        assert_eq!(shown("0.00", "0.22499999999999998"), "0.23");
        assert_eq!(shown("0.00", "1.0049999999999999"), "1.01");
        assert_eq!(shown("0%", "0.17499999999999999"), "18%");
        assert_eq!(shown("0", "1.2345678901234567E+19"), "12345678901234600000");
        assert_eq!(shown("0.00", "99.999999999999999"), "100.00");
        assert_eq!(shown("0.0", "0.000123456789012345678"), "0.0");
    }

    #[test]
    fn a_date_a_moment_before_midnight_is_still_that_day() {
        assert_eq!(shown("yyyy-mm-dd", "46339.99999999"), "2026-11-13");
        assert_eq!(
            shown("yyyy-mm-dd h:mm", "46339.99999999"),
            "2026-11-14 0:00"
        );
        assert_eq!(shown("yyyy-mm-dd", "46339.9999999999"), "2026-11-14");
        assert_eq!(shown("h:mm:ss", "0.5000057"), "12:00:00");
        assert_eq!(shown("h:mm:ss", "0.500006"), "12:00:01");
    }

    #[test]
    fn grouping_follows_the_widths_the_code_writes() {
        // Any other comma pattern groups by threes.
        assert_eq!(shown("#,#,#0", "1234567"), "1,234,567");
        assert_eq!(shown("#,###,##0", "1234567"), "1,234,567");
        assert_eq!(shown("##,##0.00", "1234567"), "1,234,567.00");
        assert_eq!(shown("#,##,##0.00", "1234567.891"), "12,34,567.89");
        assert_eq!(shown("#,##,##0.00", "116000"), "1,16,000.00");
        assert_eq!(shown("#,##,##0.00", "999"), "999.00");
        assert_eq!(shown("#,##,##0", "123456789"), "12,34,56,789");
    }

    #[test]
    fn a_literal_prefix_and_lakh_sections_render_as_excel_shows_them() {
        let lakh = "[>=10000000]##\\,##\\,##\\,##0.00;[>=100000]##\\,##\\,##0.00;##,##0.00";
        assert_eq!(shown(lakh, "116000"), "1,16,000.00");
        assert_eq!(shown(lakh, "1234567.89"), "12,34,567.89");
        assert_eq!(shown(lakh, "12345678.9"), "1,23,45,678.90");
        assert_eq!(shown(lakh, "99999"), "99,999.00");
        let rupee = "[>=100000]\"₹\"##\\,##\\,##0.00;\"₹\"##,##0.00";
        assert_eq!(shown(rupee, "493098.5"), "₹4,93,098.50");
        assert_eq!(shown(rupee, "4500"), "₹4,500.00");
        assert_eq!(shown("[$₹-4009]#,##,##0.00", "493098.5"), "₹4,93,098.50");
        assert_eq!(shown("#,##0.00_);(#,##0.00)", "5"), "5.00 ");
        assert_eq!(shown("\"Rs. \"General", "12.5"), "Rs. 12.5");
    }

    #[test]
    fn what_is_not_rendered_says_so() {
        for code in [
            "0.00E+00",
            "# ?/?",
            "[h]:mm:ss",
            "* #,##0",
            "#,##0,",
            "[$-F800]dddd, mmmm dd, yyyy",
            "[$-407]mmmm",
            "ss.0",
            "B1yyyy",
            "x0",
        ] {
            assert_eq!(render(code, "5", false), Shown::NotRendered, "{code}");
        }
        // A condition on a negative value, a negative date, and a stored value that is no number.
        assert_eq!(render("[>=0]0;0", "-5", false), Shown::NotRendered);
        assert_eq!(render("yyyy", "-5", false), Shown::NotRendered);
        assert_eq!(render("0.00", "abc", false), Shown::NotRendered);
        assert_eq!(render("0.0", "-0.01", false), Shown::NotRendered);
        assert_eq!(render(&"0".repeat(256), "5", false), Shown::NotRendered);
        assert_eq!(render("General", "5", false), Shown::General);
        assert_eq!(render("@", "5", false), Shown::General);
        assert_eq!(
            render("[$-409]mmmm", "46339", false),
            Shown::Rendered("November".into())
        );
    }
}
