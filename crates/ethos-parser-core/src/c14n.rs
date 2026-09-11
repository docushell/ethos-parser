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

//! c14n v1 — the one canonical JSON serialization (`docs/01-CONTRACT.md` §4).
//!
//! A clean-room implementation of the same contract Ethos implements, so the two produce
//! byte-identical output for the same value. The parity vectors in this module's tests are
//! Ethos's own committed vectors, independently re-derived from the Python reference in
//! `ethos/docs/determinism-contract.md` §2.
//!
//! Note what that does and does not buy: these vectors catch drift **on this side**. Ethos
//! changing its implementation would not fail anything here — it would fail Ethos's own copy of
//! the same vectors, which is where it belongs. The M6 oracle test is what actually compares
//! the two systems at runtime.
//!
//! Properties, all tested below:
//!
//! - UTF-8, no whitespace between tokens
//! - object keys sorted by Unicode code point, **explicitly at write time**
//! - minimal escaping; no Unicode normalization
//! - integers only — any non-integer number is a hard error
//! - `|n| ≤ 2^53 − 1`
//! - idempotent: `c14n(parse(c14n(v))) == c14n(v)`
//!
//! No other module in this workspace hand-rolls output JSON. One serializer means one place
//! where byte identity can break.

use serde_json::Value;
use sha2::{Digest, Sha256};

pub use crate::geom::MAX_SAFE_INT;

/// A value could not be canonicalized.
///
/// Carries a deterministic message: the same offending value produces the same text, so a
/// failure is reproducible from the message alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C14nError {
    message: String,
}

