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

//! **Optional content a document's default configuration hides** (decision #139,
//! `page-observations-v5`), and the graphics states that leave a fill's colour as it is.
//!
//! PDF 32000-1 §8.11: content inside `BDC /OC /name … EMC`, or drawn by a form XObject carrying
//! `/OC`, belongs to an optional-content group (`/OCG`), and the catalog's `/OCProperties /D` says
//! which groups a viewer shows on opening — every group but those `/OFF` names, or, where
//! `/BaseState` is `/OFF`, only those `/ON` names. That is all this module reads. `/AS` auto-state,
//! usage and intents are not read, and a membership dictionary (`/OCMD`) is not evaluated: its
//! content is answered [`Layer::NotEvaluated`] and counted, never called hidden or shown. A catalog
//! that declares no `/OCProperties` makes no content optional.

use std::collections::{BTreeMap, BTreeSet};

use crate::fonts::{resolve_array, resolve_dict};

/// What one `/OC` value tells a reader about the content it governs, where it does not show it.
///
/// Ordered so the stronger answer is the larger: content inside a group switched off is hidden,
/// whatever a membership dictionary around or within it would say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Layer {
    /// A membership dictionary, which this profile does not evaluate.
    NotEvaluated,
    /// A group the default configuration switches off.
    Off,
}

/// The default configuration's switches, read once per document from `/OCProperties /D`.
#[derive(Debug, Default)]
pub(crate) struct OptionalContent {
    /// Whether the catalog declares a default configuration at all.
    declared: bool,
    /// Whether its `/BaseState` is `/OFF`.
    base_off: bool,
    on: BTreeSet<lopdf::ObjectId>,
    off: BTreeSet<lopdf::ObjectId>,
}

impl OptionalContent {
    /// The default configuration `doc`'s catalog declares, or none.
    pub(crate) fn read(doc: &lopdf::Document) -> Self {
        let Some(d) = doc
            .catalog()
            .ok()
            .and_then(|c| resolve_dict(doc, c.get(b"OCProperties").ok()))
            .and_then(|p| resolve_dict(doc, p.get(b"D").ok()))
        else {
            return Self::default();
        };
        let groups = |key: &[u8]| -> BTreeSet<lopdf::ObjectId> {
            resolve_array(doc, d.get(key).ok())
                .unwrap_or_default()
                .iter()
                .filter_map(|o| o.as_reference().ok())
                .collect()
        };
        Self {
            declared: true,
            base_off: d.get(b"BaseState").and_then(lopdf::Object::as_name).ok() == Some(b"OFF"),
            on: groups(b"ON"),
            off: groups(b"OFF"),
        }
    }

    /// What `value` — a form's `/OC`, or the `/Properties` entry a `BDC /OC` names — is, or `None`
    /// where a viewer shows what it governs.
    pub(crate) fn layer(&self, doc: &lopdf::Document, value: &lopdf::Object) -> Option<Layer> {
        if !self.declared {
            return None;
        }
        let dict = resolve_dict(doc, Some(value))?;
        let kind = dict.get(b"Type").and_then(lopdf::Object::as_name).ok();
        if kind == Some(b"OCMD") || (kind != Some(b"OCG") && dict.has(b"OCGs")) {
            return Some(Layer::NotEvaluated);
        }
        // A group written inline is named by neither array, so it is in the base state.
        let off = match value.as_reference() {
            Ok(id) if self.base_off => !self.on.contains(&id),
            Ok(id) => self.off.contains(&id),
            Err(_) => self.base_off,
        };
        off.then_some(Layer::Off)
    }

    /// The names a resource dictionary's `/Properties` gives content a viewer does not show.
    pub(crate) fn names(
        &self,
        doc: &lopdf::Document,
        resources: Option<&lopdf::Dictionary>,
    ) -> BTreeMap<Vec<u8>, Layer> {
        let Some(properties) = resources.and_then(|r| resolve_dict(doc, r.get(b"Properties").ok()))
        else {
            return BTreeMap::new();
        };
        properties
            .iter()
            .filter_map(|(name, value)| Some((name.clone(), self.layer(doc, value)?)))
            .collect()
    }
}

/// The names a resource dictionary's `/ColorSpace` gives spaces whose components read as a device
/// colour's, each with how many it has (decision #156): `/CalGray` one, `/CalRGB` three, and
/// `/ICCBased` its `/N` — one, three or four read as gray, RGB or CMYK by their count, as the
/// recognizer reads an image in one (`docs/35-OCR-SCOPE.md` §4), and any other count as no colour
/// at all. A colour set in any other space, `/Lab`, `/Indexed`, `/Separation`, `/DeviceN` or a
/// pattern, is not known.
pub(crate) fn device_like_spaces(
    doc: &lopdf::Document,
    resources: Option<&lopdf::Dictionary>,
) -> BTreeMap<Vec<u8>, u8> {
    let Some(spaces) = resources.and_then(|r| resolve_dict(doc, r.get(b"ColorSpace").ok())) else {
        return BTreeMap::new();
    };
    let components = |o: &lopdf::Object| -> Option<u8> {
        let a = crate::fonts::resolve_array(doc, Some(o))?;
        match a.first().and_then(|f| f.as_name().ok())? {
            b"CalGray" => Some(1),
            b"CalRGB" => Some(3),
            b"ICCBased" => a
                .get(1)
                .and_then(|s| crate::fonts::resolve_stream(doc, Some(s)))
                .and_then(|s| crate::fonts::resolve_object(doc, s.dict.get(b"N").ok()))
                .and_then(|n| n.as_i64().ok())
                .and_then(|n| u8::try_from(n).ok()),
            _ => None,
        }
    };
    spaces
        .iter()
        .filter_map(|(name, v)| Some((name.clone(), components(v)?)))
        .collect()
}

/// The names a resource dictionary's `/ExtGState` gives graphics states that change no fill's
/// colour as painted (decision #139): no `/BM` but `/Normal` or `/Compatible`, no `/SMask` but
/// `/None`, and no `/ca` under one. Text drawn under any other `gs`, or one not named here, is not
/// compared with what lies beneath it: white under `/BM /Difference` is painted black.
pub(crate) fn plain_states(
    doc: &lopdf::Document,
    resources: Option<&lopdf::Dictionary>,
) -> BTreeSet<Vec<u8>> {
    let Some(states) = resources.and_then(|r| resolve_dict(doc, r.get(b"ExtGState").ok())) else {
        return BTreeSet::new();
    };
    let plain = |d: &lopdf::Dictionary| {
        let blend = match d.get(b"BM") {
            Err(_) => true,
            Ok(lopdf::Object::Name(n)) => matches!(n.as_slice(), b"Normal" | b"Compatible"),
            Ok(_) => false,
        };
        let mask = match d.get(b"SMask") {
            Err(_) => true,
            Ok(o) => o.as_name().ok() == Some(b"None"),
        };
        let alpha = d
            .get(b"ca")
            .map_or(true, |a| a.as_float().is_ok_and(|v| v >= 1.0));
        // Decision #156 (review): a transfer function maps what is painted to another colour.
        let transfer = [b"TR".as_slice(), b"TR2"].iter().all(|k| match d.get(k) {
            Err(_) => true,
            Ok(o) => matches!(o.as_name(), Ok(b"Identity" | b"Default")),
        });
        blend && mask && alpha && transfer
    };
    states
        .iter()
        .filter(|(_, v)| resolve_dict(doc, Some(v)).is_some_and(|d| plain(&d)))
        .map(|(name, _)| name.clone())
        .collect()
}
