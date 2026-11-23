// Copyright 2026 docuShell
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

//! Decision #168: a form's empty boxes, read from the squares a page outlines, on the record and
//! in Markdown.

use ethos_parser_core::{BoxSide, Profile};
use ethos_parser_pdf::Document;
use lopdf::{dictionary, Object, Stream};
use std::path::PathBuf;

/// Two pages of Helvetica, each drawing `streams[i]`.
fn pages(streams: &[&[u8]]) -> Vec<u8> {
    let mut doc = lopdf::Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut kids: Vec<Object> = Vec::new();
    for stream in streams {
        let content = doc.add_object(Stream::new(dictionary! {}, stream.to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 200.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
            "Contents" => content,
        });
        kids.push(page.into());
    }
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

/// **A box drawn before an option and one drawn after a caption are read beside them, by the ids
/// their runs take in reading order across pages** — the page draws its lower line first — and
/// Markdown writes `☐` there; a square filled in black, and one with a tick drawn inside it, are
/// none.
#[test]
fn a_form_s_empty_boxes_are_read_beside_their_runs() {
    let bytes = pages(&[
        b"BT /F1 12 Tf 1 0 0 1 40 150 Tm (Page one) Tj ET",
        b"BT /F1 10 Tf 1 0 0 1 52 120 Tm (No) Tj ET \
          BT /F1 10 Tf 1 0 0 1 120 150 Tm (Sign) Tj ET \
          BT /F1 10 Tf 1 0 0 1 52 150 Tm (Yes) Tj ET \
          BT /F1 10 Tf 1 0 0 1 52 90 Tm (Maybe) Tj ET \
          BT /F1 10 Tf 1 0 0 1 52 60 Tm (Off) Tj ET \
          0.5 w 40 149 9 9 re S \
          150 149 9 9 re S \
          67 119 9 9 re S \
          40 59 9 9 re f \
          40 89 9 9 re S 41 90 m 48 97 l S",
    ]);
    let profile = Profile::default();
    let doc = Document::open_bytes(&bytes, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::into_representation(extract, &profile).expect("seals");
    let payload = repr.payload();
    assert!(payload.pages[0].drawn_boxes.is_empty());
    let text_of = |id: &ethos_parser_core::NodeId| {
        payload
            .nodes
            .iter()
            .find(|n| &n.id == id)
            .map(|n| n.text.clone())
            .unwrap()
    };
    let mut boxes: Vec<(String, BoxSide)> = payload.pages[1]
        .drawn_boxes
        .iter()
        .map(|b| (text_of(&b.beside), b.side))
        .collect();
    boxes.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        boxes,
        [
            ("No".to_string(), BoxSide::After),
            ("Sign".to_string(), BoxSide::After),
            ("Yes".to_string(), BoxSide::Before),
        ]
    );
    let md = ethos_parser_core::to_markdown(
        &repr,
        &profile.parser_version,
        &profile.profile_sha256().unwrap(),
        &profile.markdown_rule,
    )
    .expect("projects")
    .markdown;
    assert!(md.contains("\u{2610} Yes"), "{md}");
    assert!(md.contains("Sign \u{2610}"), "{md}");
    assert!(md.contains("No \u{2610}"), "{md}");
    assert_eq!(md.matches('\u{2610}').count(), 3, "{md}");
}

/// **On a real form, each box is read beside its own option** (decision #168): the W-9's federal
/// tax classifications, which its stream draws out of reading order.
#[test]
fn the_w9_s_classification_boxes_stand_before_their_options() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .join("fixtures/gate/irs-fw9.pdf");
    let bytes = std::fs::read(&path).expect("the gate fixture irs-fw9 is in the repository");
    let profile = Profile::default();
    let doc = Document::open_bytes(&bytes, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::into_representation(extract, &profile).expect("seals");
    let payload = repr.payload();
    let beside: Vec<(String, BoxSide)> = payload
        .pages
        .iter()
        .flat_map(|p| &p.drawn_boxes)
        .map(|b| {
            let run = payload.nodes.iter().find(|n| n.id == b.beside).unwrap();
            (run.text.trim().to_string(), b.side)
        })
        .collect();
    for option in [
        "Individual/sole proprietor",
        "C corporation",
        "S corporation",
        "Partnership",
        "Trust/estate",
    ] {
        assert!(
            beside
                .iter()
                .any(|(t, side)| t.starts_with(option) && *side == BoxSide::Before),
            "{option}: {beside:?}"
        );
    }
}

/// **A box names its run by the id that run takes in reading order** (decision #168): two columns
/// the stream draws right column first, a box before an option in the left one.
#[test]
fn a_box_follows_its_run_into_reading_order() {
    let mut stream = String::new();
    for (x, words) in [
        (
            170,
            [
                "Right one",
                "Right two",
                "Right three",
                "Right four",
                "Right five",
            ],
        ),
        (
            20,
            ["Left one", "Yes", "Left three", "Left four", "Left five"],
        ),
    ] {
        for (k, w) in words.iter().enumerate() {
            stream.push_str(&format!(
                "BT /F1 10 Tf 1 0 0 1 {x} {} Tm ({w}) Tj ET ",
                170 - 14 * k as i64
            ));
        }
    }
    // Before `Yes`, set at x 32 on the second line of the left column.
    stream.push_str("0.5 w 8 155 9 9 re S");
    let bytes = pages(&[stream.as_bytes()]);
    let profile = Profile::default();
    let doc = Document::open_bytes(&bytes, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::into_representation(extract, &profile).expect("seals");
    let payload = repr.payload();
    let order: Vec<&str> = payload.nodes.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(
        order[0], "Left one",
        "the left column reads first: {order:?}"
    );
    let boxes = &payload.pages[0].drawn_boxes;
    assert_eq!(boxes.len(), 1);
    let run = payload
        .nodes
        .iter()
        .find(|n| n.id == boxes[0].beside)
        .unwrap();
    assert_eq!((run.text.as_str(), boxes[0].side), ("Yes", BoxSide::Before));
}

/// One page drawing `stream`, with `widget` (a `/Rect`, its `/AS` where given, and whether it states
/// `/FT /Btn`) in its `/Annots`, and an image `/Im1` one grey pixel in its resources.
fn page_with(stream: &[u8], widget: Option<([i64; 4], Option<&str>, bool)>) -> Vec<u8> {
    let mut doc = lopdf::Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let image = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 1, "Height" => 1,
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
        },
        vec![0u8],
    ));
    let content = doc.add_object(Stream::new(dictionary! {}, stream.to_vec()));
    let mut annots: Vec<Object> = Vec::new();
    let mut fields: Vec<Object> = Vec::new();
    if let Some((r, shown, button)) = widget {
        let mut d = dictionary! {
            "Type" => "Annot", "Subtype" => "Widget",
            "T" => Object::string_literal("agree"),
            "Rect" => r.iter().map(|&v| Object::Integer(v)).collect::<Vec<_>>(),
            "P" => page_id,
        };
        if button {
            d.set("FT", Object::Name(b"Btn".to_vec()));
        }
        if let Some(s) = shown {
            d.set("AS", Object::Name(s.as_bytes().to_vec()));
        }
        let w = doc.add_object(d);
        annots.push(w.into());
        fields.push(w.into());
    }
    doc.objects.insert(
        page_id,
        Object::Dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 200.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! { "F1" => font },
                "XObject" => dictionary! { "Im1" => image },
            },
            "Contents" => content,
            "Annots" => annots,
        }),
    );
    doc.objects.insert(
        pages_id,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 },
        ),
    );
    let acroform = doc.add_object(dictionary! { "Fields" => fields });
    let catalog = doc.add_object(
        dictionary! { "Type" => "Catalog", "Pages" => pages_id, "AcroForm" => acroform },
    );
    doc.trailer.set("Root", catalog);
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

