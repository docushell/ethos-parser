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

//! Read one entry out of an OOXML package. **Not a ZIP library.**
//!
//! # Why this is here rather than a dependency
//!
//! The `zip` crate pulls twelve transitive crates — `zopfli` (a *compressor*), `indexmap`,
//! `typed-path`, `bumpalo`, `log`, `simd-adler32` and more — to save the hundred lines below.
//! `flate2` is already in this workspace's graph, via `lopdf`'s stream decompression, so the one
//! primitive that genuinely should not be hand-rolled costs nothing new. That is the same trade
//! v1.2-S1 made when it refused an MCP framework: *"the ones available pull an async runtime,
//! which would cost the ban to save a few dozen lines"*.
//!
//! **XML is the opposite call and is not hand-rolled** — see `docx.rs`.
//!
//! # Everything here fails closed
//!
//! A truncated archive, a bad signature, a Zip64 record, an entry compressed with anything but
//! store or deflate, or a size that disagrees with what inflate produced: each is a named error,
//! never a shorter string. `docs/01-CONTRACT.md` §8, and the reason is the one v0 gives for
//! refusing an unknown content-stream operator — text that silently goes missing is undetectable
//! downstream.

use std::io::Read;

use engine_core::EngineError;

/// End of central directory record.
const EOCD_SIGNATURE: [u8; 4] = [b'P', b'K', 5, 6];
/// Central directory file header.
const CENTRAL_SIGNATURE: [u8; 4] = [b'P', b'K', 1, 2];
/// Local file header.
const LOCAL_SIGNATURE: [u8; 4] = [b'P', b'K', 3, 4];

/// The two compression methods an OOXML package uses in practice.
const METHOD_STORED: u16 = 0;
const METHOD_DEFLATE: u16 = 8;

/// The marker a Zip64 archive puts where a 32-bit size or offset would go.
const ZIP64_SENTINEL: u32 = 0xFFFF_FFFF;

/// A ceiling on what one part may inflate to, so a zip bomb is a named refusal rather than an
/// out-of-memory kill.
///
/// 256 MiB of XML is far past any real `word/document.xml` and far short of exhausting a host.
/// A part that needs more is not refused because it is suspicious; it is refused because this
/// reader will not decompress an unbounded stream on a caller's behalf without saying so.
const MAX_INFLATED_BYTES: u64 = 256 * 1024 * 1024;

fn malformed(detail: impl Into<String>) -> EngineError {
    EngineError::Malformed {
        what: "ooxml package".into(),
        detail: detail.into(),
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, EngineError> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| malformed(format!("truncated: no 16-bit field at {at}")))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, EngineError> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| malformed(format!("truncated: no 32-bit field at {at}")))
}

/// Whether these bytes even start like a ZIP local file header.
///
/// The cheap half of content-based detection; `crate::is_docx` is the half that decides.
pub fn looks_like_zip(bytes: &[u8]) -> bool {
    bytes.starts_with(&LOCAL_SIGNATURE)
}

/// The names of every entry in the package's central directory, in directory order.
///
/// Read from the **central directory**, which is the archive's own index, rather than by walking
/// local headers — a local header can be present for an entry the directory does not list, and
/// the directory is what a conforming reader is required to trust.
pub fn entry_names(archive: &[u8]) -> Result<Vec<String>, EngineError> {
    let mut names = Vec::new();
    walk_central_directory(archive, |name, _| {
        names.push(name.to_string());
        Ok(std::ops::ControlFlow::Continue(()))
    })?;
    Ok(names)
}

/// Read one entry by name, or a named error saying which of the ways it could fail happened.
pub fn read_entry(archive: &[u8], want: &str) -> Result<Vec<u8>, EngineError> {
    let mut found: Option<(u16, u32, u32)> = None;
    walk_central_directory(archive, |name, header| {
        if name != want {
            return Ok(std::ops::ControlFlow::Continue(()));
        }
        found = Some(header);
        Ok(std::ops::ControlFlow::Break(()))
    })?;

    let Some((method, compressed_size, uncompressed_size)) = found else {
        return Err(EngineError::MissingPart {
            part: format!("`{want}` is not an entry of this package"),
        });
    };
    let (method, compressed_size, uncompressed_size) =
        (method, compressed_size as usize, uncompressed_size as usize);

    // The local header repeats the name and carries its own extra field, whose length may differ
    // from the central one — so the data offset is computed here, not taken from the directory.
    let local_offset = found_local_offset(archive, want)?;
    let local = archive
        .get(local_offset..)
        .ok_or_else(|| malformed("local header offset is past the end of the archive"))?;
    if !local.starts_with(&LOCAL_SIGNATURE) {
        return Err(malformed(format!(
            "`{want}` points at offset {local_offset}, which is not a local file header"
        )));
    }
    let name_len = u16_at(local, 26)? as usize;
    let extra_len = u16_at(local, 28)? as usize;
    let data_start = 30 + name_len + extra_len;
    let data = local
        .get(data_start..data_start + compressed_size)
        .ok_or_else(|| malformed(format!("`{want}` is truncated: its data runs past the end")))?;

    let out = match method {
        METHOD_STORED => data.to_vec(),
        METHOD_DEFLATE => inflate(data, uncompressed_size, want)?,
        other => {
            return Err(EngineError::Unsupported {
                what: "ooxml compression method".into(),
                detail: format!(
                    "`{want}` uses method {other}; this reader implements store (0) and deflate \
                     (8), which is what an OOXML package uses. An unimplemented method is refused \
                     rather than skipped: a part that silently did not load is text missing from \
                     the record."
                ),
            })
        }
    };

    if out.len() != uncompressed_size {
        return Err(malformed(format!(
            "`{want}` declares {uncompressed_size} uncompressed byte(s) and produced {}; a part \
             that does not match its own directory entry is not a part this reader will speak for",
            out.len()
        )));
    }
    Ok(out)
}

