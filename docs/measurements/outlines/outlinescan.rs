//! What does a `/Outlines` tree in this repository's corpus actually contain?
//!
//! Emits one JSON record per document so the numbers a scope document would rest on are
//! measured rather than asserted: entry count, declared depth, destinations that resolve to a
//! page and destinations that do not, and whether the page order the tree declares ever steps
//! backward.
//!
//! Title decoding follows PDF 32000-1 §7.9.2.2: a text string is UTF-16BE when it opens with
//! `FE FF`, and PDFDocEncoding otherwise. Only the Latin-1-identical part of PDFDocEncoding is
//! decoded here and anything else is counted as `title_undecodable` rather than guessed — this is
//! a measurement, and a title invented to make a total look tidy would be the defect it is
//! measuring for.
use std::collections::BTreeSet;
use std::env;

#[derive(Default)]
struct Doc {
    entries: usize,
    max_depth: usize,
    resolved: usize,
    unresolved: usize,
    backward: usize,
    empty_title: usize,
    undecodable: usize,
    /// Walks abandoned early: a repeated object id, or a /First or /Next that will not resolve.
    /// Counted so a truncated walk is distinguishable from a complete one — without this, an
    /// undercount and a correct count look identical in the output.
    truncated: usize,
    pages: Vec<(String, usize)>, // (title, 1-based page) for the join in the next stage
}

fn text_string(bytes: &[u8]) -> Option<String> {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect();
        return String::from_utf16(&units).ok();
    }
    // PDFDocEncoding agrees with Latin-1 over 0x20..0x7E and 0xA0..0xFF; the 0x80..0x9F block
    // differs and is not guessed.
    if bytes.iter().all(|b| (0x20..=0x7E).contains(b) || *b >= 0xA0) {
        return Some(bytes.iter().map(|b| *b as char).collect());
    }
    None
}

/// The page an outline item points at, as a 1-based index, via `/Dest` or `/A` with `/GoTo`.
fn dest_page(
    doc: &lopdf::Document,
    item: &lopdf::Dictionary,
    page_index: &std::collections::HashMap<lopdf::ObjectId, usize>,
) -> Option<usize> {
    let target = item
        .get(b"Dest")
        .ok()
        .cloned()
        .or_else(|| {
            let a = item.get(b"A").ok()?;
            let a = match a {
                lopdf::Object::Reference(id) => doc.get_dictionary(*id).ok()?.clone(),
                lopdf::Object::Dictionary(d) => d.clone(),
                _ => return None,
            };
            if a.get(b"S").ok()?.as_name().ok()? != b"GoTo" {
                return None;
            }
            a.get(b"D").ok().cloned()
        })?;
    // A destination is an array whose first element is the page, or a name/string into /Dests.
    let array = match target {
        lopdf::Object::Array(items) => items,
        lopdf::Object::Name(ref n) => named_dest(doc, n)?,
        lopdf::Object::String(ref s, _) => named_dest(doc, s)?,
        lopdf::Object::Reference(id) => match doc.get_object(id).ok()? {
            lopdf::Object::Array(items) => items.clone(),
            _ => return None,
        },
        _ => return None,
    };
    match array.first()? {
        lopdf::Object::Reference(id) => page_index.get(id).copied(),
        // An integer first element is a page *number* in a remote destination; not this document.
        _ => None,
    }
}

