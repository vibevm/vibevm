//! Generated schema-3 owner and schema-2 retirement sidecar corpus.

use std::path::{Path, PathBuf};

use vibe_wire::generated::format_id::{ForeignParsers, FormatId, UnknownFields};
use vibe_wire::generated::scrape::e2::retirement_checkpoint::RetirementCheckpoint;
use vibe_wire::generated::scrape::e3::transaction_owner::TransactionOwner;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus(name: &str, epoch: &str) -> PathBuf {
    repo_root().join("formats/corpora").join(name).join(epoch)
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| panic!("{} readable: {error}", path.display()))
}

fn without_final_newline(bytes: &[u8]) -> &[u8] {
    bytes
        .strip_suffix(b"\r\n")
        .or_else(|| bytes.strip_suffix(b"\n"))
        .unwrap_or(bytes)
}

#[test]
fn owner_corpus_pins_the_exact_legacy_bytes_and_strict_shape() {
    let root = corpus("scrape-transaction-owner", "e3");
    let authored = read(&root.join("valid/minimal.json"));
    let wire: TransactionOwner = serde_json::from_slice(&authored).expect("owner parses");
    assert_eq!(
        serde_json::to_vec(&wire).unwrap(),
        without_final_newline(&authored),
        "x-wire-order preserves the schema-3 owner sidecar bytes"
    );

    for name in ["unknown.json", "missing.json", "type.json"] {
        assert!(
            serde_json::from_slice::<TransactionOwner>(&read(&root.join("invalid").join(name)))
                .is_err(),
            "{name} must be refused"
        );
    }
}

#[test]
fn retirement_corpus_pins_nested_order_and_every_required_null_key() {
    let root = corpus("scrape-retirement-checkpoint", "e2");
    for name in ["minimal.json", "active-file.json", "directory-entry.json"] {
        let authored = read(&root.join("valid").join(name));
        let wire: RetirementCheckpoint =
            serde_json::from_slice(&authored).unwrap_or_else(|error| panic!("{name}: {error}"));
        let encoded = serde_json::to_vec(&wire).unwrap();
        assert_eq!(
            encoded,
            without_final_newline(&authored),
            "{name}: x-wire-order preserves the schema-2 checkpoint bytes"
        );
    }

    let minimal = read(&root.join("valid/minimal.json"));
    assert!(
        String::from_utf8_lossy(&minimal).contains("\"active\":null"),
        "the absent active intent remains an explicit required-null key"
    );
    let directory = read(&root.join("valid/directory-entry.json"));
    let directory = String::from_utf8_lossy(&directory);
    for key in ["\"sha256\":null", "\"bytes\":null", "\"unix_mode\":null"] {
        assert!(
            directory.contains(key),
            "entry state retains required key {key}"
        );
    }

    for name in [
        "unknown.json",
        "missing-active.json",
        "active-type.json",
        "missing-nullable-state.json",
        "missing-bytes.json",
        "missing-unix-mode.json",
        "nested-unknown.json",
        "bytes-fractional.json",
        "bytes-negative.json",
        "bytes-overflow.json",
    ] {
        assert!(
            serde_json::from_slice::<RetirementCheckpoint>(&read(&root.join("invalid").join(name)))
                .is_err(),
            "{name} must be refused"
        );
    }
}

#[test]
fn registry_exposes_the_two_closed_nonrecoverable_epochs() {
    for (format, id, epoch) in [
        (
            FormatId::ScrapeTransactionOwner,
            "scrape-transaction-owner",
            3,
        ),
        (
            FormatId::ScrapeRetirementCheckpoint,
            "scrape-retirement-checkpoint",
            2,
        ),
    ] {
        assert_eq!(format.id(), id);
        assert_eq!(format.epoch(), epoch);
        assert!(!format.recoverable());
        assert_eq!(format.foreign_parsers(), ForeignParsers::None);
        assert_eq!(format.unknown_fields(), UnknownFields::Deny);
    }
}
