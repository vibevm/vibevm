//! The snapshot layout, the filtered envelope and the priming that lets a
//! network-denied client render (PROP-075 `#MIRROR-SNAPSHOT-LAYOUT`).
use super::*;

fn group() -> Group {
    Group::parse("org.vibevm.node").unwrap()
}

/// Upstream's shape: one record per version, newest first, as
/// `nodejs.org/dist/index.json` writes it.
fn upstream() -> CachedVersionList {
    let document = serde_json::json!([
        {"version": "v24.22.2", "lts": false},
        {"version": "v22.23.4", "lts": "Jod"},
        {"version": "v20.1.0", "lts": false},
    ])
    .to_string();
    CachedVersionList::of(
        "https://nodejs.org/dist/index.json",
        document,
        1_800_000_000,
    )
}

fn record(now: &str) -> SnapshotRecord {
    SnapshotRecord::new(
        "org.vibevm.node/node".into(),
        "1.0.0".into(),
        Some("https://nodejs.org/dist/index.json".into()),
        "https://nodejs.org/dist/v{version}/SHASUMS256.txt".into(),
        "vibe 1.0.11".into(),
        now.into(),
    )
}

/// Write a whole snapshot into a registry tree and hand back its root.
fn written(versions: &[(&str, &str)]) -> tempfile::TempDir {
    let registry = tempfile::tempdir().unwrap();
    let dir = registry.path().join(relative_dir(&group(), "node"));
    std::fs::create_dir_all(dir.join("digests")).unwrap();
    let mut snapshot = record("2026-11-10T09:00:00Z");
    let kept: Vec<semver::Version> = versions
        .iter()
        .map(|(version, _)| version.parse().unwrap())
        .collect();
    let envelope = filtered_version_list(&upstream(), "", "version", Some("v"), &kept).unwrap();
    std::fs::write(
        dir.join("versions.json"),
        serde_json::to_string(&envelope).unwrap(),
    )
    .unwrap();
    snapshot.merge(
        versions
            .iter()
            .map(|(version, digests)| {
                std::fs::write(dir.join("digests").join(version), digests).unwrap();
                SnapshotVersion {
                    version: (*version).to_string(),
                    digest_sha256: digest_of(digests.as_bytes()),
                    rendered_sha256: format!("rendered-{version}"),
                }
            })
            .collect(),
    );
    std::fs::write(dir.join("snapshot.toml"), snapshot.render().unwrap()).unwrap();
    registry
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-SNAPSHOT-LAYOUT")]
fn the_three_paths_are_under_the_registry_index() {
    let group = group();
    assert_eq!(
        relative_record(&group, "node"),
        "_index/templates/org.vibevm.node/node/snapshot.toml"
    );
    assert_eq!(
        relative_version_list(&group, "node"),
        "_index/templates/org.vibevm.node/node/versions.json"
    );
    assert_eq!(
        relative_digest(&group, "node", &"24.22.2".parse().unwrap()),
        "_index/templates/org.vibevm.node/node/digests/24.22.2"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-SNAPSHOT-LAYOUT")]
fn the_filtered_envelope_keeps_the_records_and_recomputes_its_digest() {
    let upstream = upstream();
    let keep = vec!["24.22.2".parse().unwrap(), "22.23.4".parse().unwrap()];
    let filtered = filtered_version_list(&upstream, "", "version", Some("v"), &keep).unwrap();

    // The record array is restricted and nothing else is altered: the
    // kept records are byte-verbatim, with their own `lts` fields.
    let records: Vec<serde_json::Value> = serde_json::from_str(&filtered.document).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["version"], "v24.22.2");
    assert_eq!(records[1]["lts"], "Jod");

    // The envelope's own provenance is carried over…
    assert_eq!(filtered.source_url, upstream.source_url);
    assert_eq!(filtered.fetched_at, upstream.fetched_at);
    // …and its digest is of the FILTERED bytes, never the upstream ones.
    assert_ne!(filtered.sha256, upstream.sha256);
    assert_eq!(filtered.sha256, digest_of(filtered.document.as_bytes()));
}

#[test]
fn a_selector_pointing_into_the_document_filters_the_same_way() {
    let document = serde_json::json!({"releases": [
        {"v": "1.0.0"}, {"v": "2.0.0"}
    ]})
    .to_string();
    let upstream = CachedVersionList::of("https://e.invalid/i.json", document, 1_800_000_000);
    let keep = vec!["2.0.0".parse().unwrap()];
    let filtered = filtered_version_list(&upstream, "/releases", "v", None, &keep).unwrap();
    let document: serde_json::Value = serde_json::from_str(&filtered.document).unwrap();
    assert_eq!(document["releases"].as_array().unwrap().len(), 1);
    assert_eq!(document["releases"][0]["v"], "2.0.0");
}

#[test]
fn a_selector_that_points_at_no_array_leaves_the_document_alone() {
    let upstream = CachedVersionList::of(
        "https://e.invalid/i.json",
        serde_json::json!({"not": "an array"}).to_string(),
        1_800_000_000,
    );
    let filtered = filtered_version_list(&upstream, "/nope", "version", None, &[]).unwrap();
    let document: serde_json::Value = serde_json::from_str(&filtered.document).unwrap();
    assert_eq!(document["not"], "an array");
}

#[test]
fn a_record_is_absent_when_the_registry_carries_no_snapshot() {
    let registry = tempfile::tempdir().unwrap();
    assert!(
        Snapshot::read(registry.path(), &group(), "node")
            .unwrap()
            .is_none()
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-SNAPSHOT-LAYOUT")]
fn a_snapshot_round_trips_and_reports_its_sync_time() {
    let registry = written(&[("24.22.2", "abc  node-v24.22.2.tar.gz\n")]);
    let snapshot = Snapshot::read(registry.path(), &group(), "node")
        .unwrap()
        .expect("the snapshot is there");
    assert_eq!(snapshot.record.schema, SCHEMA);
    assert_eq!(snapshot.record.source.package, "org.vibevm.node/node");
    assert_eq!(snapshot.record.source.revision, "1.0.0");
    assert_eq!(
        snapshot.record.source.digest_url,
        "https://nodejs.org/dist/v{version}/SHASUMS256.txt"
    );
    assert_eq!(snapshot.synced(), "2026-11-10T09:00:00Z");
    assert_eq!(
        snapshot
            .record
            .version(&"24.22.2".parse().unwrap())
            .unwrap()
            .rendered_sha256,
        "rendered-24.22.2"
    );
    assert!(snapshot.record.version(&"1.0.0".parse().unwrap()).is_none());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-TEMPLATE-SNAPSHOT"
)]
fn priming_fills_the_cache_so_an_offline_reader_finds_every_input() {
    let registry = written(&[
        ("24.22.2", "aa  node-v24.22.2.tar.gz\n"),
        ("22.23.4", "bb  node-v22.23.4.tar.gz\n"),
    ]);
    let cache_root = tempfile::tempdir().unwrap();
    let cache = TemplateCache::at(cache_root.path(), &group(), "node");
    let snapshot = Snapshot::read(registry.path(), &group(), "node")
        .unwrap()
        .unwrap();

    // Three files: the version list and one digest document per version.
    assert_eq!(snapshot.prime(&cache).unwrap(), 3);
    let list = cache.read_version_list().unwrap().unwrap();
    assert_eq!(list.sha256, digest_of(list.document.as_bytes()));
    let records: Vec<serde_json::Value> = serde_json::from_str(&list.document).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(
        cache
            .read_digest_document(&"22.23.4".parse().unwrap())
            .unwrap()
            .as_deref(),
        Some("bb  node-v22.23.4.tar.gz\n")
    );

    // Priming twice places nothing: an existing entry is left alone.
    assert_eq!(snapshot.prime(&cache).unwrap(), 0);
}

#[test]
fn priming_never_overwrites_what_the_cache_already_holds() {
    let registry = written(&[("24.22.2", "snapshot-bytes\n")]);
    let cache_root = tempfile::tempdir().unwrap();
    let cache = TemplateCache::at(cache_root.path(), &group(), "node");
    let version = "24.22.2".parse().unwrap();
    cache
        .write_digest_document(&version, "local-bytes\n")
        .unwrap();
    let snapshot = Snapshot::read(registry.path(), &group(), "node")
        .unwrap()
        .unwrap();
    snapshot.prime(&cache).unwrap();
    // The local copy stands: a digest document is immutable, so the two
    // cannot honestly differ, and overwriting would erase the evidence
    // the changed-digest alarm exists to raise.
    assert_eq!(
        cache.read_digest_document(&version).unwrap().as_deref(),
        Some("local-bytes\n")
    );
}

#[test]
fn a_digest_file_that_does_not_hash_to_its_row_is_not_primed() {
    let registry = written(&[("24.22.2", "aa  node.tar.gz\n")]);
    let dir = registry.path().join(relative_dir(&group(), "node"));
    std::fs::write(dir.join("digests/24.22.2"), "tampered\n").unwrap();
    let cache_root = tempfile::tempdir().unwrap();
    let cache = TemplateCache::at(cache_root.path(), &group(), "node");
    let snapshot = Snapshot::read(registry.path(), &group(), "node")
        .unwrap()
        .unwrap();
    // The version list still primes; the tampered digest does not.
    assert_eq!(snapshot.prime(&cache).unwrap(), 1);
    assert!(
        cache
            .read_digest_document(&"24.22.2".parse().unwrap())
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_row_whose_digest_file_the_mirror_never_wrote_is_a_miss_not_a_refusal() {
    let registry = written(&[("24.22.2", "aa  node.tar.gz\n")]);
    std::fs::remove_file(
        registry
            .path()
            .join(relative_dir(&group(), "node"))
            .join("digests/24.22.2"),
    )
    .unwrap();
    let cache_root = tempfile::tempdir().unwrap();
    let cache = TemplateCache::at(cache_root.path(), &group(), "node");
    let snapshot = Snapshot::read(registry.path(), &group(), "node")
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.prime(&cache).unwrap(), 1);
}

#[test]
fn an_envelope_whose_sha256_does_not_describe_its_document_is_ignored() {
    let registry = written(&[("24.22.2", "aa  node.tar.gz\n")]);
    let dir = registry.path().join(relative_dir(&group(), "node"));
    let mut envelope: CachedVersionList =
        serde_json::from_str(&std::fs::read_to_string(dir.join("versions.json")).unwrap()).unwrap();
    // Exactly the mistake the recomputation exists to prevent: the
    // records were filtered and the hash was carried over.
    envelope.sha256 = digest_of(b"some other bytes");
    std::fs::write(
        dir.join("versions.json"),
        serde_json::to_string(&envelope).unwrap(),
    )
    .unwrap();
    let snapshot = Snapshot::read(registry.path(), &group(), "node")
        .unwrap()
        .unwrap();
    assert!(snapshot.version_list().unwrap().is_none());
    let cache_root = tempfile::tempdir().unwrap();
    let cache = TemplateCache::at(cache_root.path(), &group(), "node");
    // Only the digest document primes.
    assert_eq!(snapshot.prime(&cache).unwrap(), 1);
    assert!(cache.read_version_list().unwrap().is_none());
}

#[test]
fn a_schema_this_vibe_does_not_read_is_absent_rather_than_fatal() {
    let registry = written(&[("24.22.2", "aa  node.tar.gz\n")]);
    let path = registry
        .path()
        .join(relative_dir(&group(), "node"))
        .join("snapshot.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("schema = 1", "schema = 9")).unwrap();
    assert!(
        Snapshot::read(registry.path(), &group(), "node")
            .unwrap()
            .is_none()
    );

    // So is one that does not parse at all.
    std::fs::write(&path, "this is not toml = = =\n").unwrap();
    assert!(
        Snapshot::read(registry.path(), &group(), "node")
            .unwrap()
            .is_none()
    );
}

#[test]
fn merge_adds_rows_keeps_the_others_and_sorts_by_version() {
    let mut snapshot = record("2026-11-10T09:00:00Z");
    let row = |version: &str| SnapshotVersion {
        version: version.to_string(),
        digest_sha256: format!("d-{version}"),
        rendered_sha256: format!("r-{version}"),
    };
    snapshot.merge(vec![row("22.23.4"), row("24.22.2")]);
    snapshot.merge(vec![row("20.1.0")]);
    assert_eq!(
        snapshot
            .versions
            .iter()
            .map(|row| row.version.as_str())
            .collect::<Vec<_>>(),
        ["20.1.0", "22.23.4", "24.22.2"]
    );
    // A replacement does not duplicate.
    snapshot.merge(vec![SnapshotVersion {
        version: "22.23.4".into(),
        digest_sha256: "moved".into(),
        rendered_sha256: "moved".into(),
    }]);
    assert_eq!(snapshot.versions.len(), 3);
    assert_eq!(snapshot.versions[1].digest_sha256, "moved");
}

/// The golden corpus of `[format.index-templates]`
/// (`formats/corpora/index-templates/e1`) and its **wire-parity oracle**:
/// the writer emits exactly the corpus bytes and the reader round-trips
/// them. An authored golden corpus has no journal to fold, so this is
/// what its integrity stands on besides `wire-diff`'s shift probe.
mod corpus {
    use super::*;

    fn home() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the workspace root is two levels above this crate")
            .join("formats/corpora/index-templates/e1")
    }

    /// The record the corpus's `valid/snapshot.toml` is the bytes of.
    fn representative() -> SnapshotRecord {
        let mut record = SnapshotRecord::new(
            "org.vibevm.node/node".into(),
            "1.0.0".into(),
            Some("https://nodejs.org/dist/index.json".into()),
            "https://nodejs.org/dist/v{version}/SHASUMS256.txt".into(),
            "vibe 1.0.11".into(),
            "2026-11-10T09:00:00Z".into(),
        );
        record.merge(vec![
            SnapshotVersion {
                version: "22.23.4".into(),
                digest_sha256: "c90c3f14".repeat(8),
                rendered_sha256: "11112222".repeat(8),
            },
            SnapshotVersion {
                version: "24.22.2".into(),
                digest_sha256: "aec92ae9".repeat(8),
                rendered_sha256: "33334444".repeat(8),
            },
        ]);
        record
    }

    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-SNAPSHOT-LAYOUT"
    )]
    fn the_writer_emits_the_corpus_bytes_and_the_reader_round_trips_them() {
        let path = home().join("valid/snapshot.toml");
        let golden = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        // Writer → corpus.
        assert_eq!(
            representative().render().unwrap(),
            golden,
            "the writer no longer emits `{}`; regenerate the corpus with the change and \
             declare the shift",
            path.display()
        );
        // Corpus → reader → writer, byte-identical.
        let read: SnapshotRecord = toml::from_str(&golden).unwrap();
        assert_eq!(read, representative());
        assert_eq!(read.render().unwrap(), golden);
    }

    #[test]
    fn the_corpus_envelope_is_the_client_cache_envelope_with_its_own_digest() {
        let path = home().join("valid/versions.json");
        let golden = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let envelope: CachedVersionList = serde_json::from_str(&golden).unwrap();
        assert_eq!(envelope.sha256, digest_of(envelope.document.as_bytes()));
        let records: Vec<serde_json::Value> = serde_json::from_str(&envelope.document).unwrap();
        assert_eq!(records.len(), 2);
        // Round-trips through the writer's own spelling.
        assert_eq!(serde_json::to_string_pretty(&envelope).unwrap(), golden);
    }

    /// Write every corpus file from the writer itself, so the golden
    /// bytes are the writer's own and never a transcription of them.
    pub(super) fn write_corpus() {
        let home = home();
        for sub in ["valid", "semantic-gap", "structural-invalid"] {
            std::fs::create_dir_all(home.join(sub)).unwrap();
        }
        std::fs::write(
            home.join("valid/snapshot.toml"),
            representative().render().unwrap(),
        )
        .unwrap();
        let keep = vec!["24.22.2".parse().unwrap(), "22.23.4".parse().unwrap()];
        let envelope = filtered_version_list(&upstream(), "", "version", Some("v"), &keep).unwrap();
        std::fs::write(
            home.join("valid/versions.json"),
            serde_json::to_string_pretty(&envelope).unwrap(),
        )
        .unwrap();
        std::fs::create_dir_all(home.join("valid/digests")).unwrap();
        std::fs::write(
            home.join("valid/digests/24.22.2"),
            "aec92ae9  node-v24.22.2-linux-x64.tar.gz\n",
        )
        .unwrap();
        let mut unknown = representative();
        unknown.schema = 2;
        std::fs::write(
            home.join("semantic-gap/unknown-schema.toml"),
            unknown.render().unwrap(),
        )
        .unwrap();
        std::fs::write(
            home.join("structural-invalid/not-toml.toml"),
            "# A snapshot a half-finished write left behind.\nschema = = 1\n",
        )
        .unwrap();
    }

    /// The two cases the reader must answer «absent» to rather than fail,
    /// each kept as bytes so a reader change cannot quietly turn one into
    /// a refusal that makes a whole registry unusable.
    #[test]
    fn the_gap_cases_read_as_absent_rather_than_fatal() {
        let registry = tempfile::tempdir().unwrap();
        let dir = registry.path().join(relative_dir(&group(), "node"));
        std::fs::create_dir_all(&dir).unwrap();
        for case in [
            "semantic-gap/unknown-schema.toml",
            "structural-invalid/not-toml.toml",
        ] {
            let bytes = std::fs::read(home().join(case)).unwrap();
            std::fs::write(dir.join("snapshot.toml"), &bytes).unwrap();
            assert!(
                Snapshot::read(registry.path(), &group(), "node")
                    .unwrap()
                    .is_none(),
                "{case} must read as absent"
            );
        }
    }
}

#[test]
#[ignore = "generator: run with --ignored to (re)write the golden corpus bytes"]
fn regenerate_the_golden_corpus() {
    use corpus::*;
    write_corpus();
}
