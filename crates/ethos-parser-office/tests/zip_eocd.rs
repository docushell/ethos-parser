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

//! **The last `PK\x05\x06` in the trailing window is not necessarily the record** (v2-S15).
//!
//! `find_eocd` took it with `rposition` and nothing else. The end-of-central-directory record is
//! followed by a comment of up to 65535 arbitrary bytes, so a comment that happens to contain the
//! signature puts the last match INSIDE the comment, past the real record — and the reads that
//! follow then interpret comment bytes as the entry count and the directory offset.
//!
//! Not exploitable into more than a confusing refusal: the walker validates each entry's own
//! signature and gives up. But "this package is malformed" is the wrong answer to a valid archive,
//! and the cause it names is not the one the file has.
//!
//! A conforming candidate identifies itself — its declared comment length must put the end of the
//! file exactly `22 + comment_len` bytes later — which is what the fix checks.

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

/// A single-entry stored ZIP, with `comment` appended after the EOCD and declared in it.
fn zip_with_comment(name: &str, data: &[u8], comment: &[u8]) -> Vec<u8> {
    let crc = crc32(data);
    let mut out = Vec::new();

    out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    out.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(data);

    let directory_offset = out.len() as u32;
    let mut dir = Vec::new();
    dir.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    dir.extend_from_slice(&[10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    dir.extend_from_slice(&crc.to_le_bytes());
    dir.extend_from_slice(&(data.len() as u32).to_le_bytes());
    dir.extend_from_slice(&(data.len() as u32).to_le_bytes());
    dir.extend_from_slice(&(name.len() as u16).to_le_bytes());
    dir.extend_from_slice(&[0; 12]);
    dir.extend_from_slice(&0u32.to_le_bytes());
    dir.extend_from_slice(name.as_bytes());

    let directory_size = dir.len() as u32;
    out.extend_from_slice(&dir);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&directory_size.to_le_bytes());
    out.extend_from_slice(&directory_offset.to_le_bytes());
    out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
    out.extend_from_slice(comment);
    out
}

const PART: &str = "word/document.xml";
const BODY: &[u8] = b"<?xml version=\"1.0\"?><document/>";

/// The control: no comment, and the last match IS the record.
#[test]
fn an_archive_without_a_comment_reads() {
    let z = zip_with_comment(PART, BODY, b"");
    let got = ethos_parser_office::zip::read_entry(&z, PART).expect("reads");
    assert_eq!(got, BODY);
}

/// An ordinary comment does not move the record, and must not confuse the scan either.
#[test]
fn an_archive_with_a_plain_comment_reads() {
    let z = zip_with_comment(
        PART,
        BODY,
        b"produced by a tool that likes to sign its work",
    );
    let got = ethos_parser_office::zip::read_entry(&z, PART).expect("reads");
    assert_eq!(got, BODY);
}

/// **The defect**: a comment containing the EOCD signature.
///
/// `rposition` finds the copy in the comment, 8 bytes from the end, and reads the entry count and
/// directory offset out of whatever follows it. Before v2-S15 this refused a perfectly valid
/// archive, naming a malformation it does not have.
#[test]
fn a_comment_containing_the_signature_does_not_displace_the_record() {
    // The signature, then four bytes so it is not itself at the very end — the shape a nested
    // archive or an embedded thumbnail leaves behind.
    let z = zip_with_comment(PART, BODY, b"PK\x05\x06\x00\x00\x00\x00");
    let got = ethos_parser_office::zip::read_entry(&z, PART)
        .expect("a valid archive whose COMMENT contains the signature must still read");
    assert_eq!(got, BODY);
}

/// The signature at the very end of the comment — the worst placement, because the false
/// candidate is then exactly where a record would be if the comment were empty.
#[test]
fn a_signature_at_the_end_of_the_comment_does_not_displace_the_record() {
    let z = zip_with_comment(PART, BODY, b"trailing: PK\x05\x06");
    let got = ethos_parser_office::zip::read_entry(&z, PART)
        .expect("the false candidate must lose to the one whose comment length checks out");
    assert_eq!(got, BODY);
}

/// Bytes that are not an archive at all are still refused, and for the right reason. A scan that
/// became more permissive while looking for a self-consistent candidate would be a worse trade
/// than the bug it fixed.
#[test]
fn bytes_with_no_record_are_still_refused() {
    let e = ethos_parser_office::zip::read_entry(b"PK\x05\x06 and nothing else at all", PART)
        .expect_err("this is not a ZIP");
    assert_eq!(e.code(), "malformed", "got {e}");
}
