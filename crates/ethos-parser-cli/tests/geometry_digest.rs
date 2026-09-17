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

//! **Every box `extract` emits on the committed fixtures, pinned as one digest per document**
//! (`docs/OPEN-WORK.md` §5, 2026-09-17).
//!
//! No golden covered geometry. A box that moved reached CI only through a test that reads that
//! box, so a transform, a quantisation step or a typed absence could move on a fixture no test
//! names and ship unannounced. This pins, for every PDF the manifest resolves under the `engine`
//! and `conformance` roots and for the two form documents of the `gate` root, the SHA-256 of the
//! canonical bytes of `extract`'s `geometry` member — every node's measured box or typed absence,
//! keyed by node id — or, for a document `extract` refuses, its exit code.
//!
//! **A move here is an announcement, not a failure to route around.** When the table moves, the
//! test prints it whole as it now stands: paste it over `PINNED` in the commit that moves it, and
//! say in that commit which boxes moved and why. The six larger gate documents are left out on cost
//! (`nist-sp-800-53Ar5` alone emits a 1 GB artifact); the table gate and the round-trip instrument
//! read them.
//!
//! Fixtures resolve through `fixtures/manifest.json`'s roots. **A missing fixture is a failure,
//! never a skip.**

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// The gate documents small enough to extract on every run.
const GATE: [&str; 2] = ["irs-f1040sd-2025.pdf", "irs-fw9.pdf"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

fn manifest() -> Value {
    serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest readable"),
    )
    .expect("manifest is valid JSON")
}

fn root(manifest: &Value, name: &str) -> PathBuf {
    let decl = &manifest["roots"][name];
    match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(decl["default"].as_str().expect("default")),
    }
}

/// The documents, as `root/path` beside the file, sorted by that name.
fn corpus() -> Vec<(String, PathBuf)> {
    let manifest = manifest();
    let mut out = Vec::new();
    for f in manifest["fixtures"].as_array().expect("fixtures") {
        let r = f["root"].as_str().expect("root");
        let path = f["path"].as_str().expect("path");
        let kept = matches!(r, "engine" | "conformance") || (r == "gate" && GATE.contains(&path));
        if kept && path.ends_with(".pdf") {
            let file = root(&manifest, r).join(path);
            assert!(
                file.is_file(),
                "fixture `{r}/{path}` missing at {}. A missing fixture is a failure, never a skip.",
                file.display()
            );
            out.push((format!("{r}/{path}"), file));
        }
    }
    out.sort();
    let counts = &manifest["counts"];
    let expected = counts["engine_owned"].as_u64().expect("engine_owned")
        + counts["conformance_ethos_owned"]
            .as_u64()
            .expect("conformance_ethos_owned")
        + GATE.len() as u64;
    assert_eq!(
        out.len() as u64,
        expected,
        "the manifest's counts and its fixture list disagree about what this test reads"
    );
    out
}

/// The digest of one document's `geometry` member, or the exit code `extract` refused it with.
fn digest(file: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("extract")
        .arg(file)
        .output()
        .expect("the engine binary runs");
    match out.status.code() {
        Some(0) => {
            let artifact: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
            let geometry = artifact
                .get("geometry")
                .expect("an extract artifact carries `geometry`");
            let bytes = ethos_parser_core::c14n_bytes(geometry).expect("canonicalises");
            format!("sha256:{}", ethos_parser_core::sha256_hex_bytes(&bytes))
        }
        Some(code) => format!("exit {code}"),
        None => "killed by a signal".to_string(),
    }
}

