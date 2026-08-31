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
//! store or deflate, a size that disagrees with what inflate produced, **or a part whose CRC-32
//! disagrees with the one its own directory records**: each is a named error, never a shorter
//! string. `docs/01-CONTRACT.md` §8, and the reason is the one v0 gives for refusing an unknown
//! content-stream operator — text that silently goes missing is undetectable downstream.
//!
//! # Two integrity checks, and why the second one took a measurement
//!
//! The length check was the only one until **v2-S14 (0.33.0)**. v2-S13's mutation harness measured
//! what it misses: on four of fourteen packages a byte flipped inside the main part's compressed
//! data left a stream `miniz_oxide` still inflated to exactly the declared length, so the damage
//! reached the XML reader and the extracted text came out byte-identical to the original's.
//!
//! Adding the CRC check was never in doubt as a correctness matter; the risk was compatibility,
//! because archives written by careless tools really do carry wrong CRCs and refusing one would be
//! a regression dressed as a hardening. So the false-refusal rate was measured first — **zero
//! across 40 valid packages and 2,370 entries**, the sixteen in `fixtures/office/` plus 26
//! real-world office documents — and the refusal shipped on that number.
//!
//! **One narrow exemption, and it is stated rather than implied.** [`read_entry_for_detection`]
//! does not verify, and it has exactly one caller: `odt::declared_media_type`, reading `mimetype`
//! to decide which reader a package belongs to. Refusing a *routing* question on an integrity
//! failure makes the engine answer a different question wrongly — measured, it turned a corrupt
//! `.ods` into *"not an OpenDocument spreadsheet"*. That function's own documentation carries the
//! argument and the evidence. Every other call site verifies.

use std::io::Read;

use ethos_parser_core::EngineError;

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
    walk_central_directory(archive, |name, _, _| {
        names.push(name.to_string());
        Ok(std::ops::ControlFlow::Continue(()))
    })?;
    Ok(names)
}

/// The archive's **first physical entry**: its name, and whether it is stored uncompressed.
///
/// Read from the local file header at **offset 0**, not from the central directory.
///
/// `None` when the bytes do not begin with a local file header, or when the header is truncated.
///
/// # Why this one reads the leading bytes when everything else reads the directory
///
/// [`entry_names`] and [`read_entry`] use the central directory because it is the archive's own
/// index and its order is "an implementation detail of whoever wrote it". This function exists for
/// the one requirement where that is exactly wrong. The OpenDocument package specification says the
/// `mimetype` entry *shall be the first file of the zip file* and *shall not be compressed*, and
/// the stated purpose is that **a consumer can identify the document type from the leading bytes
/// without inflating anything**. That is a claim about the archive's physical layout.
///
/// Checking the directory's order instead is a proxy, and it is wrong in both directions: a
/// conforming package repacked by any tool that writes its directory in name order has
/// `META-INF/manifest.xml` sorted before `mimetype`, so a genuine `.odt` would be declined — and
/// through the CLI it would fall past the office branch to the PDF reader and be refused with a
/// message about a PDF header. Reading offset 0 asks the question the specification actually poses.
pub fn first_entry(archive: &[u8]) -> Result<Option<(String, bool)>, EngineError> {
    if !archive.starts_with(&LOCAL_SIGNATURE) {
        return Ok(None);
    }
    let method = u16_at(archive, 8)?;
    let name_len = u16_at(archive, 26)? as usize;
    let Some(name_bytes) = archive.get(30..30 + name_len) else {
        return Ok(None);
    };
    // A non-UTF-8 name is refused rather than replaced, for the reason the directory walk gives:
    // a lossy name would make `mimetype` look absent when it is present under mangled bytes.
    let name = std::str::from_utf8(name_bytes)
        .map_err(|e| malformed(format!("the first local header has a non-UTF-8 name: {e}")))?;
    Ok(Some((name.to_string(), method == METHOD_STORED)))
}

/// Read one entry by name, or a named error saying which of the ways it could fail happened.
pub fn read_entry(archive: &[u8], want: &str) -> Result<Vec<u8>, EngineError> {
    read_entry_inner(archive, want, ChecksumCheck::Verify)
}