fn inflate(data: &[u8], declared: usize, name: &str) -> Result<Vec<u8>, EngineError> {
    if declared as u64 > MAX_INFLATED_BYTES {
        return Err(EngineError::ResourceLimit {
            limit: format!("uncompressed bytes for `{name}`"),
            configured: MAX_INFLATED_BYTES.to_string(),
        });
    }
    let mut out = Vec::with_capacity(declared);
    // Bounded by the declared size plus one byte: if the stream produces more than it promised,
    // the length check in the caller fires instead of the allocation growing without limit.
    flate2::read::DeflateDecoder::new(data)
        .take(MAX_INFLATED_BYTES + 1)
        .read_to_end(&mut out)
        .map_err(|e| malformed(format!("`{name}` will not inflate: {e}")))?;
    Ok(out)
}

/// The local-header offset the central directory records for `want`.
fn found_local_offset(archive: &[u8], want: &str) -> Result<usize, EngineError> {
    let mut offset = None;
    walk_central_directory_full(archive, |name, _, local| {
        if name == want {
            offset = Some(local);
            return Ok(std::ops::ControlFlow::Break(()));
        }
        Ok(std::ops::ControlFlow::Continue(()))
    })?;
    offset.ok_or_else(|| {
        malformed(format!(
            "`{want}` vanished between two reads of the central directory"
        ))
    })
}

fn walk_central_directory<F>(archive: &[u8], mut visit: F) -> Result<(), EngineError>
where
    F: FnMut(&str, (u16, u32, u32)) -> Result<std::ops::ControlFlow<()>, EngineError>,
{
    walk_central_directory_full(archive, |name, header, _| visit(name, header))
}

/// Walk the central directory, handing each entry's name, sizes and local-header offset to `visit`.
fn walk_central_directory_full<F>(archive: &[u8], mut visit: F) -> Result<(), EngineError>
where
    F: FnMut(&str, (u16, u32, u32), usize) -> Result<std::ops::ControlFlow<()>, EngineError>,
{
    let eocd = find_eocd(archive)?;
    let entries = u16_at(&archive[eocd..], 10)? as usize;
    let directory_offset = u32_at(&archive[eocd..], 16)?;
    if directory_offset == ZIP64_SENTINEL {
        return Err(EngineError::Unsupported {
            what: "zip64 container".into(),
            detail: "this reader implements the 32-bit container, which is what an OOXML \
                     package uses; a Zip64 archive is refused rather than read with the wrong \
                     offsets"
                .into(),
        });
    }

    let mut at = directory_offset as usize;
    for index in 0..entries {
        let header = archive
            .get(at..)
            .ok_or_else(|| malformed(format!("central directory entry {index} is past the end")))?;
        if !header.starts_with(&CENTRAL_SIGNATURE) {
            return Err(malformed(format!(
                "central directory entry {index} has no `PK\\x01\\x02` signature"
            )));
        }
        let method = u16_at(header, 10)?;
        let compressed = u32_at(header, 20)?;
        let uncompressed = u32_at(header, 24)?;
        let name_len = u16_at(header, 28)? as usize;
        let extra_len = u16_at(header, 30)? as usize;
        let comment_len = u16_at(header, 32)? as usize;
        let local_offset = u32_at(header, 42)?;

        if compressed == ZIP64_SENTINEL
            || uncompressed == ZIP64_SENTINEL
            || local_offset == ZIP64_SENTINEL
        {
            return Err(EngineError::Unsupported {
                what: "zip64 container".into(),
                detail: format!("central directory entry {index} carries Zip64 sentinels"),
            });
        }

        let name_bytes = header
            .get(46..46 + name_len)
            .ok_or_else(|| malformed(format!("central directory entry {index} has no name")))?;
        // A part name that is not UTF-8 is refused rather than replaced: a lossy name would make
        // `word/document.xml` look absent when it is present under mangled bytes.
        let name = std::str::from_utf8(name_bytes).map_err(|e| {
            malformed(format!(
                "central directory entry {index} has a non-UTF-8 name: {e}"
            ))
        })?;

        if visit(
            name,
            (method, compressed, uncompressed),
            local_offset as usize,
        )?
        .is_break()
        {
            return Ok(());
        }
        at += 46 + name_len + extra_len + comment_len;
    }
    Ok(())
}

/// Locate the end-of-central-directory record by scanning back from the tail.
fn find_eocd(archive: &[u8]) -> Result<usize, EngineError> {
    // 22 bytes fixed, plus a comment of at most 65535.
    let window = archive.len().min(22 + 0xFFFF);
    let start = archive.len() - window;
    let found = archive[start..]
        .windows(4)
        .rposition(|w| w == EOCD_SIGNATURE)
        .map(|at| start + at);
    found.ok_or_else(|| {
        malformed(
            "no end-of-central-directory record. These bytes are not a ZIP container, or the \
             archive is truncated — either way there is nothing here to read a part out of.",
        )
    })
}
