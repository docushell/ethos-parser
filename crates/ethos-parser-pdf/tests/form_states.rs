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

//! Decision #162 (`form-annotations-v3`): a widget's label and a check box's or radio button's
//! state, each read from what the widget itself states, on made-up forms.

use ethos_parser_core::{
    codes, CheckState, CheckStateAbsence, FieldValue, FormFieldAttributes, Profile,
};
use ethos_parser_pdf::{Document, ExtractArtifact};
use lopdf::{dictionary, Object, Stream};

/// One page drawing `Agree`, with the widgets `build` adds (given the page's id) in its `/Annots`
/// and their top fields in the form's `/Fields`.
fn form(
    build: impl FnOnce(&mut lopdf::Document, lopdf::ObjectId) -> Vec<lopdf::ObjectId>,
) -> Vec<u8> {
    let mut doc = lopdf::Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 1 0 0 1 60 120 Tm (Agree) Tj ET".to_vec(),
    ));
    let widgets = build(&mut doc, page_id);
    let mut fields: Vec<Object> = Vec::new();
    for w in &widgets {
        let top = match doc
            .get_dictionary(*w)
            .ok()
            .and_then(|d| d.get(b"Parent").ok())
        {
            Some(Object::Reference(p)) => *p,
            _ => *w,
        };
        if !fields.contains(&Object::Reference(top)) {
            fields.push(Object::Reference(top));
        }
    }
    doc.objects.insert(
        page_id,
        Object::Dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 144.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
            "Contents" => content,
            "Annots" => widgets.iter().map(|w| Object::Reference(*w)).collect::<Vec<_>>(),
        }),
    );
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1,
        }),
    );
    let acroform = doc.add_object(dictionary! { "Fields" => fields });
    let catalog = doc.add_object(dictionary! {
        "Type" => "Catalog", "Pages" => pages_id, "AcroForm" => acroform,
    });
    doc.trailer.set("Root", catalog);
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

/// A widget appearance stream: what an `/AP /N` state draws.
fn appearance(doc: &mut lopdf::Document) -> Object {
    doc.add_object(Stream::new(
        dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()] },
        b"0 0 10 10 re f".to_vec(),
    ))
    .into()
}

/// A check box widget: `/AS` as given (`None` for none), and `/AP /N` declaring `states`.
fn check_box(
    doc: &mut lopdf::Document,
    page: lopdf::ObjectId,
    name: &str,
    shown: Option<&str>,
    states: &[&str],
) -> lopdf::ObjectId {
    let mut normal = lopdf::Dictionary::new();
    for s in states {
        normal.set(s.as_bytes().to_vec(), appearance(doc));
    }
    let mut d = dictionary! {
        "Type" => "Annot", "Subtype" => "Widget", "FT" => "Btn",
        "T" => Object::string_literal(name),
        "Rect" => vec![40.into(), 118.into(), 52.into(), 130.into()],
        "P" => page,
        "AP" => dictionary! { "N" => normal },
    };
    if let Some(s) = shown {
        d.set("AS", Object::Name(s.as_bytes().to_vec()));
    }
    doc.add_object(d)
}

fn extract(bytes: &[u8]) -> ExtractArtifact {
    let profile = Profile::default();
    let doc = Document::open_bytes(bytes, &profile).expect("opens");
    ethos_parser_pdf::extract(&doc, &profile).expect("extracts")
}

fn fields(a: &ExtractArtifact) -> Vec<FormFieldAttributes> {
    a.pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .filter_map(|o| match &o.attributes {
            ethos_parser_core::NodeAttributes::FormField(f) => Some(f.clone()),
            _ => None,
        })
        .collect()
}

fn not_determinable(a: &ExtractArtifact) -> Option<String> {
    a.assurance
        .limitations
        .iter()
        .find(|l| l.code == codes::FORM_FIELD_STATE_NOT_DETERMINABLE)
        .map(|l| l.detail.clone())
}

