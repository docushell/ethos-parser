//! Does the text this engine declares undescended actually exist?
//!
//! `form-xobjects-not-descended` counts `Do` calls whose XObject is not a readable `/Image`.
//! That set includes forms holding nothing but vector art, so the count alone cannot size the
//! prize. This opens each `/Form` a page's resources declare and asks one question of its
//! content stream: does it show any text.
use std::collections::BTreeSet;
use std::env;

fn main() {
    let mut docs = 0usize;
    let mut docs_with_forms = 0usize;
    let mut forms = 0usize;
    let mut forms_with_text = 0usize;
    let mut show_ops_total = 0usize;
    let mut shown_bytes_total = 0usize;
    let mut failed = 0usize;
    let mut worst: Vec<(String, usize, usize)> = Vec::new();

    for path in env::args().skip(1) {
        let doc = match lopdf::Document::load(&path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("! {path}: {e}");
                failed += 1;
                continue;
            }
        };
        docs += 1;
        let mut seen: BTreeSet<lopdf::ObjectId> = BTreeSet::new();
        let mut doc_forms = 0usize;
        let mut doc_text_forms = 0usize;
        let mut doc_show = 0usize;

        for (_, page_id) in doc.get_pages() {
            let Ok(res) = doc.get_dictionary(page_id).and_then(|p| {
                p.get(b"Resources").and_then(|r| match r {
                    lopdf::Object::Reference(id) => doc.get_dictionary(*id),
                    lopdf::Object::Dictionary(d) => Ok(d),
                    _ => Err(lopdf::Error::ObjectNotFound(page_id)),
                })
            }) else {
                continue;
            };
            let Ok(xo) = res.get(b"XObject").and_then(|x| match x {
                lopdf::Object::Reference(id) => doc.get_dictionary(*id),
                lopdf::Object::Dictionary(d) => Ok(d),
                _ => Err(lopdf::Error::ObjectNotFound(page_id)),
            }) else {
                continue;
            };
            for (_, obj) in xo.iter() {
                let lopdf::Object::Reference(id) = obj else { continue };
                if !seen.insert(*id) {
                    continue;
                }
                let Ok(stream) = doc.get_object(*id).and_then(|o| o.as_stream()) else { continue };
                let subtype = stream.dict.get(b"Subtype").ok().and_then(|s| s.as_name().ok());
                if subtype != Some(b"Form".as_ref()) {
                    continue;
                }
                doc_forms += 1;
                let Ok(content) = stream.decompressed_content() else { continue };
                let Ok(ops) = lopdf::content::Content::decode(&content) else { continue };
                let mut shows = 0usize;
                let mut bytes = 0usize;
                for op in &ops.operations {
                    if !matches!(op.operator.as_str(), "Tj" | "TJ" | "'" | "\"") {
                        continue;
                    }
                    shows += 1;
                    // Count the bytes actually shown: a single `Tj` can draw a paragraph, so an
                    // operation count does not bound text volume and cannot decide this on its own.
                    for operand in &op.operands {
                        match operand {
                            lopdf::Object::String(b, _) => bytes += b.len(),
                            lopdf::Object::Array(items) => {
                                for it in items {
                                    if let lopdf::Object::String(b, _) = it {
                                        bytes += b.len();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                shown_bytes_total += bytes;
                if shows > 0 {
                    doc_text_forms += 1;
                    doc_show += shows;
                }
            }
        }
        forms += doc_forms;
        forms_with_text += doc_text_forms;
        show_ops_total += doc_show;
        if doc_forms > 0 {
            docs_with_forms += 1;
            if doc_show > 0 {
                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                worst.push((name, doc_text_forms, doc_show));
            }
        }
    }

    println!("documents read                 : {docs}  (failed to load: {failed})");
    println!("documents declaring a /Form    : {docs_with_forms}");
    println!("/Form XObjects found           : {forms}");
    println!("...of which SHOW ANY TEXT      : {forms_with_text}");
    println!("total show operations in them  : {show_ops_total}");
    println!("total BYTES those ops show    : {shown_bytes_total}");
    worst.sort_by_key(|w| std::cmp::Reverse(w.2));
    println!("\ndocuments whose forms show text, most first:");
    for (name, nforms, shows) in worst.iter().take(15) {
        println!("   {name:26} forms_with_text={nforms:3}  show_ops={shows}");
    }
}