fn boxes_on(bytes: &[u8]) -> usize {
    let profile = Profile::default();
    let doc = Document::open_bytes(bytes, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::into_representation(extract, &profile).expect("seals");
    repr.payload()
        .pages
        .iter()
        .map(|p| p.drawn_boxes.len())
        .sum()
}

/// **A box a widget covers, or one an image is placed in, is none** (decision #168, review): the
/// widget says what the field holds — unless it says it is off, as the empty box does — and an
/// image tick marks its box as a drawn one does.
#[test]
fn a_box_under_a_widget_or_holding_an_image_is_none() {
    let drawn = b"BT /F1 10 Tf 1 0 0 1 52 150 Tm (Yes) Tj ET 0.5 w 40 149 9 9 re S";
    let rect = [40, 149, 49, 158];
    assert_eq!(boxes_on(&page_with(drawn, None)), 1, "the box alone");
    assert_eq!(
        boxes_on(&page_with(drawn, Some((rect, Some("Yes"), true)))),
        0,
        "a widget on"
    );
    assert_eq!(
        boxes_on(&page_with(drawn, Some((rect, None, true)))),
        0,
        "a widget saying nothing"
    );
    assert_eq!(
        boxes_on(&page_with(drawn, Some((rect, Some("Off"), true)))),
        1,
        "a widget off"
    );
    // Second review: a widget stating no button type leaves the box, and so does one only
    // brushing its edge.
    assert_eq!(
        boxes_on(&page_with(drawn, Some((rect, Some("Off"), false)))),
        1,
        "a widget of no stated type"
    );
    assert_eq!(
        boxes_on(&page_with(
            drawn,
            Some(([20, 157, 120, 175], Some("Yes"), true))
        )),
        1,
        "a widget brushing it"
    );
    // A run whose leading spaces cross the box takes it, its ink standing after the box.
    let spaced = b"BT /F1 10 Tf 1 0 0 1 30 150 Tm (        Yes) Tj ET 0.5 w 40 149 9 9 re S";
    assert_eq!(
        boxes_on(&page_with(spaced, None)),
        1,
        "leading spaces across it"
    );
    let ticked = b"BT /F1 10 Tf 1 0 0 1 52 150 Tm (Yes) Tj ET 0.5 w 40 149 9 9 re S \
                   q 7 0 0 7 41 150 cm /Im1 Do Q";
    assert_eq!(boxes_on(&page_with(ticked, None)), 0, "an image inside it");
}