impl C14nError {
    /// Construct with a deterministic message.
    ///
    /// Public so callers that canonicalize their own values (for example [`crate::Profile`])
    /// can report a serialization failure in the same currency, rather than unwrapping and
    /// turning a contract violation into a panic.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The deterministic failure message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl core::fmt::Display for C14nError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for C14nError {}

fn err(message: &str) -> C14nError {
    C14nError {
        message: message.to_string(),
    }
}

/// Serialize a JSON value to canonical bytes.
///
/// Object keys are sorted **explicitly here**, never by relying on the map's own iteration
/// order. That is not defensive style, it is a specific hazard: `serde_json`'s `preserve_order`
/// feature is additive, so any crate anywhere in the final dependency graph enabling it would
/// switch `Map` to insertion order and silently change every fingerprint this crate produces.
/// Sorting at write time makes the output correct under either map flavour, and
/// `tests/preserve_order.rs` builds the whole workspace with that feature forced on to prove it.
///
/// # Errors
///
/// Returns [`C14nError`] if the value contains a non-integer number, or an integer whose
/// magnitude exceeds [`MAX_SAFE_INT`].
pub fn c14n_bytes(value: &Value) -> Result<Vec<u8>, C14nError> {
    let mut out = Vec::with_capacity(256);
    write_value(value, &mut out)?;
    Ok(out)
}

fn write_value(value: &Value, out: &mut Vec<u8>) -> Result<(), C14nError> {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                // `unsigned_abs`, not `abs`: `i64::MIN.abs()` overflows — it panics in debug and
                // wraps in release, where the wrapped value would slip past a naive range check.
                if i.unsigned_abs() > MAX_SAFE_INT as u64 {
                    return Err(err("integer exceeds 2^53-1 in canonical value"));
                }
                out.extend_from_slice(i.to_string().as_bytes());
            } else if let Some(u) = n.as_u64() {
                if u > MAX_SAFE_INT as u64 {
                    return Err(err("integer exceeds 2^53-1 in canonical value"));
                }
                out.extend_from_slice(u.to_string().as_bytes());
            } else {
                return Err(err("non-integer number in canonical value"));
            }
        }
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(item, out)?;
            }
            out.push(b']');
        }
        Value::Object(map) => {
            out.push(b'{');
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            // String `Ord` is Unicode code point order — exactly the contract sort.
            entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
            for (i, (k, v)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_string(k, out);
                out.push(b':');
                write_value(v, out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

/// Minimal escaping. Non-ASCII is emitted literally and never normalized: extracted text is
/// evidence, and NFC-folding it would silently change bytes a citation may quote.
fn write_string(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{0008}' => out.extend_from_slice(b"\\b"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\u{000C}' => out.extend_from_slice(b"\\f"),
            '\r' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// Lowercase hex sha256 over the canonical bytes of `value`.
///
/// # Errors
///
/// Propagates [`C14nError`] from [`c14n_bytes`].
pub fn sha256_hex(value: &Value) -> Result<String, C14nError> {
    Ok(hex(&Sha256::digest(c14n_bytes(value)?)))
}

/// Join field values that are ALREADY canonical bytes into one canonical object.
///
/// The one caller that earns this exists because an artifact's payload is
/// canonicalized twice on the emit path — once inside `seal` for the fingerprint,
/// once inside `to_canonical_bytes` for the print — and on a 932 MB artifact the
/// second pass is most of the wall clock. `seal` keeps the bytes it hashed;
/// this splices them into the envelope without re-walking the payload. Keys are
/// sorted here and duplicates are refused, exactly as [`canonical_bytes_of`]
/// would have done, so the output is byte-identical to serializing the whole
/// struct — a property the caller's tests pin.
///
/// # Values are BORROWED, and that is the whole point of the signature
///
/// The payload is the largest thing in the artifact — 82% of it on a gate
/// document — and the caller holds it already, cached from `seal`. Taking
/// `Vec<u8>` by value made the one caller `clone()` that payload to hand it
/// over, so an emit path whose entire reason for existing is *not* walking the
/// payload twice was copying it instead. Borrowing removes a full-artifact
/// allocation from the peak with no change to a single emitted byte.
///
/// # Errors
///
/// [`C14nError`] on a duplicate key.
pub fn canonical_object(mut fields: Vec<(&str, &[u8])>) -> Result<Vec<u8>, C14nError> {
    fields.sort_by(|a, b| a.0.cmp(b.0));
    if let Some(pair) = fields.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(C14nError::new(format!(
            "duplicate key \"{}\" in canonical value",
            pair[0].0
        )));
    }
    let mut out = Vec::with_capacity(fields.iter().map(|(k, v)| k.len() + v.len() + 4).sum());
    out.push(b'{');
    for (i, (key, value)) in fields.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        write_string(key, &mut out);
        out.push(b':');
        out.extend_from_slice(value);
    }
    out.push(b'}');
    Ok(out)
}

/// Canonical bytes of any serializable value, streamed straight to sorted-key
/// output with no intermediate `serde_json::Value` tree.
///
/// This exists for the emit path. `c14n_bytes(&serde_json::to_value(v)?)` builds
/// the whole artifact as a `Value` DOM — a `BTreeMap` insert and a key `String`
/// per field, per node — before a single byte is written, which made
/// serialization the dominant cost of extraction on large documents. This
/// function produces **byte-identical** output (an equivalence property test
/// below feeds both paths the same values), including byte-identical refusal
/// messages for non-integer and out-of-range numbers: the contract is c14n v1
/// either way, this is only a cheaper route to it.
///
/// Objects still sort at write time, by the raw key string exactly as
/// [`c14n_bytes`] sorts, so the `preserve_order` hazard documented there cannot
/// reach this path either — entry order from the serialized type never survives.
///
/// # Errors
///
/// The refusals of [`c14n_bytes`] — non-integer numbers and integers whose
/// magnitude exceeds [`MAX_SAFE_INT`] — plus three cases where this route is
/// deliberately STRICTER than `to_value` would have been, all unreachable from
/// the workspace's serialized types today and each a fail-closed answer to a
/// hazard `to_value` papers over: a non-string map key (`to_value` stringifies
/// `1` into `"1"` — an invented key name), a NaN or infinity (`to_value`
/// silently emits `null` — a value the type never held), and a duplicate key
/// from a colliding `#[serde(flatten)]` (`to_value` keeps whichever came last —
/// a field silently dropped).
pub fn canonical_bytes_of<T: serde::Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, C14nError> {
    let mut out = Vec::with_capacity(256);
    value
        .serialize(CanonicalSerializer { out: &mut out })
        .map_err(|e| e.0)?;
    Ok(out)
}

/// [`C14nError`] wearing serde's error trait, so the serializer can travel
/// through `Serialize` impls.
#[derive(Debug)]
struct SerError(C14nError);

impl core::fmt::Display for SerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for SerError {}

impl serde::ser::Error for SerError {
    fn custom<T: core::fmt::Display>(msg: T) -> Self {
        SerError(C14nError::new(msg.to_string()))
    }
}

fn ser_err(message: &str) -> SerError {
    SerError(err(message))
}

fn write_checked_i64(i: i64, out: &mut Vec<u8>) -> Result<(), SerError> {
    if i.unsigned_abs() > MAX_SAFE_INT as u64 {
        return Err(ser_err("integer exceeds 2^53-1 in canonical value"));
    }
    out.extend_from_slice(i.to_string().as_bytes());
    Ok(())
}

fn write_checked_u64(u: u64, out: &mut Vec<u8>) -> Result<(), SerError> {
    if u > MAX_SAFE_INT as u64 {
        return Err(ser_err("integer exceeds 2^53-1 in canonical value"));
    }
    out.extend_from_slice(u.to_string().as_bytes());
    Ok(())
}

struct CanonicalSerializer<'a> {
    out: &'a mut Vec<u8>,
}

/// Streams sequence elements straight into the parent buffer — arrays keep their
/// order, so nothing needs staging.
struct CanonicalSeq<'a> {
    out: &'a mut Vec<u8>,
    first: bool,
}

/// Buffers `(raw key, value bytes)` pairs and writes them sorted on end — the
/// map/struct counterpart of [`c14n_bytes`]'s explicit write-time sort. The raw
/// key is kept for the sort because escaped bytes do not order like code points.
struct CanonicalMap<'a> {
    out: &'a mut Vec<u8>,
    entries: Vec<(String, Vec<u8>)>,
    pending_key: Option<String>,
    /// A variant wrapper (`{"variant":…}`) already opened in `out`, to close.
    close_variant: bool,
}