/// `root/path` → the digest of its `geometry`, or the exit code. Regenerate by running this test
/// and pasting the table it prints; name the moved boxes in the commit that does.
const PINNED: &[(&str, &str)] = &[
    (
        "conformance/failure/corrupt-header-valid/document.pdf",
        "exit 2",
    ),
    (
        "conformance/failure/image-only-or-blank-page/document.pdf",
        "sha256:4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945",
    ),
    ("conformance/failure/invalid-header/document.pdf", "exit 2"),
    (
        "conformance/failure/memory-limit-simulated/document.pdf",
        "sha256:1661924b22bc63e324b28e29058b73290ab6c910d6ef22bd83bdeefee192a109",
    ),
    (
        "conformance/failure/password-protected/document.pdf",
        "exit 2",
    ),
    (
        "conformance/foreign/opendataloader/real/source.pdf",
        "sha256:0ba82d98759d44097b731f78becc896bde5872a0f76266c5de291bb14128c238",
    ),
    (
        "conformance/synthetic/heading-export/document.pdf",
        "sha256:392f5c64ff0eab595f846a49ad1ef3c38b7f022a616b5102937ab0013febb249",
    ),
    (
        "conformance/synthetic/hyphenated-line-break/document.pdf",
        "sha256:95438a55e3889ab895646262a31f4ee32d215c0ef507c7a93c3488dbbb159e18",
    ),
    (
        "conformance/synthetic/ligature-fi-embedded-font/document.pdf",
        "sha256:9e68ac928bdb1a26065e1f44c2aa83572fbd4ed1fdaecd2ab69d719562efa3c6",
    ),
    (
        "conformance/synthetic/list-items/document.pdf",
        "sha256:4429677b1f989dec2bb8c7538bcb241c9e95c74668b812a2475f5f2931311653",
    ),
    (
        "conformance/synthetic/rotation-90/document.pdf",
        "sha256:67bc4a47d880d194dac3b20cf5060c120f11db6b0e2ec5c8def065856472f81b",
    ),
    (
        "conformance/synthetic/simple-text/document.pdf",
        "sha256:1661924b22bc63e324b28e29058b73290ab6c910d6ef22bd83bdeefee192a109",
    ),
    (
        "conformance/synthetic/table-regular-grid/document.pdf",
        "sha256:09e4537c2b818c2bc87c35e6346ac53efd23d12d927228fade51b14691e993ec",
    ),
    (
        "conformance/synthetic/two-columns/document.pdf",
        "sha256:5d3e757f44b0acfa756954512487034461da93278c3aaf62ec22fd4fc654ff8f",
    ),
    (
        "conformance/synthetic/two-lines/document.pdf",
        "sha256:2992ff261edf10d31bb270cabbb0a4eec0e112c6ef312791fc5d21f906088748",
    ),
    (
        "engine/absent-font-metrics/document.pdf",
        "sha256:9e68ac928bdb1a26065e1f44c2aa83572fbd4ed1fdaecd2ab69d719562efa3c6",
    ),
    (
        "engine/absent-font-widths/document.pdf",
        "sha256:9e68ac928bdb1a26065e1f44c2aa83572fbd4ed1fdaecd2ab69d719562efa3c6",
    ),
    (
        "engine/annotation-contents/document.pdf",
        "sha256:0d48adbf39aaaf1775f43f9eaa191ad7a8faa2cb7156f18244d5dfc8d26a237c",
    ),
    (
        "engine/background-panel-not-a-grid/document.pdf",
        "sha256:6f9f6f1b0aa13cb4a4f8c791d9df2cc2896164f47200d84cfeeec81421d6fee9",
    ),
    (
        "engine/both-table-rules/document.pdf",
        "sha256:2a0e8f9ee80c93d51df41e1b38d0498f9ef5d0f71d4b98bfab045c2a76e9b961",
    ),
    (
        "engine/broken-font-encoding/document.pdf",
        "sha256:b5aacecc6b4f457354f6c3241fc49ef07ed34ec1ec38060e6c95c4875a54f893",
    ),
    (
        "engine/composite-font-cid-widths/document.pdf",
        "sha256:6ce61bce70ffe2321c70920ac5305c01a55f051a4077aaf1bcaf32f147be569f",
    ),
    (
        "engine/composite-font-non-identity-cmap/document.pdf",
        "sha256:9e68ac928bdb1a26065e1f44c2aa83572fbd4ed1fdaecd2ab69d719562efa3c6",
    ),
    (
        "engine/crop-box-smaller-than-media/document.pdf",
        "sha256:549ba8fae1afce6ccb6d3d7c9d6812059281bd830117ee7a3320711fbdf9633f",
    ),
    (
        "engine/engine-tagged-blocks/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/engine-tagged-classmap/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/engine-tagged-mixed/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/engine-tagged-nested-frames/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/engine-tagged-widget-objr/document.pdf",
        "sha256:213a540c99412fd98f311d26eac820881168ba4c773b26297ee34d8299ec2434",
    ),
    (
        "engine/form-field-value/document.pdf",
        "sha256:213a540c99412fd98f311d26eac820881168ba4c773b26297ee34d8299ec2434",
    ),
    (
        "engine/form-orphan-widget/document.pdf",
        "sha256:9079bf0f44d5f9ea14d44de92c86b6e2b659ae4d0d8b85e8d4d85089d11579af",
    ),
    (
        "engine/form-xfa-stub/document.pdf",
        "sha256:70a0dd2db9980624fa756687a73db0013b66821e90af4e8e65b7626f19cfd9dd",
    ),
    (
        "engine/form-xobject-text-drawn/document.pdf",
        "sha256:90fe6d81cc1d2b270ce8d4c3db17774107dbd66107d05f5640082d757df795aa",
    ),
    (
        "engine/horizontal-scaling-tz/document.pdf",
        "sha256:0cd5a1502fbed8b0c3df1978fdbe2b29e8279851a98ef264bf494dedb699135b",
    ),
    (
        "engine/image-declared-not-drawn/document.pdf",
        "sha256:6fe6218baebdfe329500681bc630ba3fcb162031084ed9a5c6c8286606a38f78",
    ),
    (
        "engine/image-xobject-drawn/document.pdf",
        "sha256:d4181a340d0c57bda8de56fd289336109227dca0115e1b0cb2c43b84ff0d3fd6",
    ),
    (
        "engine/ink-past-the-media-box/document.pdf",
        "sha256:b29d2e6c0f6d242d57bbfcd153184da0365f406115f2ae6ef8265ea282c6f4d6",
    ),
    (
        "engine/inline-image-filtered/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/invisible-render-mode/document.pdf",
        "sha256:8776b15af64da76f3e63873a384470df50d1c67bd3b18d9be1416e91797793d3",
    ),
    (
        "engine/leading-gap-nested-frames/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/leading-gap-two-blocks/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/markdown-hyphen-break/document.pdf",
        "sha256:7fc3063779637790bc4eeb9de9307ea40c2d2e1a31f57d061359626430c22716",
    ),
    (
        "engine/markdown-table-cells/document.pdf",
        "sha256:fe10ead670873935a6e094913b56dd4e229fa5a6457a6a361bd8ca9342f0fc88",
    ),
    (
        "engine/markdown-two-blocks/document.pdf",
        "sha256:a6e74144b71e67986fafa2c7a7aea5f8e1891c8365143dceafce9950e75f7d28",
    ),
    (
        "engine/measured-ink-box/document.pdf",
        "sha256:9ddc15b9df0643e27915ac9fa9d1f935cc9480358ae0b1cca92da42fe26ef79f",
    ),
    (
        "engine/off-page-and-offset-box/document.pdf",
        "sha256:6d4bc69319a24d1cd8acd7f904a6d39a280e8ea834c0da530a6215ed0082c96b",
    ),
    (
        "engine/rotated-and-mirrored-text/document.pdf",
        "sha256:1e35803a16bd36592fda0833641fffa7dc931818dafd3de4bdbe4391292dfd8e",
    ),
    (
        "engine/ruled-table-grid/document.pdf",
        "sha256:91bed705f32c27b750cfb5285b1eb1dd4cd6998d0b555f1c00884ee8ef58b7ec",
    ),
    (
        "engine/ruled-table-overlap/document.pdf",
        "sha256:c0ec3e29a491a77b6f76aad2c56f9f5296f9c24e0f72f3992edc41678a15885e",
    ),
    (
        "engine/ruled-wins-shared-region/document.pdf",
        "sha256:e285d38740728ad114e6b4f0c8df2a18c1d16bb33263bc6ec9331546b9ac472f",
    ),
    (
        "engine/shared-content-stream/document.pdf",
        "sha256:269ae951831809a4a7de7ca7db13987857d71ac29242f25840aba0ede46ebc2a",
    ),
    (
        "engine/show-text-quote-operators/document.pdf",
        "sha256:2311e2f8e43a2f29d20dd6e56d15f1e8fb438a64e38390eebf73d44ded0469cc",
    ),
    (
        "engine/simple-font-two-byte-tounicode/document.pdf",
        "sha256:8662da10f98b6af03f30dc784cf17534052bce6584f9ca7fde98bb469fdf42d8",
    ),
    (
        "engine/stroke-ruled-columns-not-drawn/document.pdf",
        "sha256:799ad8cc130543177945661e680e897fb8f97cf7a7326f40e23c164f21943351",
    ),
    (
        "engine/stroke-ruled-field-boxes/document.pdf",
        "sha256:f582a412c73da0fb6a78fb4ce14b791d7212e9c514e52f058f431040ba719bec",
    ),
    (
        "engine/stroke-ruled-worksheet/document.pdf",
        "sha256:8c1d83565243609fd82571d6573de565e42e49404ad7ee020f4be705c7b94f11",
    ),
    (
        "engine/synthesized-space-tj/document.pdf",
        "sha256:f7c7518097e6c0a288d89257abbf5d8843fc017451619bddeeac30a2afbb0a7d",
    ),
    ("engine/tagged-cycle/document.pdf", "exit 2"),
    (
        "engine/tagged-list-items/document.pdf",
        "sha256:05472bd78c74d3b7fda15abd498268fa942c845fc279d5e2e34168faf892d65b",
    ),
    (
        "engine/tagged-rolemap/document.pdf",
        "sha256:8be4f818eccad823ab2528233407a07f9d74b6bd8c9745a31941c0e809ce5505",
    ),
    (
        "engine/tagged-structure-roles/document.pdf",
        "sha256:921af13b2f98ba2e1ba8d4a25c44aa3afe318791744ad3bab128fb879adb7a86",
    ),
    (
        "engine/tagged-table-agrees/document.pdf",
        "sha256:1fceb525c4ac0978b6d682de2a231df8452fed7e8288b376b99e81ed410903ac",
    ),
    (
        "engine/tagged-table-disagrees/document.pdf",
        "sha256:1fceb525c4ac0978b6d682de2a231df8452fed7e8288b376b99e81ed410903ac",
    ),
    (
        "engine/tagged-widget-objr/document.pdf",
        "sha256:213a540c99412fd98f311d26eac820881168ba4c773b26297ee34d8299ec2434",
    ),
    (
        "engine/two-column-14-lines/document.pdf",
        "sha256:c33c75d295097e129af97af4fcff2d26da17f2657a9db12f3368050722954e33",
    ),
    (
        "engine/two-column-15-lines/document.pdf",
        "sha256:93e477cb097f9608e911681fd6ce6c0b2744ca3b3ce6ad72edb51ab827149a58",
    ),
    (
        "engine/unruled-near-miss/document.pdf",
        "sha256:c8da07dede912bfa990917d6135b720afacbe77d43fbcbab7750c3ba7de21ccb",
    ),
    (
        "engine/untagged-artifact-furniture/document.pdf",
        "sha256:0105244b13b31c986bd51687b53c28db8a3722f50d54bea87e5b2cb17e942134",
    ),
    (
        "engine/untagged-mcid-by-name/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/untagged-mcid-no-tree/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/untagged-oc-by-name/document.pdf",
        "sha256:b90ded4aa43e84ed8286eb818104aade45e384607ee51789f68e141a9906a417",
    ),
    (
        "engine/untagged-shredded-line/document.pdf",
        "sha256:14433ea3798455a427b62f7572fe0ad2acad44bffa0f1f5eae54383305f13bc6",
    ),
    (
        "engine/whitespace-past-the-page-edge/document.pdf",
        "sha256:c1834cb25e192815f388c0fabdfde44932ec43f12006b6ac4043e58a05ade3bd",
    ),
    (
        "gate/irs-f1040sd-2025.pdf",
        "sha256:e88c09571cb5883f555196dd9cbab797465c19d82d4bc6d89648d812749f469c",
    ),
    (
        "gate/irs-fw9.pdf",
        "sha256:a9c0a60fae743582fe3cf061bef5abcd4d2fa32cf874aba2ce216720c029f61e",
    ),
];

