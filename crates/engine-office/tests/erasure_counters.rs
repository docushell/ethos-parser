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

//! v2-S9.1: no A14 erasure counter in this crate reaches its total by an operation that can wrap.
//!
//! The per-reader tests beside each reader prove the fold saturates. This one proves the readers
//! still use it. The two are not the same guard: a fold that is correct and a reader that stopped
//! calling it would leave every one of those tests passing, which is exactly how v2-S9's defect
//! survived — the EPUB sites were repaired and the other seven readers kept the plain `+=` that
//! the review had already reproduced as an 85× under-declaration.

use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every counter declared under **A14**, by the name it carries in the source.
///
/// **The list is the guard.** A counter missing from it is a counter nothing watches, and this
/// list shipped four short: `spine_items_not_read`, `text_outside_a_block`, `undecodable_bytes`
/// and `other_kinds` all reach an artifact as a declared erasure and none of them was named here.
/// `other_kinds` was the one that mattered — it was still on a plain `+=` in two files, because
/// v2-S9.1's own search was for names ending `_not_read` and it does not end that way.
/// [`the_counter_list_is_complete`] now derives the list from the source instead of trusting it.
const ERASURE_COUNTERS: [&str; 11] = [
    "regions_not_read",
    "foreign_text_not_read",
    "shapes_not_read",
    "alternatives_not_read",
    "destinations_not_read",
    "text_outside_a_cell",
    "text_outside_a_shape",
    "text_outside_a_block",
    "spine_items_not_read",
    "undecodable_bytes",
    "other_kinds",
];

/// Every file that holds one, including the three the S9.1 site list did not name: `epub.rs`,
/// repaired at v2-S9 and pinned here so it stays repaired, and `docx.rs` and `xlsx.rs`, whose
/// entry-name counts carried the cast half of the same defect.
const READERS: [&str; 9] = [
    "lib.rs", "pptx.rs", "odt.rs", "ods.rs", "odp.rs", "rtf.rs", "epub.rs", "docx.rs", "xlsx.rs",
];

/// `engine-pdf`'s document-level accumulators live in one file, and v2-S9.1 repaired two of them.
///
/// They were guarded by nothing: this file scans `engine-office/src`, and `extract.rs`'s own test
/// exercises the two private helpers rather than the accumulation sites. A path is enough here —
/// the counters are the same shape and the same rule applies to them.
const PDF_EXTRACT: &str = "../../engine-pdf/src/extract.rs";

/// The `engine-pdf` counters, which do not share `engine-office`'s naming.
const PDF_COUNTERS: [&str; 8] = [
    "unclaimed_tree_items",
    "mcids_unbound",
    "composite_fonts",
    "inline_images",
    "unresolved_xobjects",
    "encoding_dropped_runs",
    "props_by_name",
    "unresolved_field_parents",
];

/// An accumulation that cannot wrap: the crate's saturating fold, or `saturating_add` itself.
const SATURATING: [&str; 2] = ["declare(", "saturating_add("];

/// Where each file's shipping half ends.
///
/// The skip is **asserted sound rather than assumed**, and that is not pedantry: the sibling guard
/// in `engine-cli/tests/no_format_cli.rs` shipped with exactly this shape and was wrong, because
/// `engine-pdf/src/lib.rs` carries a `#[cfg(test)]` at line 56 and the skip therefore hid that
/// crate's whole public surface. For these nine files the shape holds — one `#[cfg(test)]`, at the
/// end, introducing nothing but `mod tests` — and [`the_test_region_skip_is_sound`] fails the
/// moment that stops being true.
fn shipping_half(source: &str) -> &str {
    source
        .split_once("#[cfg(test)]")
        .map_or(source, |(before, _)| before)
}