impl CanonicalMap<'_> {
    fn finish(self) -> Result<(), SerError> {
        let mut entries = self.entries;
        entries.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        // Refused, not collapsed: a `#[serde(flatten)]` whose inner and outer
        // fields collide reaches a streaming serializer as two entries under one
        // key. Emitting both would be invalid canonical JSON (idempotence breaks);
        // silently keeping one — the `to_value` route's last-wins behavior — would
        // drop a field no one decided to drop. No serialized type in this
        // workspace collides today, so the refusal costs nothing and a future
        // collision fails loudly at the first emit instead of shipping.
        if let Some(window) = entries.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(SerError(C14nError::new(format!(
                "duplicate key \"{}\" in canonical value",
                window[0].0
            ))));
        }
        // **At the top level, adopt the largest entry's buffer instead of copying it.** Sorting keys
        // means every field is serialized into its own staging buffer first, and the loop below then
        // copies each into `out`. For a representation payload the largest staging buffer is `nodes`
        // — 99.9% of the payload, 805 MiB on the largest gate document — and for the length of that
        // copy it existed twice, once in staging and once in `out`, inside `seal`, which is where
        // that document peaks. Nothing has been written to `out` exactly when this is the top level,
        // so the result can be built AROUND that buffer instead: shift its bytes right in place,
        // write the prefix into the gap, append the suffix. The bytes are the ones the loop would
        // have written, in the order it would have written them.
        //
        // Measured on this code, interleaved, five runs a side: peak RSS 4664.8 -> 3717.0 MiB
        // (-947.9) on `nist-sp-800-53Ar5`, peak footprint -910.9, output byte-identical, wall time
        // within 2%.
        // Reserving `out` at its final size was measured beside it and bought nothing: the regrowth
        // that prevents only ever added capacity nobody wrote. `docs/measurements/memory-ceiling/`
        // §11.
        if self.out.is_empty() {
            if let Some(big) = entries
                .iter()
                .enumerate()
                .max_by_key(|(_, (_, value))| value.len())
                .map(|(i, _)| i)
            {
                let mut prefix = vec![b'{'];
                for (i, (key, value)) in entries[..big].iter().enumerate() {
                    if i > 0 {
                        prefix.push(b',');
                    }
                    write_string(key, &mut prefix);
                    prefix.push(b':');
                    prefix.extend_from_slice(value);
                }
                if big > 0 {
                    prefix.push(b',');
                }
                write_string(&entries[big].0, &mut prefix);
                prefix.push(b':');
                let mut suffix = Vec::new();
                for (key, value) in &entries[big + 1..] {
                    suffix.push(b',');
                    write_string(key, &mut suffix);
                    suffix.push(b':');
                    suffix.extend_from_slice(value);
                }
                suffix.push(b'}');
                if self.close_variant {
                    suffix.push(b'}');
                }
                let mut adopted = std::mem::take(&mut entries[big].1);
                let (gap, len) = (prefix.len(), adopted.len());
                adopted.reserve(gap + suffix.len());
                adopted.resize(len + gap, 0);
                adopted.copy_within(0..len, gap);
                adopted[..gap].copy_from_slice(&prefix);
                adopted.extend_from_slice(&suffix);
                *self.out = adopted;
                return Ok(());
            }
        }
        self.out.push(b'{');
        for (i, (key, value)) in entries.into_iter().enumerate() {
            if i > 0 {
                self.out.push(b',');
            }
            write_string(&key, self.out);
            self.out.push(b':');
            self.out.extend_from_slice(&value);
        }
        self.out.push(b'}');
        if self.close_variant {
            self.out.push(b'}');
        }
        Ok(())
    }
}

/// Accepts strings (and the shapes that ARE strings: chars, unit variants,
/// `Display`-serialized newtypes) as object keys, and refuses everything else.
/// That is stricter than `serde_json`, which stringifies bool and integer keys —
/// `{1: …}` becoming `{"1": …}` is an invented key name, and inventing what goes
/// on the wire is what this module exists to prevent. No serialized type in the
/// workspace carries a non-string-keyed map, so the refusal is a tripwire, not a
/// behavior change.
struct KeySerializer;

impl serde::Serializer for KeySerializer {
    type Ok = String;
    type Error = SerError;
    type SerializeSeq = serde::ser::Impossible<String, SerError>;
    type SerializeTuple = serde::ser::Impossible<String, SerError>;
    type SerializeTupleStruct = serde::ser::Impossible<String, SerError>;
    type SerializeTupleVariant = serde::ser::Impossible<String, SerError>;
    type SerializeMap = serde::ser::Impossible<String, SerError>;
    type SerializeStruct = serde::ser::Impossible<String, SerError>;
    type SerializeStructVariant = serde::ser::Impossible<String, SerError>;

