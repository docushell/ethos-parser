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

//! Form XObjects a page draws (`docs/30-FORM-XOBJECTS-SCOPE.md`).
//!
//! A `Do` naming a `/Form` runs a content stream the page draws, so the text it shows is page
//! text. A form is resolved the first time a page draws it — its operations, its `/Matrix` and its
//! own resources — and [`crate::content::Interpreter`] runs it where the `Do` stands. The
//! interpreter still holds no document: it asks a [`FormResolver`] by object id.
//!
//! What a page may cost is decision #34's, forms included: a form's operations come out of the
//! page's operation allowance when it is decoded, and again each time it is run, and its decoded
//! bytes are charged to the document's budget.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::Arc;

use ethos_parser_core::EngineError;

use crate::fonts::Font;
use crate::text_state::Matrix;

/// How deep forms may nest before a `Do` is not entered and counted (§7). The deepest in
/// ParseBench's 2,078 pages is 3.
pub(crate) const MAX_FORM_DEPTH: usize = 8;

/// One form XObject, ready to run.
#[derive(Debug)]
pub(crate) struct Form {
    /// Its content stream's operations.
    pub(crate) operations: Vec<lopdf::content::Operation>,
    /// Its `/Matrix`, the identity where it states none.
    pub(crate) matrix: Matrix,
    /// The fonts its own `/Resources` name, or the page's where it has none (§4).
    pub(crate) fonts: BTreeMap<String, Arc<Font>>,
    /// The XObjects its own `/Resources` name, or the page's where it has none.
    pub(crate) xobjects: BTreeMap<String, lopdf::ObjectId>,
}

/// What an XObject is to the interpreter.
pub(crate) enum Lookup {
    /// Not a form this profile runs: an image, a stream with no readable `/Subtype`, a form lost
    /// to its `/Length`, or one whose `/Matrix` is not six numbers. Placed and counted as before.
    NotRun,
    /// A form, ready to run.
    Form(Arc<Form>),
    /// A form whose operations pass what is left of the page's operation allowance.
    PastLimit,
}

/// Resolves an XObject's object id to what it is to the interpreter.
pub(crate) type FormResolver<'a> = dyn Fn(lopdf::ObjectId) -> Result<Lookup, EngineError> + 'a;

/// A form XObject's stream and matrix, or `None` when `id` is not a form this profile runs.
pub(crate) fn form_stream(
    doc: &lopdf::Document,
    id: lopdf::ObjectId,
) -> Option<(&lopdf::Stream, Matrix)> {
    let stream = doc.get_object(id).ok()?.as_stream().ok()?;
    if stream
        .dict
        .get(b"Subtype")
        .and_then(lopdf::Object::as_name)
        .ok()?
        != b"Form"
    {
        return None;
    }
    let matrix = match stream.dict.get(b"Matrix") {
        Err(_) => Matrix::IDENTITY,
        Ok(m) => {
            let items = m.as_array().ok().filter(|items| items.len() == 6)?;
            crate::content::matrix_operands(items).ok()?
        }
    };
    Some((stream, matrix))
}

/// A form's own `/Resources`, or the page's where it states none — PDF 32000-1 §7.8.3 permits
/// that and marks it obsolete.
pub(crate) fn form_resources(
    doc: &lopdf::Document,
    form: &lopdf::Dictionary,
    page_dict: &lopdf::Dictionary,
) -> Option<lopdf::Dictionary> {
    match form.get(b"Resources") {
        Ok(own) => crate::fonts::resolve_dict(doc, Some(own)),
        Err(_) => crate::extract::page_resources(doc, page_dict),
    }
}

/// The forms one page draws, each resolved the first time it is drawn.
pub(crate) struct PageForms<'d> {
    doc: &'d crate::document::Document,
    page_dict: &'d lopdf::Dictionary,
    page_number: u32,
    budget: &'d crate::budget::ContentBudget,
    /// What is left of the page's operation allowance for decoding forms.
    ops_left: Cell<usize>,
    cache: RefCell<BTreeMap<lopdf::ObjectId, Option<Arc<Form>>>>,
}

impl<'d> PageForms<'d> {
    /// The forms of the page `page_dict`, decoded within `ops_left` operations between them.
    pub(crate) fn new(
        doc: &'d crate::document::Document,
        page_dict: &'d lopdf::Dictionary,
        page_number: u32,
        budget: &'d crate::budget::ContentBudget,
        ops_left: usize,
    ) -> Self {
        Self {
            doc,
            page_dict,
            page_number,
            budget,
            ops_left: Cell::new(ops_left),
            cache: RefCell::new(BTreeMap::new()),
        }
    }

    /// What `id` is to the interpreter.
    ///
    /// # Errors
    ///
    /// What the page's own stream would be refused for — a filter that does not decode, an
    /// operator outside Table A.1 — and a font of the form's that does not load.
    pub(crate) fn resolve(&self, id: lopdf::ObjectId) -> Result<Lookup, EngineError> {
        if let Some(hit) = self.cache.borrow().get(&id) {
            return Ok(hit.clone().map_or(Lookup::NotRun, Lookup::Form));
        }
        let doc = self.doc.inner();
        let Some((stream, matrix)) = form_stream(doc, id) else {
            self.cache.borrow_mut().insert(id, None);
            return Ok(Lookup::NotRun);
        };
        let label = format!("page {}, form {} {} R", self.page_number, id.0, id.1);
        let Some((operations, bytes)) =
            crate::extract::content_operations(doc, &label, vec![id], self.ops_left.get())?
        else {
            return Ok(Lookup::PastLimit);
        };
        self.ops_left.set(self.ops_left.get() - operations.len());
        self.budget.charge(bytes, 0)?;
        let resources = form_resources(doc, &stream.dict, self.page_dict);
        let (fonts, xobjects) = match &resources {
            Some(r) => (
                crate::fonts::load_fonts(self.doc, r)?,
                crate::images::resource_xobjects(doc, r),
            ),
            None => (BTreeMap::new(), BTreeMap::new()),
        };
        let form = Arc::new(Form {
            operations,
            matrix,
            fonts,
            xobjects,
        });
        self.cache.borrow_mut().insert(id, Some(Arc::clone(&form)));
        Ok(Lookup::Form(form))
    }

    /// The forms this page resolved, which are the forms it drew.
    pub(crate) fn drawn(&self) -> Vec<Arc<Form>> {
        self.cache.borrow().values().flatten().cloned().collect()
    }
}