/// `/Names` → `/Dests` name tree, and the older `/Dests` dictionary in the catalog.
fn named_dest(doc: &lopdf::Document, name: &[u8]) -> Option<Vec<lopdf::Object>> {
    let catalog = doc.catalog().ok()?;
    let unwrap = |o: &lopdf::Object| -> Option<Vec<lopdf::Object>> {
        let o = match o {
            lopdf::Object::Reference(id) => doc.get_object(*id).ok()?,
            other => other,
        };
        match o {
            lopdf::Object::Array(items) => Some(items.clone()),
            lopdf::Object::Dictionary(d) => match d.get(b"D").ok()? {
                lopdf::Object::Array(items) => Some(items.clone()),
                lopdf::Object::Reference(id) => match doc.get_object(*id).ok()? {
                    lopdf::Object::Array(items) => Some(items.clone()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    };
    if let Ok(dests) = catalog.get(b"Dests") {
        let d = match dests {
            lopdf::Object::Reference(id) => doc.get_dictionary(*id).ok()?,
            lopdf::Object::Dictionary(d) => d,
            _ => return None,
        };
        if let Ok(hit) = d.get(name) {
            return unwrap(hit);
        }
    }
    // The /Names name tree. Walk /Kids to the leaf whose /Names array holds the key.
    let names = catalog.get(b"Names").ok()?;
    let names = match names {
        lopdf::Object::Reference(id) => doc.get_dictionary(*id).ok()?,
        lopdf::Object::Dictionary(d) => d,
        _ => return None,
    };
    let root = names.get(b"Dests").ok()?;
    let mut stack = vec![match root {
        lopdf::Object::Reference(id) => doc.get_dictionary(*id).ok()?.clone(),
        lopdf::Object::Dictionary(d) => d.clone(),
        _ => return None,
    }];
    let mut guard = 0;
    while let Some(node) = stack.pop() {
        guard += 1;
        if guard > 10_000 {
            return None;
        }
        if let Ok(lopdf::Object::Array(pairs)) = node.get(b"Names") {
            for pair in pairs.chunks(2) {
                if let (Some(lopdf::Object::String(k, _)), Some(v)) = (pair.first(), pair.get(1)) {
                    if k.as_slice() == name {
                        return unwrap(v);
                    }
                }
            }
        }
        if let Ok(lopdf::Object::Array(kids)) = node.get(b"Kids") {
            for kid in kids {
                if let lopdf::Object::Reference(id) = kid {
                    if let Ok(d) = doc.get_dictionary(*id) {
                        stack.push(d.clone());
                    }
                }
            }
        }
    }
    None
}

fn walk(
    doc: &lopdf::Document,
    first: lopdf::ObjectId,
    depth: usize,
    seen: &mut BTreeSet<lopdf::ObjectId>,
    page_index: &std::collections::HashMap<lopdf::ObjectId, usize>,
    out: &mut Doc,
    last_page: &mut usize,
) {
    let mut cur = Some(first);
    while let Some(id) = cur {
        if !seen.insert(id) {
            out.truncated += 1; // a cycling /Next or /First; refuse rather than loop
            return;
        }
        let Ok(item) = doc.get_dictionary(id) else {
            out.truncated += 1;
            return;
        };
        out.entries += 1;
        out.max_depth = out.max_depth.max(depth);

        match item.get(b"Title").ok() {
            Some(lopdf::Object::String(bytes, _)) => match text_string(bytes) {
                Some(t) if t.trim().is_empty() => out.empty_title += 1,
                Some(t) => {
                    if let Some(p) = dest_page(doc, item, page_index) {
                        out.pages.push((t, p + 1));
                    }
                }
                None => {
                    out.undecodable += 1;
                    let odd: Vec<String> = bytes
                        .iter()
                        .filter(|b| **b < 0x20 || (0x7F..0xA0).contains(*b))
                        .map(|b| format!("{b:#04x}"))
                        .collect();
                    eprintln!("UNDEC {odd:?} raw={bytes:?}");
                }
            },
            _ => out.empty_title += 1,
        }

        match dest_page(doc, item, page_index) {
            Some(p) => {
                out.resolved += 1;
                if p + 1 < *last_page {
                    out.backward += 1;
                }
                *last_page = p + 1;
            }
            None => out.unresolved += 1,
        }

        if let Ok(lopdf::Object::Reference(child)) = item.get(b"First") {
            walk(doc, *child, depth + 1, seen, page_index, out, last_page);
        }
        cur = match item.get(b"Next") {
            Ok(lopdf::Object::Reference(n)) => Some(*n),
            _ => None,
        };
    }
}

fn main() {
    let mut all = Vec::new();
    for path in env::args().skip(1) {
        let Ok(doc) = lopdf::Document::load(&path) else {
            eprintln!("! {path}: will not load");
            continue;
        };
        let page_index: std::collections::HashMap<lopdf::ObjectId, usize> = doc
            .get_pages()
            .into_iter()
            .enumerate()
            .map(|(i, (_, id))| (id, i))
            .collect();
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        let Ok(catalog) = doc.catalog() else { continue };
        let Ok(lopdf::Object::Reference(outlines)) = catalog.get(b"Outlines") else {
            println!("{{\"doc\":{name:?},\"outlines\":false}}");
            continue;
        };
        let Ok(root) = doc.get_dictionary(*outlines) else { continue };
        let mut d = Doc::default();
        if let Ok(lopdf::Object::Reference(first)) = root.get(b"First") {
            let mut seen = BTreeSet::new();
            let mut last = 0usize;
            walk(&doc, *first, 1, &mut seen, &page_index, &mut d, &mut last);
        }
        println!(
            "{{\"doc\":{:?},\"outlines\":true,\"entries\":{},\"max_depth\":{},\"resolved\":{},\
             \"unresolved\":{},\"backward\":{},\"empty_title\":{},\"undecodable\":{},\"truncated\":{},\"pages\":{}}}",
            name, d.entries, d.max_depth, d.resolved, d.unresolved, d.backward, d.empty_title,
            d.undecodable, d.truncated, doc.get_pages().len()
        );
        let titles: Vec<String> = d
            .pages
            .iter()
            .map(|(t, p)| format!("{{\"title\":{t:?},\"page\":{p}}}"))
            .collect();
        eprintln!("TITLES {name} [{}]", titles.join(","));
        all.push(name);
    }
    eprintln!("# {} document(s) scanned", all.len());
}
