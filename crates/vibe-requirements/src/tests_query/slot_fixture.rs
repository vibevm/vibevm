//! Test-only Mixed record for a normal pre-descriptor installed package.

use std::fs;
use std::path::Path;

use vibe_core::ContentHash;
use vibe_core::manifest::SpecFormat;

pub(super) fn write_legacy_record(slot: &Path, body: &str) {
    let specs_path = vibe_core::layout::current_specs_root()
        .join("RULE.md")
        .to_string_lossy()
        .replace('\\', "/");
    let manifest = fs::read(slot.join("vibe.toml")).unwrap();
    let record = vibe_workspace::vibedeps::SlotRecord {
        schema: vibe_workspace::vibedeps::SLOT_RECORD_SCHEMA,
        source_hash: ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
        spec_format: SpecFormat::Mixed,
        converter_recipe: None,
        derived_hash: None,
        overlay_hash: None,
        files: vec![
            vibe_workspace::vibedeps::SlotFile {
                path: "vibe.toml".into(),
                sha256: vibe_workspace::vibedeps::sha256_bytes(&manifest),
                source: None,
                disposition: None,
            },
            vibe_workspace::vibedeps::SlotFile {
                path: specs_path,
                sha256: vibe_workspace::vibedeps::sha256_bytes(body.as_bytes()),
                source: None,
                disposition: None,
            },
        ],
    };
    vibe_workspace::vibedeps::write_slot_record(slot, &record).unwrap();
}