    fn serialize_str(self, v: &str) -> Result<String, SerError> {
        Ok(v.to_string())
    }
    fn serialize_char(self, v: char) -> Result<String, SerError> {
        Ok(v.to_string())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<String, SerError> {
        Ok(variant.to_string())
    }
    fn serialize_newtype_struct<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<String, SerError> {
        value.serialize(self)
    }
    fn collect_str<T: core::fmt::Display + ?Sized>(self, value: &T) -> Result<String, SerError> {
        Ok(value.to_string())
    }

    fn serialize_bool(self, _: bool) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_i8(self, _: i8) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_i16(self, _: i16) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_i32(self, _: i32) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_i64(self, _: i64) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_u8(self, _: u8) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_u16(self, _: u16) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_u32(self, _: u32) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_u64(self, _: u64) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_f32(self, _: f32) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_f64(self, _: f64) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_none(self) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_some<T: serde::Serialize + ?Sized>(self, _: &T) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_unit(self) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_newtype_variant<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<String, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, SerError> {
        Err(ser_err("map key must be a string in canonical value"))
    }
}

impl<'a> serde::Serializer for CanonicalSerializer<'a> {
    type Ok = ();
    type Error = SerError;
    type SerializeSeq = CanonicalSeq<'a>;
    type SerializeTuple = CanonicalSeq<'a>;
    type SerializeTupleStruct = CanonicalSeq<'a>;
    type SerializeTupleVariant = CanonicalSeq<'a>;
    type SerializeMap = CanonicalMap<'a>;
    type SerializeStruct = CanonicalMap<'a>;
    type SerializeStructVariant = CanonicalMap<'a>;

    fn serialize_bool(self, v: bool) -> Result<(), SerError> {
        self.out
            .extend_from_slice(if v { b"true" as &[u8] } else { b"false" });
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<(), SerError> {
        write_checked_i64(i64::from(v), self.out)
    }
    fn serialize_i16(self, v: i16) -> Result<(), SerError> {
        write_checked_i64(i64::from(v), self.out)
    }
    fn serialize_i32(self, v: i32) -> Result<(), SerError> {
        write_checked_i64(i64::from(v), self.out)
    }
    fn serialize_i64(self, v: i64) -> Result<(), SerError> {
        write_checked_i64(v, self.out)
    }
    fn serialize_i128(self, v: i128) -> Result<(), SerError> {
        i64::try_from(v)
            .map_err(|_| ser_err("integer exceeds 2^53-1 in canonical value"))
            .and_then(|v| write_checked_i64(v, self.out))
    }
    fn serialize_u8(self, v: u8) -> Result<(), SerError> {
        write_checked_u64(u64::from(v), self.out)
    }
    fn serialize_u16(self, v: u16) -> Result<(), SerError> {
        write_checked_u64(u64::from(v), self.out)
    }
    fn serialize_u32(self, v: u32) -> Result<(), SerError> {
        write_checked_u64(u64::from(v), self.out)
    }
    fn serialize_u64(self, v: u64) -> Result<(), SerError> {
        write_checked_u64(v, self.out)
    }
    fn serialize_u128(self, v: u128) -> Result<(), SerError> {
        u64::try_from(v)
            .map_err(|_| ser_err("integer exceeds 2^53-1 in canonical value"))
            .and_then(|v| write_checked_u64(v, self.out))
    }
    fn serialize_f32(self, _: f32) -> Result<(), SerError> {
        Err(ser_err("non-integer number in canonical value"))
    }
    fn serialize_f64(self, _: f64) -> Result<(), SerError> {
        Err(ser_err("non-integer number in canonical value"))
    }
    fn serialize_char(self, v: char) -> Result<(), SerError> {
        let mut buf = [0u8; 4];
        write_string(v.encode_utf8(&mut buf), self.out);
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<(), SerError> {
        write_string(v, self.out);
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<(), SerError> {
        // What `to_value` does with bytes: an array of integers.
        self.out.push(b'[');
        for (i, byte) in v.iter().enumerate() {
            if i > 0 {
                self.out.push(b',');
            }
            self.out.extend_from_slice(byte.to_string().as_bytes());
        }
        self.out.push(b']');
        Ok(())
    }
    fn serialize_none(self) -> Result<(), SerError> {
        self.out.extend_from_slice(b"null");
        Ok(())
    }
    fn serialize_some<T: serde::Serialize + ?Sized>(self, value: &T) -> Result<(), SerError> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), SerError> {
        self.out.extend_from_slice(b"null");
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), SerError> {
        self.out.extend_from_slice(b"null");
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), SerError> {
        write_string(variant, self.out);
        Ok(())
    }
    fn serialize_newtype_struct<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), SerError> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), SerError> {
        self.out.push(b'{');
        write_string(variant, self.out);
        self.out.push(b':');
        value.serialize(CanonicalSerializer { out: self.out })?;
        self.out.push(b'}');
        Ok(())
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, SerError> {
        self.out.push(b'[');
        Ok(CanonicalSeq {
            out: self.out,
            first: true,
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, SerError> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, SerError> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, SerError> {
        self.out.push(b'{');
        write_string(variant, self.out);
        self.out.extend_from_slice(b":[");
        Ok(CanonicalSeq {
            out: self.out,
            first: true,
        })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, SerError> {
        Ok(CanonicalMap {
            out: self.out,
            entries: Vec::new(),
            pending_key: None,
            close_variant: false,
        })
    }
    fn serialize_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, SerError> {
        Ok(CanonicalMap {
            out: self.out,
            entries: Vec::with_capacity(len),
            pending_key: None,
            close_variant: false,
        })
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, SerError> {
        self.out.push(b'{');
        write_string(variant, self.out);
        self.out.push(b':');
        Ok(CanonicalMap {
            out: self.out,
            entries: Vec::with_capacity(len),
            pending_key: None,
            close_variant: true,
        })
    }
    fn is_human_readable(&self) -> bool {
        // `serde_json` answers true, and types branch on this (hex vs raw bytes);
        // answering false here would change what they hand us.
        true
    }
}

impl serde::ser::SerializeSeq for CanonicalSeq<'_> {
    type Ok = ();
    type Error = SerError;
    fn serialize_element<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), SerError> {
        if !self.first {
            self.out.push(b',');
        }
        self.first = false;
        value.serialize(CanonicalSerializer { out: self.out })
    }
    fn end(self) -> Result<(), SerError> {
        self.out.push(b']');
        Ok(())
    }
}

impl serde::ser::SerializeTuple for CanonicalSeq<'_> {
    type Ok = ();
    type Error = SerError;
    fn serialize_element<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<(), SerError> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleStruct for CanonicalSeq<'_> {
    type Ok = ();
    type Error = SerError;
    fn serialize_field<T: serde::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<(), SerError> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleVariant for CanonicalSeq<'_> {
    type Ok = ();
    type Error = SerError;
    fn serialize_field<T: serde::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<(), SerError> {
        self.out.extend_from_slice(b"]}");
        Ok(())
    }
}

impl serde::ser::SerializeMap for CanonicalMap<'_> {
    type Ok = ();
    type Error = SerError;
    fn serialize_key<T: serde::Serialize + ?Sized>(&mut self, key: &T) -> Result<(), SerError> {
        self.pending_key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }
    fn serialize_value<T: serde::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), SerError> {
        let key = self
            .pending_key
            .take()
            .ok_or_else(|| ser_err("map value serialized before its key"))?;
        let mut bytes = Vec::new();
        value.serialize(CanonicalSerializer { out: &mut bytes })?;
        self.entries.push((key, bytes));
        Ok(())
    }
    fn end(self) -> Result<(), SerError> {
        self.finish()
    }
}

impl serde::ser::SerializeStruct for CanonicalMap<'_> {
    type Ok = ();
    type Error = SerError;
    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), SerError> {
        let mut bytes = Vec::new();
        value.serialize(CanonicalSerializer { out: &mut bytes })?;
        self.entries.push((key.to_string(), bytes));
        Ok(())
    }
    fn end(self) -> Result<(), SerError> {
        self.finish()
    }
}