/// Whether [`read_entry_inner`] compares the CRC-32 the central directory records.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ChecksumCheck {
    /// The reading path: a part this engine will speak for must match its own checksum.
    Verify,
    /// The DETECTION path, and the distinction is not a loophole — see [`read_entry_for_detection`].
    Skip,
}

/// Read an entry for a **routing** decision, without the checksum check.
///
/// # Why detection does not verify, when reading does
///
/// Detection asks *what kind of document is this*; reading asks *what does it say*. Refusing a
/// routing question on an integrity failure does not make the engine safer — it makes it answer a
/// different question wrongly. Measured, on a real `.ods` with one bit flipped in its `mimetype`
/// entry's stored CRC: with detection verifying, `declared_media_type` returns `None`, the package
/// falls past every office branch, and the CLI refuses it with
///
/// > missing required part: `word/document.xml` — this package is a ZIP but not a word-processing
/// > document, and it is neither a workbook … nor an OpenDocument spreadsheet
///
/// which is **fail-closed naming the wrong cause** about a document that plainly *is* an
/// OpenDocument spreadsheet. That is the defect v2-S6 fixed for an `.ods`, v2-S8 for an `.rtf` and
/// for the ZIP shape, and v2-S10 for the last member of that shape; re-introducing it as the price
/// of a checksum would trade a silent corruption for a loud lie.
///
/// **Nothing is skipped that the checksum was protecting.** The one caller is
/// `odt::declared_media_type`, reading `mimetype` — which the OpenDocument container requires to be
/// **stored**, uncompressed, and which [`first_entry`] has already confirmed is stored before this
/// is reached. For stored bytes the content *is* the check: damage changes the media-type string
/// itself, and a string that no longer matches a known type fails detection on its own terms. The
/// CRC earns its place on a **deflated** part, where a damaged stream can still inflate to exactly
/// the declared length — which is the case v2-S13 found and v2-S14 exists to close.
///
/// A corrupt package therefore still refuses; it refuses on the part that carries the evidence,
/// under `Malformed { what: "ooxml part checksum" }`, which is the true cause.
pub(crate) fn read_entry_for_detection(archive: &[u8], want: &str) -> Result<Vec<u8>, EngineError> {
    read_entry_inner(archive, want, ChecksumCheck::Skip)
}