#[test]
fn a_check_box_showing_a_declared_on_state_is_checked_by_that_state() {
    let a = extract(&form(|doc, page| {
        vec![check_box(doc, page, "agree", Some("Yes"), &["Yes", "Off"])]
    }));
    assert_eq!(fields(&a)[0].state, Some(CheckState::Checked("Yes".into())));
    assert_eq!(not_determinable(&a), None);
    // The on state is the document's own name, whatever it is.
    let a = extract(&form(|doc, page| {
        vec![check_box(doc, page, "agree", Some("Oui"), &["Oui", "Off"])]
    }));
    assert_eq!(fields(&a)[0].state, Some(CheckState::Checked("Oui".into())));
}

#[test]
fn a_check_box_showing_off_is_unchecked_whatever_its_appearance_declares() {
    for states in [&["Yes", "Off"][..], &["Yes"][..], &[][..]] {
        let a = extract(&form(|doc, page| {
            vec![check_box(doc, page, "agree", Some("Off"), states)]
        }));
        assert_eq!(
            fields(&a)[0].state,
            Some(CheckState::Unchecked),
            "{states:?}"
        );
        assert_eq!(not_determinable(&a), None);
    }
}

#[test]
fn a_check_box_naming_no_state_or_an_undeclared_one_is_not_determinable_and_counted() {
    let a = extract(&form(|doc, page| {
        vec![
            check_box(doc, page, "a", None, &["Yes", "Off"]),
            check_box(doc, page, "b", Some("Yes"), &["On", "Off"]),
            check_box(doc, page, "c", Some("Yes"), &[]),
        ]
    }));
    let states: Vec<_> = fields(&a).into_iter().map(|f| f.state).collect();
    assert_eq!(
        states,
        [
            Some(CheckState::NotDeterminable(
                CheckStateAbsence::NoAppearanceState
            )),
            Some(CheckState::NotDeterminable(
                CheckStateAbsence::UndeclaredAppearanceState
            )),
            Some(CheckState::NotDeterminable(
                CheckStateAbsence::UndeclaredAppearanceState
            )),
        ]
    );
    let detail = not_determinable(&a).expect("counted");
    assert!(
        detail.starts_with("3 check box or radio button widget(s)"),
        "{detail}"
    );
}

#[test]
fn a_radio_group_states_which_widget_is_on_and_the_shared_value_decides_nothing() {
    let a = extract(&form(|doc, page| {
        let group = doc.new_object_id();
        let kid = |doc: &mut lopdf::Document, on: &str, shown: &str| {
            let n = dictionary! { on => appearance(doc), "Off" => appearance(doc) };
            doc.add_object(dictionary! {
                "Type" => "Annot", "Subtype" => "Widget", "Parent" => group,
                "Rect" => vec![40.into(), 100.into(), 52.into(), 112.into()], "P" => page,
                "AS" => Object::Name(shown.as_bytes().to_vec()), "AP" => dictionary! { "N" => n },
            })
        };
        let kids = vec![kid(doc, "Small", "Off"), kid(doc, "Large", "Large")];
        doc.objects.insert(
            group,
            Object::Dictionary(dictionary! {
                "FT" => "Btn", "Ff" => 1 << 15, "T" => Object::string_literal("size"),
                "TU" => Object::string_literal("Cup size"),
                // The group's value says Small; each widget says what it shows.
                "V" => "Small",
                "Kids" => kids.iter().map(|k| Object::Reference(*k)).collect::<Vec<_>>(),
            }),
        );
        kids
    }));
    let f = fields(&a);
    assert_eq!(f[0].state, Some(CheckState::Unchecked));
    assert_eq!(f[1].state, Some(CheckState::Checked("Large".into())));
    assert_eq!(
        f[0].value,
        FieldValue::Name("Small".into()),
        "the value is kept as written"
    );
    assert_eq!(
        f[0].label.as_deref(),
        Some("Cup size"),
        "the kid's field is the group"
    );
    assert!(f.iter().all(|f| f.flags.contains(&"radio".to_string())));
}

