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

//! **A declared size is a claim, not a reservation** (v2-S15).
//!
//! `zip::read_entry` inflated into `Vec::with_capacity(declared)`, where `declared` is the
//! uncompressed size from the central directory — attacker-controlled, and checked only against
//! `MAX_INFLATED_BYTES` (256 MiB). So a two-kilobyte part that merely CLAIMED 256 MiB allocated
//! 256 MiB before inflate produced a byte. The part is refused afterwards by the length check, so
//! this was never a correctness bug; it was an amplification one, and the ceiling intended to
//! bound the damage was what set the size of each allocation. It runs once per part read, and a
//! workbook reads one part per sheet.
//!
//! Asserting this needs the allocator, because the observable is a `with_capacity` that is
//! immediately discarded on the error path — there is no returned value to inspect. So this file
//! installs a counting global allocator and watches the peak across one read. That is also why it
//! is its own test binary: a `#[global_allocator]` is per-binary, and putting it here leaves every
//! other office suite on the system allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

struct Counting;

// SAFETY: every method forwards to `System` unchanged; the atomics only observe.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            let live = LIVE
                .fetch_add(new_size.saturating_sub(layout.size()), Ordering::Relaxed)
                .saturating_add(new_size.saturating_sub(layout.size()));
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// The lie: 200 MiB, comfortably under `MAX_INFLATED_BYTES` so the cap check at the top of
/// `read_entry` passes it through to the allocation.
const DECLARED_LIE: u32 = 200 * 1024 * 1024;

/// A raw-deflate stream carrying `data` in one stored (BTYPE=00) block.
///
/// Hand-built rather than produced with `flate2` so this test needs no dev-dependency: a stored
/// block is `BFINAL|BTYPE` then LEN and its one's complement, then the bytes verbatim.
fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x01];
    let len = u16::try_from(data.len()).expect("test payload fits a stored block");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// One deflated entry whose central-directory uncompressed size is whatever `declared` says,
/// regardless of what the data actually inflates to.
fn zip_with_declared(name: &str, payload: &[u8], declared: u32) -> Vec<u8> {
    let compressed = deflate_stored(payload);
    let crc = crc32(payload);
    let mut out = Vec::new();

    // Local file header. Method 8 = deflate; the two `0,0` pairs after the version are flags,
    // then method, then time and date.
    out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    out.extend_from_slice(&[10, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    out.extend_from_slice(&declared.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&compressed);

    let directory_offset = out.len() as u32;
    let mut directory = Vec::new();
    directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    directory.extend_from_slice(&[10, 0, 10, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
    directory.extend_from_slice(&crc.to_le_bytes());
    directory.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    directory.extend_from_slice(&declared.to_le_bytes());
    directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
    directory.extend_from_slice(&[0; 12]);
    directory.extend_from_slice(&0u32.to_le_bytes());
    directory.extend_from_slice(name.as_bytes());

    let directory_size = directory.len() as u32;
    out.extend_from_slice(&directory);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&directory_size.to_le_bytes());
    out.extend_from_slice(&directory_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// **A part that claims 200 MiB and carries eleven bytes must not allocate 200 MiB.**
///
/// The read is expected to FAIL — the declared length does not match what inflated, and refusing
/// that is correct and unchanged. What this asserts is the cost of reaching the refusal.
#[test]
fn a_lying_declared_size_does_not_reserve_itself() {
    let archive = zip_with_declared("word/document.xml", b"hello world", DECLARED_LIE);

    PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
    let before = PEAK.load(Ordering::Relaxed);

    let result = ethos_parser_office::zip::read_entry(&archive, "word/document.xml");

    let growth = PEAK.load(Ordering::Relaxed).saturating_sub(before);

    assert!(
        result.is_err(),
        "a part whose declaration does not match what it inflates to must still be refused; \
         this test is about the cost of refusing it, not about accepting it"
    );

    // 8 MiB is two orders of magnitude below the 200 MiB claim and far above the 64 KiB the
    // clamped `with_capacity` takes, so this discriminates the fix from the defect without
    // being sensitive to allocator noise or to the harness around it.
    const CEILING: usize = 8 * 1024 * 1024;
    assert!(
        growth < CEILING,
        "reading an 11-byte part that DECLARES {DECLARED_LIE} bytes grew peak allocation by \
         {growth} bytes. Before v2-S15 `Vec::with_capacity(declared)` reserved the declaration \
         itself, so this was ~{DECLARED_LIE}. The declaration is attacker-controlled and this \
         path runs once per part read."
    );
}

/// The clamp must not have broken the ordinary case: a part whose declaration is honest still
/// reads, and still reads to exactly its bytes.
#[test]
fn an_honest_part_still_reads() {
    let payload = b"<?xml version=\"1.0\"?><document/>";
    let archive = zip_with_declared("word/document.xml", payload, payload.len() as u32);
    let got = ethos_parser_office::zip::read_entry(&archive, "word/document.xml")
        .expect("an honest part must read");
    assert_eq!(got, payload, "the clamp must not truncate or alter content");
}