fn read_entry_inner(
    archive: &[u8],
    want: &str,
    checksum: ChecksumCheck,
) -> Result<Vec<u8>, EngineError> {
    // **One walk** (v2-S15). This called `walk_central_directory` for the header and then
    // `found_local_offset` for the offset, and those are two full O(entries) traversals of the
    // same directory for two fields of the same 46-byte record — so reading a part cost two
    // scans, and a package of 2,370 entries paid ~4,700 header parses to retrieve one part. The
    // offset sits in the header the first walk already had; it was being discarded by a thin
    // adapter whose only job was to drop the third closure argument.
    let mut found: Option<(u16, u32, u32, u32, usize)> = None;
    walk_central_directory(archive, |name, header, local_offset| {
        if name != want {
            return Ok(std::ops::ControlFlow::Continue(()));
        }
        let (method, crc, compressed, uncompressed) = header;
        found = Some((method, crc, compressed, uncompressed, local_offset));
        Ok(std::ops::ControlFlow::Break(()))
    })?;

    let Some((method, declared_crc, compressed_size, uncompressed_size, local_offset)) = found
    else {
        return Err(EngineError::MissingPart {
            part: format!("`{want}` is not an entry of this package"),
        });
    };
    let (method, compressed_size, uncompressed_size) =
        (method, compressed_size as usize, uncompressed_size as usize);

    // The local header repeats the name and carries its own extra field, whose length may differ
    // from the central one — so the data offset is computed below from the LOCAL header rather
    // than taken from the directory. Only the header's position comes from the directory.
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

    // **The CRC-32 check, v2-S14.** The length check above was the only integrity check this
    // reader made until 0.33.0, and v2-S13's mutation harness measured what it misses: a byte
    // flipped inside a deflated part can leave a stream `miniz_oxide` still inflates to EXACTLY
    // the declared length, substituting a NUL where the invalid back-reference was. zlib refuses
    // the same bytes outright — the permissiveness is the backend's, not the format's — and the
    // damage landed in a namespace URI the OOXML readers match by suffix, so the extracted text
    // came out byte-identical to the original's. The directory carried a CRC-32 for that entry
    // the whole time and nothing read it.
    //
    // **Measured before it shipped**, because refusing a valid archive is a regression dressed as
    // a hardening: 40 valid packages and 2,370 entries — the 16 in `fixtures/office/` plus 26
    // real-world `.docx`/`.xlsx`/`.pptx` gathered off a developer machine — produced **zero**
    // mismatches. Zero was the condition the owner set for shipping the refusal.
    let mut crc = flate2::Crc::new();
    crc.update(&out);
    let computed = crc.sum();
    if checksum == ChecksumCheck::Verify && computed != declared_crc {
        return Err(EngineError::Malformed {
            // A DISTINCT `what` from `malformed()`'s "ooxml package", because a checksum failure
            // is a different cause from a truncation or a bad signature and a caller writing
            // policy has to tell them apart without parsing prose.
            what: "ooxml part checksum".into(),
            detail: format!(
                "`{want}` inflated to its declared {uncompressed_size} byte(s) but its CRC-32 is \
                 {computed:#010x} where the central directory records {declared_crc:#010x}. The \
                 part is corrupt in a way the length cannot see, which is exactly the case this \
                 check exists for: a damaged deflate stream that still produces the right number \
                 of bytes reaches the XML reader as though it were intact, and the text it yields \
                 can be indistinguishable from the original's."
            ),
        });
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
    // **Grow into the declared size; do not reserve it** (v2-S15). `declared` comes from the
    // central directory, which is attacker-controlled up to `MAX_INFLATED_BYTES` — so
    // `with_capacity(declared)` let a two-kilobyte part that merely CLAIMS 256 MiB allocate
    // 256 MiB before inflate produced a byte. The length check below refuses that part, but only
    // after the allocation, and this runs once per part read: a workbook is one part per sheet, so
    // forty sheets each declaring the cap reserved ten gigabytes across a read that returns
    // nothing. The ceiling meant to bound the damage was setting the size of each allocation.
    //
    // `read_to_end` doubles from a small start, so declining to trust the declaration costs a
    // handful of reallocations on a genuinely large part and 64 KiB on a hostile one.
    let mut out = Vec::with_capacity(declared.min(64 * 1024));
    // Bounded by `MAX_INFLATED_BYTES + 1`, not by the declared size — this comment said "the
    // declared size plus one byte" until v2-S13.5 and never matched the line below it. So a part
    // declaring a few bytes that inflates to 200 MiB is held to the 256 MiB cap rather than to its
    // own declaration, and the caller's length check is what refuses it afterwards. The one extra
    // byte is what makes a stream that overruns the cap detectable instead of silently truncated.
    flate2::read::DeflateDecoder::new(data)
        .take(MAX_INFLATED_BYTES + 1)
        .read_to_end(&mut out)
        .map_err(|e| malformed(format!("`{name}` will not inflate: {e}")))?;
    Ok(out)
}

/// Walk the central directory, handing each entry's name, sizes and local-header offset to `visit`.
///
/// There was a second, thinner `walk_central_directory` here whose whole body was
/// `walk_central_directory_full(archive, |name, header, _| visit(name, header))` — one walker
/// wrapping another to drop an argument. It cost a full second traversal per part read, because
/// the caller that needed the offset had to go back for it through `found_local_offset`. Both are
/// gone (v2-S15); a call site that does not want the offset binds `_`.
///
/// `found_local_offset` took the error `"vanished between two reads of the central directory"`
/// with it. That condition was only ever reachable BECAUSE there were two reads, so removing the
/// second read removes the race rather than leaving a message about one that can no longer occur.
fn walk_central_directory<F>(archive: &[u8], mut visit: F) -> Result<(), EngineError>
where
    F: FnMut(&str, (u16, u32, u32, u32), usize) -> Result<std::ops::ControlFlow<()>, EngineError>,
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
        // The CRC-32 the archive's own index records for this entry's UNCOMPRESSED bytes.
        let crc = u32_at(header, 16)?;
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
            (method, crc, compressed, uncompressed),
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