#[test]
fn a_push_button_and_a_text_field_carry_no_state() {
    let a = extract(&form(|doc, page| {
        let push = doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "FT" => "Btn", "Ff" => 1 << 16,
            "T" => Object::string_literal("go"), "AS" => "Off", "P" => page,
            "Rect" => vec![40.into(), 80.into(), 90.into(), 95.into()],
        });
        let text = doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "FT" => "Tx", "AS" => "Off", "P" => page,
            "T" => Object::string_literal("name"), "TU" => Object::string_literal("Your name"),
            "Rect" => vec![40.into(), 60.into(), 200.into(), 75.into()],
        });
        // No `/FT` anywhere: not called a button, so no state is claimed.
        let untyped = doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "AS" => "Off", "P" => page,
            "T" => Object::string_literal("what"),
            "Rect" => vec![40.into(), 40.into(), 52.into(), 52.into()],
        });
        vec![push, text, untyped]
    }));
    let f = fields(&a);
    assert!(f.iter().all(|f| f.state.is_none()), "{f:?}");
    assert_eq!(f[1].label.as_deref(), Some("Your name"));
    assert_eq!(not_determinable(&a), None);
}

#[test]
fn a_label_is_read_from_the_widgets_own_field_never_from_a_group_above_it() {
    let a = extract(&form(|doc, page| {
        let section = doc.new_object_id();
        let field = doc.new_object_id();
        let widget = doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "Parent" => field, "P" => page,
            "Rect" => vec![40.into(), 60.into(), 200.into(), 75.into()],
        });
        // The field states no `/TU`; the section above it does, for the whole section.
        doc.objects.insert(
            field,
            Object::Dictionary(dictionary! {
                "FT" => "Tx", "T" => Object::string_literal("city"), "Parent" => section,
                "Kids" => vec![widget.into()],
            }),
        );
        doc.objects.insert(
            section,
            Object::Dictionary(dictionary! {
                "T" => Object::string_literal("address"),
                "TU" => Object::string_literal("Home address"),
                "Kids" => vec![field.into()],
            }),
        );
        vec![widget]
    }));
    let f = &fields(&a)[0];
    assert_eq!(f.field_name.as_deref(), Some("address.city"));
    assert_eq!(f.label, None, "a group's label is not its field's");
}

/// QA round 1 on decision #162: the reachable shapes the first tests missed.
#[test]
fn indirect_states_and_appearances_are_followed() {
    let a = extract(&form(|doc, page| {
        let shown = doc.add_object(Object::Name(b"Yes".to_vec()));
        let drawn = appearance(doc);
        let normal = doc.add_object(dictionary! { "Yes" => drawn });
        let ap = doc.add_object(dictionary! { "N" => normal });
        vec![doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "FT" => "Btn", "P" => page,
            "T" => Object::string_literal("agree"), "AS" => shown, "AP" => ap,
            "Rect" => vec![40.into(), 118.into(), 52.into(), 130.into()],
        })]
    }));
    assert_eq!(fields(&a)[0].state, Some(CheckState::Checked("Yes".into())));
}

#[test]
fn a_state_the_normal_appearance_does_not_draw_is_not_determinable() {
    let undeclared = Some(CheckState::NotDeterminable(
        CheckStateAbsence::UndeclaredAppearanceState,
    ));
    // `/N` one stream, with no states; `/AP` with a down appearance only; a null entry.
    let a = extract(&form(|doc, page| {
        let stream = appearance(doc);
        let down = appearance(doc);
        let widget = |doc: &mut lopdf::Document, name: &str, ap: lopdf::Dictionary| {
            doc.add_object(dictionary! {
                "Type" => "Annot", "Subtype" => "Widget", "FT" => "Btn", "P" => page,
                "T" => Object::string_literal(name), "AS" => "Yes", "AP" => ap,
                "Rect" => vec![40.into(), 118.into(), 52.into(), 130.into()],
            })
        };
        vec![
            widget(doc, "a", dictionary! { "N" => stream }),
            widget(
                doc,
                "b",
                dictionary! { "D" => dictionary! { "Yes" => down } },
            ),
            widget(
                doc,
                "c",
                dictionary! { "N" => dictionary! { "Yes" => Object::Null } },
            ),
        ]
    }));
    for f in fields(&a) {
        assert_eq!(f.state, undeclared, "{f:?}");
    }
    assert!(not_determinable(&a).expect("counted").starts_with("3 "));
}

