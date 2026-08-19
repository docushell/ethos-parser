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

//! **Why there is no `liteparse → ethos.grounding.v1` adapter** (v1.2-S5).
//!
//! `docs/13-V12-MILESTONES.md` S5 asks for a foreign parser's output mapped into the grounding
//! shape — **"if it is worth it"** — under one standing constraint: *whatever it maps is not
//! `Extracted`*. S5 was implemented as far as the evidence allows and the answer is **no**, for
//! two reasons that are properties of `ethos.grounding.v1` itself rather than of any one document.
//!
//! This file is that refusal made mechanical. It asserts the three schema facts the refusal rests
//! on, so the day someone relaxes one of them the refusal is **reopened deliberately** rather than
//! quietly becoming false. It is the same idiom as `profile::tests::the_default_profile_is_pinned`:
//! a decision pinned to the artifact it was made about.
//!
//! # Wall 1 — the producer cannot be named, and the schema requires a name
//!
//! Checklist row **L20**, recorded in `docs/06-STEAL-REFUSE.md` and measured from
//! `output/json.rs:46-65`:
//!
//! > LiteParse … emits `page, width, height, text, text_items` **and nothing else**
//!
//! That is checklist **L20**, *"no versioned output contract"* — the omission this repository was
//! built to attack. So a LiteParse artifact **cannot say what produced it**, while
//! `ethos.grounding.v1` requires `producer: {name, version}` with a non-empty string in each.
//!
//! A caller-supplied version is not a way out. `additionalProperties: false` runs the length of
//! this schema, so there is nowhere to record that a field was *asserted* rather than *measured* —
//! a claimed version would be indistinguishable from one the engine read. This repository's
//! precedent is the opposite and explicit: `VerifierBinary::identify` pins a verifier by version
//! **and binary digest**, because an identity that can be asserted is an identity that can
//! disagree with what it describes.
//!
//! # Wall 2 — the boxes are loose, and the schema cannot say so
//!
//! Checklist **L18**, measured (§18.2 #5): LiteParse's bbox is a union of
//! `FPDFText_GetLooseCharBox` — em boxes, ascent-to-descent, **not ink**, *"for a line of `acme`
//! the box is as tall as if it contained `Ãj`"*. `docs/01-CONTRACT.md` §5.3 answers that directly:
//!
//! > When a box *is* emitted, the artifact says **what kind of box it is** … v0 emits measured ink
//! > boxes only … **If a future version emits loose boxes, it declares those separately.**
//!
//! `ethos.grounding.v1` has no field for that declaration and no room to add one. Loose boxes in
//! this schema would be *"loose char boxes sold as precise positioning, with nothing in the output
//! saying which it is"* — L18, the thing this repository refuses — reproduced inside its own
//! artifact and carrying its own `artifact_type`. That is worse than not shipping the adapter,
//! because the artifact would look like evidence.
//!
//! # What is NOT a wall, and is worth recording
//!
//! The memo predicted the blocker would be coordinates: an adapter *"would declare
//! `coordinate_origin: unknown` unless it also reads the source PDF"*. **That one dissolves.**
//! LiteParse's space is top-left, 72 DPI, `CropBox`→`MediaBox` (§18.2 #3), and this engine's
//! visible box is `/CropBox` clipped to the media box, or the media box where none is declared
//! (`engine-pdf/src/extract.rs`). Same box, same origin, and 72 DPI is one point per unit — so
//! points × 100 is centipoints exactly. An adapter reading the PDF for page geometry could have
//! declared `top-left` honestly.
//!
//! Floats are not a wall either (checklist L22): a value that will not land on an integer
//! centipoint is an omission with a count, which is the honesty `project()` already uses for a
//! node with no measurable ink box.
//!
//! So the refusal is narrow and it is about **provenance and box semantics**, not about geometry.
//! If LiteParse ever emits a versioned, self-identifying artifact and declares its box semantics,
//! the two walls fall and this decision is worth taking again — which is exactly why the schema
//! facts below are asserted rather than remembered.