impl serde::ser::SerializeStructVariant for CanonicalMap<'_> {
    type Ok = ();
    type Error = SerError;
    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), SerError> {
        serde::ser::SerializeStruct::serialize_field(self, key, value)
    }
    fn end(self) -> Result<(), SerError> {
        self.finish()
    }
}

/// Lowercase hex sha256 over raw bytes, for source-artifact identity.
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(digest: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    fn c14n_str(v: &Value) -> String {
        String::from_utf8(c14n_bytes(v).unwrap()).unwrap()
    }

    // --- parity vectors -------------------------------------------------------------------
    //
    // Lifted from Ethos's committed c14n tests (`ethos-core/src/c14n.rs`), which are themselves
    // cross-checked against a Python reference. Matching these hashes is the practical proof
    // that this clean-room implementation is byte-compatible with Ethos c14n v1 — without
    // taking a Cargo dependency on Ethos.

    #[test]
    fn parity_empty_object() {
        let v = json!({});
        assert_eq!(c14n_str(&v), "{}");
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }

    #[test]
    fn parity_key_order() {
        let v = json!({"b": 2, "a": 1, "_": 0, "Z": -3});
        assert_eq!(c14n_str(&v), r#"{"Z":-3,"_":0,"a":1,"b":2}"#);
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "9e8c5fa78b63297991b5b7b45bd334ccc61bd1058c5cd8ca6ee0451f78cd6cc1"
        );
    }

    #[test]
    fn parity_strings_and_ints() {
        let v = json!({
            "text": "líne1\nl\"ine2\tend — \u{1F4A1}",
            "n_zero": 0, "n_neg": -42, "arr": [3, 1, 2], "flag": true, "nothing": null
        });
        assert_eq!(
            c14n_str(&v),
            "{\"arr\":[3,1,2],\"flag\":true,\"n_neg\":-42,\"n_zero\":0,\"nothing\":null,\"text\":\"líne1\\nl\\\"ine2\\tend — \u{1F4A1}\"}"
        );
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "86b355efaa571cac1ddb71d422a9971e6042c55ec5369305cce095f2c181426e"
        );
    }

    #[test]
    fn parity_controls_and_backslash() {
        let v = json!({"bel": "\u{0007}", "backslash": "a\\b"});
        assert_eq!(
            c14n_str(&v),
            "{\"backslash\":\"a\\\\b\",\"bel\":\"\\u0007\"}"
        );
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "a1cc2b96cfaf4e1d27ca13e7c2e56faadf76bd027d233fce5a57124e36ea6dfd"
        );
    }

    #[test]
    fn parity_fingerprint_manifest() {
        let v = json!({
            "config_sha256": "68cc61753d299917cc7773f069c18aca31c8ac68f43736a94cb57eee05144084",
            "payload_sha256": "dad47d0ac4ab90f60691eb884c4c7e58d38ef7b87ef3df4bf602cd6087c9c757",
            "profile_id": "ethos-deterministic-v1",
            "profile_sha256": "d6145b9210845db39ad592ea549788432b52a649778c9947f5b2d91173e38070",
            "schema_version": "1.0.0",
            "source_fingerprint": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"
        });
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "b5d30710d0c25cc38d8dec924ecaf57ae4f81276dd5dc14d75cb3b5b6bde62d3"
        );
    }

    // --- float rejection ------------------------------------------------------------------

    #[test]
    fn floats_are_rejected_at_every_depth() {
        assert!(c14n_bytes(&json!(1.5)).is_err());
        assert!(c14n_bytes(&json!({"x": 1.5})).is_err());
        assert!(c14n_bytes(&json!([0.1])).is_err());
        assert!(c14n_bytes(&json!({"a": {"b": [1, 2, {"c": 0.25}]}})).is_err());
        assert!(c14n_bytes(&json!([[[[2.5]]]])).is_err());
        assert!(c14n_bytes(&json!({"ok": 1, "bad": [{"deep": -3.5}]})).is_err());
    }

    #[test]
    fn a_float_is_an_error_not_a_rounding() {
        let e = c14n_bytes(&json!({"x": 1.5})).unwrap_err();
        assert_eq!(e.message(), "non-integer number in canonical value");
        // The nearest integer must never appear in output as a silent repair.
        assert!(c14n_bytes(&json!({"x": 1.5})).is_err());
    }

    /// Float rejection must survive a change in how `serde_json` represents numbers.
    ///
    /// The rejection works because `Number::as_i64`/`as_u64` return `None` for anything stored as
    /// a float. That is an implementation detail of a dependency, so it is pinned from the
    /// *outside*: these are values whose JSON text is float-shaped even though the value is
    /// mathematically an integer. If a future `serde_json` normalised `1.0` to an integer, or
    /// `-0` to `0`, these assertions would start failing and the change would be visible instead
    /// of silently widening what counts as canonical.
    #[test]
    fn float_shaped_text_stays_rejected_regardless_of_number_representation() {
        for text in ["1.0", "-0", "-0.0", "0.0", "1e2", "1E2", "1.5e3", "2.0"] {
            let v: Value = serde_json::from_str(text)
                .unwrap_or_else(|e| panic!("{text} should parse as JSON: {e}"));
            assert!(
                c14n_bytes(&v).is_err(),
                "`{text}` parsed to {v} and was accepted by c14n; float-shaped text must be \
                 rejected however serde_json chooses to store it"
            );
        }

        // The integer spellings of the same values remain fine. `-0` is deliberately absent:
        // JSON text `-0` is float-shaped and is asserted rejected in the loop above.
        for text in ["1", "0", "2", "100", "-1", "-100"] {
            let v: Value = serde_json::from_str(text).unwrap();
            assert!(c14n_bytes(&v).is_ok(), "`{text}` must remain canonical");
        }
    }

    #[test]
    fn integers_at_the_2_53_boundary() {
        assert!(c14n_bytes(&json!(MAX_SAFE_INT)).is_ok());
        assert!(c14n_bytes(&json!(MAX_SAFE_INT + 1)).is_err());
        assert!(c14n_bytes(&json!(-MAX_SAFE_INT)).is_ok());
        assert!(c14n_bytes(&json!(-MAX_SAFE_INT - 1)).is_err());
    }

    #[test]
    fn i64_min_errors_rather_than_panicking() {
        // `i64::MIN.abs()` overflows; `unsigned_abs` must catch it cleanly.
        assert!(c14n_bytes(&json!(i64::MIN)).is_err());
        assert!(c14n_bytes(&json!({"n": i64::MIN})).is_err());
        assert!(c14n_bytes(&json!(u64::MAX)).is_err());
    }

    // --- escaping -------------------------------------------------------------------------

    #[test]
    fn control_characters_use_lowercase_four_digit_escapes() {
        assert_eq!(c14n_str(&json!("\u{0001}")), r#""\u0001""#);
        assert_eq!(c14n_str(&json!("\u{001f}")), r#""\u001f""#);
        assert_eq!(c14n_str(&json!("\u{000b}")), r#""\u000b""#);
        // The named short escapes take precedence where they exist.
        assert_eq!(c14n_str(&json!("\u{0008}")), r#""\b""#);
        assert_eq!(c14n_str(&json!("\t")), r#""\t""#);
        assert_eq!(c14n_str(&json!("\n")), r#""\n""#);
        assert_eq!(c14n_str(&json!("\u{000c}")), r#""\f""#);
        assert_eq!(c14n_str(&json!("\r")), r#""\r""#);
    }

    #[test]
    fn non_ascii_is_literal_and_never_normalized() {
        // Space (U+0020) is the first codepoint at or above the escape threshold, so it is the
        // boundary case that must come through literally.
        assert_eq!(c14n_str(&json!(" ")), r#"" ""#);
        assert_eq!(c14n_str(&json!("é")), "\"é\"");
        assert_eq!(c14n_str(&json!("日本語")), "\"日本語\"");
        assert_eq!(c14n_str(&json!("\u{1F4A1}")), "\"\u{1F4A1}\"");

        // Precomposed é (U+00E9) and decomposed e + combining acute (U+0065 U+0301) are
        // different byte sequences and must stay different. Normalizing would silently rewrite
        // evidence a citation may quote verbatim.
        let precomposed = c14n_str(&json!("\u{00E9}"));
        let decomposed = c14n_str(&json!("\u{0065}\u{0301}"));
        assert_ne!(precomposed, decomposed);
        assert_eq!(precomposed.len(), 4); // quote + 2 UTF-8 bytes + quote
        assert_eq!(decomposed.len(), 5); // quote + 1 + 2 UTF-8 bytes + quote
    }

    #[test]
    fn only_quote_and_backslash_are_escaped_above_the_control_range() {
        assert_eq!(c14n_str(&json!("\"")), r#""\"""#);
        assert_eq!(c14n_str(&json!("\\")), r#""\\""#);
        // Solidus is NOT escaped — a `\/` would be valid JSON but different bytes.
        assert_eq!(c14n_str(&json!("/")), r#""/""#);
    }

    #[test]
    fn keys_are_escaped_by_the_same_rule_as_values() {
        let v = json!({"a\nb": 1});
        assert_eq!(c14n_str(&v), "{\"a\\nb\":1}");
    }

    // --- structure ------------------------------------------------------------------------

    #[test]
    fn no_whitespace_anywhere() {
        let s = c14n_str(&json!({"a": [1, 2, {"b": "c"}], "d": null}));
        assert!(
            !s.contains(' '),
            "canonical output must not contain spaces: {s}"
        );
        assert!(!s.contains('\n'));
        assert_eq!(s, r#"{"a":[1,2,{"b":"c"}],"d":null}"#);
    }

    #[test]
    fn array_order_is_preserved_because_it_is_semantic() {
        // Element order IS reading order; sorting arrays would destroy meaning.
        assert_eq!(c14n_str(&json!([3, 1, 2])), "[3,1,2]");
    }

    #[test]
    fn key_order_is_by_code_point_not_ascii_case_or_locale() {
        // Uppercase sorts before lowercase; '_' (0x5F) sits between them.
        let v = json!({"a": 1, "A": 2, "_": 3, "b": 4, "B": 5});
        assert_eq!(c14n_str(&v), r#"{"A":2,"B":5,"_":3,"a":1,"b":4}"#);

        // Non-ASCII keys sort by code point, above all ASCII.
        let v = json!({"é": 1, "z": 2, "A": 3});
        assert_eq!(c14n_str(&v), "{\"A\":3,\"z\":2,\"é\":1}");
    }

    #[test]
    fn nested_objects_are_sorted_at_every_level() {
        let v = json!({"z": {"y": 1, "x": 2}, "a": {"c": 3, "b": 4}});
        assert_eq!(c14n_str(&v), r#"{"a":{"b":4,"c":3},"z":{"x":2,"y":1}}"#);
    }

    #[test]
    fn a_colliding_flatten_is_refused_not_collapsed() {
        // `to_value` would keep whichever field came last; the streaming route
        // refuses, because a silently dropped field is worse than a loud error.
        #[derive(serde::Serialize)]
        struct Inner {
            id: u32,
        }
        #[derive(serde::Serialize)]
        struct Colliding {
            id: u32,
            #[serde(flatten)]
            inner: Inner,
        }
        let err = canonical_bytes_of(&Colliding {
            id: 1,
            inner: Inner { id: 2 },
        })
        .unwrap_err();
        assert_eq!(err.message(), "duplicate key \"id\" in canonical value");

        // The non-colliding flatten shape both artifact writers use stays
        // byte-equal with the Value route.
        #[derive(serde::Serialize)]
        struct Fine {
            top: u32,
            #[serde(flatten)]
            inner: Inner,
        }
        let fine = Fine {
            top: 1,
            inner: Inner { id: 2 },
        };
        assert_eq!(
            canonical_bytes_of(&fine).unwrap(),
            c14n_bytes(&serde_json::to_value(&fine).unwrap()).unwrap(),
        );
    }

    /// **Adopting the largest entry writes the bytes the copy loop would have.** The top level is
    /// the only place `finish` adopts, so every position the largest value can sort into is
    /// covered — first, middle, last, alone — plus the empty object and a nested value, each against
    /// the `Value` route, which shares no code with `finish`.
    #[test]
    fn adopting_the_largest_entry_is_byte_identical_wherever_it_sorts() {
        let big = "y".repeat(10_000);
        for v in [
            json!({ "a": big, "m": "x", "z": [1, 2] }),
            json!({ "a": "x", "m": big, "z": [1, 2] }),
            json!({ "a": "x", "m": [1, 2], "z": big }),
            json!({ "only": big }),
            json!({}),
            json!({ "a": { "nested": big, "b": 1 }, "q\"uote": "k", "\u{e9}": 2 }),
        ] {
            assert_eq!(
                canonical_bytes_of(&v).expect("streaming"),
                c14n_bytes(&v).expect("value route"),
                "{v}"
            );
        }
    }

    /// A struct variant opens its `{"Variant":` wrapper in `out` before its fields are staged, so
    /// `out` is not empty and the adopt path must not run — the wrapper still has to close.
    #[test]
    fn a_top_level_struct_variant_still_closes_its_wrapper() {
        #[derive(serde::Serialize)]
        enum E {
            Variant { small: u8, large: String },
        }
        let v = E::Variant {
            small: 1,
            large: "z".repeat(10_000),
        };
        assert_eq!(
            canonical_bytes_of(&v).expect("streaming"),
            c14n_bytes(&serde_json::to_value(&v).expect("value")).expect("value route"),
        );
    }

    #[test]
    fn streaming_refusals_carry_the_value_route_messages_byte_for_byte() {
        // The refusal text is part of the contract's determinism: a consumer
        // diagnosing a failure must see one message whichever route produced it.
        let float = serde_json::json!({"x": 1.5});
        assert_eq!(
            canonical_bytes_of(&float).unwrap_err().message(),
            c14n_bytes(&float).unwrap_err().message(),
        );
        let too_big = serde_json::json!({"x": MAX_SAFE_INT + 1});
        assert_eq!(
            canonical_bytes_of(&too_big).unwrap_err().message(),
            c14n_bytes(&too_big).unwrap_err().message(),
        );
    }

    #[test]
    fn streaming_serializes_rust_shapes_the_value_route_agrees_on() {
        // Shapes that reach the serializer as Rust types rather than Values:
        // structs, options, enums in every variant form, nested maps.
        #[derive(serde::Serialize)]
        struct Probe {
            b: Option<u32>,
            a: Vec<&'static str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            skipped: Option<bool>,
            map: std::collections::BTreeMap<String, i64>,
        }
        let probe = Probe {
            b: None,
            a: vec!["z", "a"],
            skipped: None,
            map: [("k2".to_string(), 2), ("k1".to_string(), 1)].into(),
        };
        assert_eq!(
            canonical_bytes_of(&probe).unwrap(),
            c14n_bytes(&serde_json::to_value(&probe).unwrap()).unwrap(),
        );

        #[derive(serde::Serialize)]
        #[serde(rename_all = "snake_case")]
        enum Variants {
            Unit,
            Newtype(u8),
            Tuple(u8, u8),
            Struct { z: u8, a: u8 },
        }
        for variant in [
            Variants::Unit,
            Variants::Newtype(1),
            Variants::Tuple(1, 2),
            Variants::Struct { z: 1, a: 2 },
        ] {
            assert_eq!(
                canonical_bytes_of(&variant).unwrap(),
                c14n_bytes(&serde_json::to_value(&variant).unwrap()).unwrap(),
            );
        }
    }

    // --- property tests -------------------------------------------------------------------

    fn arb_canonical_value() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::from),
            (-MAX_SAFE_INT..=MAX_SAFE_INT).prop_map(Value::from),
            "\\PC*".prop_map(Value::from),
        ];
        leaf.prop_recursive(4, 32, 8, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
                proptest::collection::btree_map("\\PC*", inner, 0..6)
                    .prop_map(|m| Value::Object(m.into_iter().collect())),
            ]
        })
    }

    proptest! {
        /// The idempotence gate: `c14n(parse(c14n(v))) == c14n(v)`.
        #[test]
        fn c14n_is_idempotent(v in arb_canonical_value()) {
            let once = c14n_bytes(&v).unwrap();
            let reparsed: Value = serde_json::from_slice(&once).unwrap();
            let twice = c14n_bytes(&reparsed).unwrap();
            prop_assert_eq!(once, twice);
        }

        /// Canonical output is always valid JSON that parses back to an equal value.
        #[test]
        fn c14n_output_reparses_equal(v in arb_canonical_value()) {
            let bytes = c14n_bytes(&v).unwrap();
            let reparsed: Value = serde_json::from_slice(&bytes).unwrap();
            prop_assert_eq!(&reparsed, &v);
        }

        /// Canonical output is always valid UTF-8.
        #[test]
        fn c14n_output_is_utf8(v in arb_canonical_value()) {
            prop_assert!(String::from_utf8(c14n_bytes(&v).unwrap()).is_ok());
        }

        /// The streaming serializer is the same contract by a cheaper route: for any
        /// canonicalizable value, both paths produce identical bytes.
        #[test]
        fn canonical_bytes_of_matches_the_value_route(v in arb_canonical_value()) {
            prop_assert_eq!(canonical_bytes_of(&v).unwrap(), c14n_bytes(&v).unwrap());
        }

        /// Key insertion order never reaches the output.
        #[test]
        fn key_insertion_order_is_irrelevant(mut keys in proptest::collection::vec("[a-z]{1,4}", 1..8)) {
            keys.sort();
            keys.dedup();
            let forward: serde_json::Map<String, Value> =
                keys.iter().map(|k| (k.clone(), Value::from(1))).collect();
            let backward: serde_json::Map<String, Value> =
                keys.iter().rev().map(|k| (k.clone(), Value::from(1))).collect();
            prop_assert_eq!(
                c14n_bytes(&Value::Object(forward)).unwrap(),
                c14n_bytes(&Value::Object(backward)).unwrap()
            );
        }
    }
}