#[test]
fn a_state_whose_name_is_not_utf8_is_not_determinable() {
    let a = extract(&form(|doc, page| {
        let drawn = appearance(doc);
        let mut normal = lopdf::Dictionary::new();
        normal.set(b"Fran\xe7ais".to_vec(), drawn);
        vec![doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "FT" => "Btn", "P" => page,
            "T" => Object::string_literal("lang"), "AS" => Object::Name(b"Fran\xe7ais".to_vec()),
            "AP" => dictionary! { "N" => normal },
            "Rect" => vec![40.into(), 118.into(), 52.into(), 130.into()],
        })]
    }));
    assert_eq!(
        fields(&a)[0].state,
        Some(CheckState::NotDeterminable(
            CheckStateAbsence::UndecodableAppearanceState
        ))
    );
}

#[test]
fn the_nearest_ff_decides_a_push_button_an_explicit_zero_included() {
    let read = |kid_flags: Option<i64>| {
        let a = extract(&form(|doc, page| {
            let parent = doc.new_object_id();
            let drawn = appearance(doc);
            let mut kid = dictionary! {
                "Type" => "Annot", "Subtype" => "Widget", "Parent" => parent, "P" => page,
                "T" => Object::string_literal("b"), "AS" => "Yes",
                "AP" => dictionary! { "N" => dictionary! { "Yes" => drawn } },
                "Rect" => vec![40.into(), 118.into(), 52.into(), 130.into()],
            };
            if let Some(f) = kid_flags {
                kid.set("Ff", f);
            }
            let kid = doc.add_object(kid);
            doc.objects.insert(
                parent,
                Object::Dictionary(dictionary! {
                    "FT" => "Btn", "Ff" => 1 << 16, "T" => Object::string_literal("a"),
                    "Kids" => vec![kid.into()],
                }),
            );
            vec![kid]
        }));
        fields(&a)[0].state.clone()
    };
    assert_eq!(read(None), None, "an inherited push button has no state");
    assert_eq!(
        read(Some(0)),
        Some(CheckState::Checked("Yes".into())),
        "the kid's own 0 wins"
    );
}

#[test]
fn a_nameless_field_and_a_merged_widget_give_their_own_label() {
    let a = extract(&form(|doc, page| {
        let section = doc.new_object_id();
        let field = doc.new_object_id();
        let widget = doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "Parent" => field, "P" => page,
            "Rect" => vec![40.into(), 60.into(), 200.into(), 75.into()],
        });
        // A field with no `/T` of its own, under a named section.
        doc.objects.insert(
            field,
            Object::Dictionary(dictionary! {
                "FT" => "Tx", "TU" => Object::string_literal("City"), "Parent" => section,
                "Kids" => vec![widget.into()],
            }),
        );
        doc.objects.insert(
            section,
            Object::Dictionary(dictionary! {
                "T" => Object::string_literal("address"),
                "TU" => Object::string_literal("Home address"),
                "Kids" => vec![field.into()],
            }),
        );
        // A merged field and widget with a `/TU` and no `/T`.
        let merged = doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "FT" => "Btn", "P" => page,
            "TU" => Object::string_literal("I agree"), "AS" => "Off",
            "Rect" => vec![40.into(), 118.into(), 52.into(), 130.into()],
        });
        vec![widget, merged]
    }));
    let f = fields(&a);
    assert_eq!(f[0].label.as_deref(), Some("City"));
    assert_eq!(f[1].label.as_deref(), Some("I agree"));
}

#[test]
fn a_label_in_bytes_that_do_not_decode_is_absent_and_counted() {
    let a = extract(&form(|doc, page| {
        vec![doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "FT" => "Tx", "P" => page,
            "T" => Object::string_literal("name"),
            // PDFDocEncoding 0x7F: a code the decoders part over.
            "TU" => Object::String(vec![b'A', 0x7F], lopdf::StringFormat::Literal),
            "Rect" => vec![40.into(), 60.into(), 200.into(), 75.into()],
        })]
    }));
    assert_eq!(fields(&a)[0].label, None);
    assert!(a
        .assurance
        .limitations
        .iter()
        .any(|l| l.code == codes::FORM_ANNOTATION_TEXT_UNDECODABLE));
}