use std::path::PathBuf;

use serde_json::Value;

fn schema() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/ethos-grounding-source.schema.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_slice(&bytes).expect("the pinned schema is JSON")
}

/// **Wall 1.** A producer that cannot name itself cannot be represented here.
///
/// Both fields are required and both must be non-empty, so there is no honest spelling of "the
/// thing that made this does not say what it is". Relax either and S5 is worth reopening — with a
/// deliberate argument about what an unattributed evidence artifact would mean.
#[test]
fn the_grounding_artifact_requires_a_producer_that_names_itself() {
    let schema = schema();
    let producer = &schema["$defs"]["producer"];

    let required: Vec<&str> = producer["required"]
        .as_array()
        .expect("producer declares required fields")
        .iter()
        .map(|v| v.as_str().expect("a field name"))
        .collect();
    assert_eq!(
        required,
        vec!["name", "version"],
        "a LiteParse artifact carries neither, and this is the field that would have to hold them"
    );

    for field in ["name", "version"] {
        assert_eq!(
            producer["properties"][field]["minLength"], 1,
            "`{field}` may not be empty, so there is no honest spelling of an unnamed producer"
        );
    }

    assert_eq!(
        producer["additionalProperties"],
        Value::Bool(false),
        "nowhere to record that a producer identity was asserted by a caller rather than measured"
    );
}

/// **Wall 2.** The artifact cannot say what kind of box it carries.
///
/// `01-CONTRACT.md` §5.3 requires an emitted box to declare its own semantics. This schema has no
/// property anywhere that could, which is why the ethos-engine path declares it through `producer`
/// and its profile instead — and why a foreign producer with *loose* boxes has nothing to declare
/// it with.
#[test]
fn the_grounding_artifact_has_no_way_to_declare_box_semantics() {
    let schema = schema();

    let mut names: Vec<String> = Vec::new();
    collect_property_names(&schema, &mut names);
    assert!(
        !names.is_empty(),
        "an empty walk would pass vacuously and prove nothing"
    );

    for name in &names {
        let lowered = name.to_ascii_lowercase();
        for token in [
            "ink",
            "loose",
            "em_box",
            "box_kind",
            "box_semantics",
            "glyph",
        ] {
            assert!(
                !lowered.contains(token),
                "`{name}` looks like a box-semantics declaration; if one has been added, S5's \
                 second wall has fallen and the LiteParse refusal is worth taking again"
            );
        }
    }

    // And no room to add one without moving the schema, which is the point: a field that would
    // make loose boxes honest here cannot arrive by accident.
    assert_eq!(schema["additionalProperties"], Value::Bool(false));
    for def in ["element", "span"] {
        assert_eq!(
            schema["$defs"][def]["additionalProperties"],
            Value::Bool(false),
            "`{def}` accepts extra fields, so a box could acquire undeclared semantics"
        );
    }
}

/// The coordinate system is a `const` pair, which is the half of the memo's prediction that held.
///
/// There is no `unknown` origin and this slice does not add one: widening it is a change to the
/// **verifier's** contract, not an adapter's business. Recorded here so the reason stays attached
/// to the fact.
#[test]
fn the_coordinate_system_admits_no_unknown_origin() {
    let schema = schema();
    let coords = &schema["$defs"]["coordinate_system"];
    assert_eq!(coords["properties"]["unit"]["const"], "centipoint");
    assert_eq!(coords["properties"]["origin"]["const"], "top-left");
    assert_eq!(coords["additionalProperties"], Value::Bool(false));
}

/// Every `properties` key anywhere in the schema.
fn collect_property_names(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(properties)) = map.get("properties") {
                out.extend(properties.keys().cloned());
            }
            for nested in map.values() {
                collect_property_names(nested, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_property_names(item, out);
            }
        }
        _ => {}
    }
}
