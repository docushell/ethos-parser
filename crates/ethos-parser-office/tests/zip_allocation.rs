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
/// Allocation EVENTS, not bytes. Live-byte peak cannot see the defect v2-S15 introduced and then
/// removed: growing a 16 MiB part by doubling from 64 KiB frees each block as it allocates the
/// next, so live bytes look fine while RSS — a high-water mark that never comes back down —
/// climbed to 392 MiB on a sixteen-sheet workbook that had cost 19.5 MiB. Counting the events is
/// what sees it.
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

// SAFETY: every method forwards to `System` unchanged; the atomics only observe.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        ALLOCS.fetch_add(1, Ordering::Relaxed);
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
        ALLOCS.fetch_add(1, Ordering::Relaxed);
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

/// A raw-deflate stream carrying `data` in stored (BTYPE=00) blocks.
///
/// Hand-built rather than produced with `flate2` so this test needs no dev-dependency: a stored
/// block is `BFINAL|BTYPE` then LEN and its one's complement, then the bytes verbatim. LEN is a
/// `u16`, so anything past 65,535 bytes is several blocks with BFINAL set only on the last.
fn deflate_stored(data: &[u8]) -> Vec<u8> {
    const MAX_BLOCK: usize = u16::MAX as usize;
    let mut out = Vec::new();
    let mut rest = data;
    loop {
        let take = rest.len().min(MAX_BLOCK);
        let final_block = take == rest.len();
        out.push(u8::from(final_block));
        let len = take as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(&rest[..take]);
        rest = &rest[take..];
        if final_block {
            return out;
        }
    }
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

/// **An honest part is reserved in one go, not grown into by doubling.**
///
/// This is the regression test for the fix that was itself a regression. v2-S15 first replaced
/// `with_capacity(declared)` with `with_capacity(declared.min(64 * 1024))` — safe against a lying
/// declaration, and a 20x memory regression on honest files, because `read_to_end` then doubles
/// 64 KiB -> 2 MiB in about five steps and RSS keeps the high-water mark of every intermediate.
/// Measured on a workbook of sixteen 16 MiB sheets: 19.5 MiB peak before, 392.2 MiB after,
/// scaling linearly with sheet count where it had been flat.
///
/// The bound is `data.len() * 1032` — DEFLATE's maximum expansion against the compressed bytes,
/// which are physically present rather than claimed. So an honest part reserves its exact size in
/// one allocation while a lying one is still held to what its bytes could possibly produce.
///
/// Counting allocation EVENTS rather than bytes is the point: the byte-peak assertion above
/// passed throughout the regression.
#[test]
fn an_honest_part_is_reserved_once_rather_than_doubled_into() {
    // Two megabytes of one repeated byte: compresses hard, so `data.len() * 1032` still comfortably
    // covers the true size and the reservation is exact.
    let payload = vec![b'p'; 2 * 1024 * 1024];
    let archive = zip_with_declared("xl/worksheets/sheet1.xml", &payload, payload.len() as u32);

    let before = ALLOCS.load(Ordering::Relaxed);
    let got = ethos_parser_office::zip::read_entry(&archive, "xl/worksheets/sheet1.xml")
        .expect("an honest part must read");
    let events = ALLOCS.load(Ordering::Relaxed) - before;

    assert_eq!(got.len(), payload.len(), "the part must read in full");

    // Measured at v2-S15, on this exact payload: reserving correctly takes **3** events, and
    // growing into it from a 64 KiB clamp takes **9** — the five doublings plus the same
    // incidentals. Six sits between them with headroom on the side that matters, and the numbers
    // are written down because a threshold nobody measured is a threshold that drifts.
    //
    // The first draft of this assertion said `< 12`, and the 64 KiB clamp PASSED it. A regression
    // test that does not fail on the regression is the thing this file is about, so it was
    // re-measured against all three variants before being pinned.
    assert!(
        events < 6,
        "reading a 2 MiB part took {events} allocation events. Reserving the right size takes a \
         handful; growing into it by doubling from 64 KiB takes five more and leaves RSS at the \
         high-water mark of every intermediate block, which is the 20x regression v2-S15 measured \
         and removed."
    );
}