#[test]
fn each_state_has_its_wire_shape() {
    let json = |s: &CheckState| serde_json::to_string(s).unwrap();
    assert_eq!(
        json(&CheckState::Checked("Yes".into())),
        r#"{"kind":"checked","value":"Yes"}"#
    );
    assert_eq!(json(&CheckState::Unchecked), r#"{"kind":"unchecked"}"#);
    assert_eq!(
        json(&CheckState::NotDeterminable(
            CheckStateAbsence::NoAppearanceState
        )),
        r#"{"kind":"not_determinable","value":"no_appearance_state"}"#
    );
}

/// A profile that reads no form fields counts no field's state: it emits none to carry one.
#[test]
fn a_profile_without_form_fields_counts_no_state() {
    let bytes = form(|doc, page| vec![check_box(doc, page, "a", None, &["Yes", "Off"])]);
    let mut profile = Profile::default();
    profile.capabilities.form_fields = false;
    let doc = Document::open_bytes(&bytes, &profile).expect("opens");
    let a = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    assert!(fields(&a).is_empty());
    assert_eq!(not_determinable(&a), None);
}

/// Decision #164: a flat form's typed values, drawn last in fonts of their own, each name the
/// caption they were read beside — on every page, by the node ids that page's runs end up with.
#[test]
fn a_fillers_value_names_its_caption_by_node_id_on_every_page() {
    let mut doc = lopdf::Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut fonts = lopdf::Dictionary::new();
    for (name, base) in [
        ("F1", "Helvetica"),
        ("F2", "Courier"),
        ("F3", "Times-Roman"),
        ("F4", "Helvetica-Bold"),
    ] {
        let f = doc
            .add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => base });
        fonts.set(name, f);
    }
    // Irregular, as a real form is: a neat grid of captions and values would be read as a table,
    // whose cells #157 leaves where they are.
    let content = b"BT /F1 10 Tf 1 0 0 1 50 740 Tm (Please complete every part of this form.) Tj ET \
BT /F1 10 Tf 1 0 0 1 50 700 Tm (Name:) Tj ET BT /F1 10 Tf 1 0 0 1 320 700 Tm (Date:) Tj ET \
BT /F1 10 Tf 1 0 0 1 50 670 Tm (Town of birth:) Tj ET \
BT /F1 10 Tf 1 0 0 1 50 640 Tm (Telephone number:) Tj ET \
BT /F2 10 Tf 1 0 0 1 90 700 Tm (Ada Lovelace) Tj ET BT /F3 10 Tf 1 0 0 1 355 700 Tm (10 Dec 1815) Tj ET \
BT /F4 10 Tf 1 0 0 1 125 670 Tm (London) Tj ET BT /F2 10 Tf 1 0 0 1 150 639 Tm (555 0100) Tj ET"
        .to_vec();
    let mut kids = Vec::new();
    for _ in 0..2 {
        let c = doc.add_object(Stream::new(dictionary! {}, content.clone()));
        kids.push(Object::Reference(doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => c,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "Font" => fonts.clone() },
        })));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => 2 }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("saves");

    let profile = Profile::default();
    let opened = Document::open_bytes(&bytes, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&opened, &profile).expect("extracts");
    let repr = ethos_parser_pdf::into_representation(extract, &profile).expect("seals");
    let nodes = &repr.payload().nodes;
    for page in &repr.payload().pages {
        let on_page: Vec<_> = nodes.iter().filter(|n| n.parent == page.id).collect();
        let id_of = |text: &str| {
            on_page
                .iter()
                .find(|n| n.text == text)
                .map(|n| n.id.clone())
                .expect("drawn")
        };
        let beside = |text: &str| match &on_page.iter().find(|n| n.text == text).unwrap().attributes
        {
            ethos_parser_core::NodeAttributes::TextRun(a) => a.read_beside.clone(),
            _ => None,
        };
        assert_eq!(
            beside("Ada Lovelace"),
            Some(id_of("Name:")),
            "page {}",
            page.index
        );
        assert_eq!(
            beside("10 Dec 1815"),
            Some(id_of("Date:")),
            "page {}",
            page.index
        );
        assert_eq!(
            beside("London"),
            Some(id_of("Town of birth:")),
            "page {}",
            page.index
        );
        assert_eq!(
            beside("555 0100"),
            Some(id_of("Telephone number:")),
            "page {}",
            page.index
        );
        assert_eq!(beside("Name:"), None, "a caption is read beside nothing");
    }
}