/// Normalise every run of whitespace to one space.
///
/// Without this the guard knows exactly one spelling of `x += 1`, and `x  += 1` or a tab walks
/// straight past it.
fn normalise(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn the_test_region_skip_is_sound() {
    for file in READERS {
        let source = std::fs::read_to_string(src_dir().join(file)).expect("readable source");
        let after = match source.split_once("#[cfg(test)]") {
            Some((_, after)) => after,
            None => continue,
        };
        assert_eq!(
            after.matches("#[cfg(test)]").count(),
            0,
            "{file} has more than one `#[cfg(test)]`, so skipping from the first one hides \
             shipping code between them"
        );
        for line in after.lines() {
            let starts_item = [
                "pub fn ",
                "pub(crate) fn ",
                "fn ",
                "pub struct ",
                "struct ",
                "pub enum ",
                "enum ",
                "pub const ",
                "const ",
                "impl ",
                "pub use ",
                "pub mod ",
                "mod ",
            ]
            .iter()
            .any(|k| line.starts_with(k));
            if starts_item {
                assert!(
                    line.starts_with("mod tests"),
                    "{file} declares `{}` after its first `#[cfg(test)]`. The skip in \
                     `shipping_half` would hide it.",
                    line.trim()
                );
            }
        }
    }
}

/// **The list above is derived, not trusted.**
///
/// Every erasure counter in this crate is a `let mut x = 0u32;` local that ends up on a returned
/// struct. Deriving the set from the source and comparing it to [`ERASURE_COUNTERS`] is what turns
/// the list from a thing someone remembered to update into a thing that cannot silently ship
/// short — which it did: `other_kinds` was missing, and it was missing *and* still on a plain
/// `+=`, in two files.
#[test]
fn the_counter_list_is_complete() {
    let mut derived: Vec<String> = Vec::new();
    for file in READERS {
        let source = std::fs::read_to_string(src_dir().join(file)).expect("readable source");
        for line in source.lines() {
            let line = normalise(line);
            let Some(rest) = line.strip_prefix("let mut ") else {
                continue;
            };
            let Some((name, tail)) = rest.split_once(" = ") else {
                continue;
            };
            if tail.starts_with("0u32") && !derived.iter().any(|d| d == name) {
                derived.push(name.to_string());
            }
        }
    }
    derived.sort();
    let mut listed: Vec<String> = ERASURE_COUNTERS.iter().map(|c| c.to_string()).collect();
    listed.sort();
    assert_eq!(
        derived, listed,
        "the counters this crate actually declares and the ones this file watches have diverged. \
         A counter missing from ERASURE_COUNTERS is a counter nothing guards."
    );
}

#[test]
fn no_erasure_counter_reaches_its_total_by_an_operation_that_can_wrap() {
    let mut accumulations = 0usize;
    let office = READERS.iter().map(|f| (*f, &ERASURE_COUNTERS[..]));
    let pdf = std::iter::once((PDF_EXTRACT, &PDF_COUNTERS[..]));
    for (file, counters) in office.chain(pdf) {
        let path = src_dir().join(file);
        let source = std::fs::read_to_string(&path).expect("readable source");
        for (number, raw) in source.lines().enumerate() {
            let line = normalise(raw);
            // A doc comment naming the defect is the reason the repair is here, not the defect.
            if line.starts_with("//") {
                continue;
            }
            // A `let` binds a counter's starting value; it does not accumulate into it.
            if line.starts_with("let ") || line.contains(" let ") {
                continue;
            }
            for counter in counters {
                // Both spellings of "this line adds into the counter". `x = x + 1` wraps exactly
                // as `x += 1` does, and a guard that only knew the second would have called the
                // first clean.
                if !line.contains(&format!("{counter} +="))
                    && !line.contains(&format!("{counter} = "))
                {
                    continue;
                }
                accumulations += 1;
                assert!(
                    SATURATING.iter().any(|s| line.contains(s)),
                    "{file}:{} accumulates `{counter}` by an operation that can wrap past \
                     `u32::MAX` and come back small — the defect v2-S9 reproduced at 85x. Fold it \
                     through `crate::declare` instead.\n  {}",
                    number + 1,
                    line
                );
                break;
            }
        }
    }
    // A guard that scanned nothing passes every assertion above. This is the floor the nine
    // readers actually reach today; it moves only when a counter is added or removed.
    assert!(
        accumulations >= 30,
        "only {accumulations} accumulation(s) found — the guard is reading the wrong files"
    );
}

/// An `as u32` cast is the worse half of the same defect: it wraps in **debug as well as
/// release**, so no test and no CI job can catch it, where a `+=` at least panics in a debug
/// build. Every count that feeds an erasure counter goes through `crate::declared_len`.
///
/// Scanned over the shipping half only. The `#[cfg(test)]` ZIP builders in `ods.rs` and `epub.rs`
/// write genuine 32-bit ZIP header fields, where `as u32` is the format's own width and not a
/// narrowing of anything counted.
#[test]
fn no_erasure_counter_is_narrowed_by_an_unchecked_cast() {
    let mut scanned = 0usize;
    for file in READERS {
        let path = src_dir().join(file);
        let source = std::fs::read_to_string(&path).expect("readable source");
        for (number, raw) in shipping_half(&source).lines().enumerate() {
            let line = normalise(raw);
            if line.starts_with("//") {
                continue;
            }
            scanned += 1;
            for cast in [".count() as u32", ".len() as u32"] {
                assert!(
                    !line.contains(cast),
                    "{file}:{} narrows an input-driven count with `{cast}`, which wraps silently \
                     in every build profile. Use `crate::declared_len` instead.",
                    number + 1
                );
            }
        }
    }
    assert!(scanned > 2000, "only {scanned} shipping line(s) scanned");
}