#[test]
fn every_box_on_the_committed_fixtures_is_pinned() {
    let corpus = corpus();
    // Four threads: the documents are independent and the binary is the slow part.
    let now: Vec<(String, String)> = std::thread::scope(|scope| {
        let chunks: Vec<_> = corpus
            .chunks(corpus.len().div_ceil(4))
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|(name, file)| (name.clone(), digest(file)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        chunks
            .into_iter()
            .flat_map(|handle| handle.join().expect("a digest thread panicked"))
            .collect()
    });

    let pinned: Vec<(String, String)> = PINNED
        .iter()
        .map(|&(name, d)| (name.to_string(), d.to_string()))
        .collect();
    if now == pinned {
        return;
    }
    let was = |name: &str| {
        pinned
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d.as_str())
    };
    let moved: Vec<String> = now
        .iter()
        .filter(|(name, d)| was(name) != Some(d.as_str()))
        .map(|(name, d)| format!("  {name}: {} -> {d}", was(name).unwrap_or("(not pinned)")))
        .chain(
            pinned
                .iter()
                .filter(|(name, _)| !now.iter().any(|(n, _)| n == name))
                .map(|(name, d)| format!("  {name}: {d} -> (no longer read)")),
        )
        .collect();
    let table: String = now
        .iter()
        .map(|(name, d)| format!("    (\"{name}\", \"{d}\"),\n"))
        .collect();
    panic!(
        "the geometry `extract` emits moved on {} of {} document(s):\n{}\n\nIf the move is \
         intended, paste this over `PINNED` in the commit that makes it, and name there which \
         boxes moved and why:\n\nconst PINNED: &[(&str, &str)] = &[\n{table}];\n",
        moved.len(),
        now.len(),
        moved.join("\n")
    );
}
