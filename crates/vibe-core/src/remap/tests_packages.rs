//! The acceptance ledger and the derived provider map
//! (`PROP-066#consent`, `PROP-066#providers`).
//!
//! The properties asserted here are the ones the whole consent model rests
//! on: a question is the default, an answer is bound to the bytes it was
//! given about, switching a package off is not the same as forgetting it,
//! and a row two packages both offer is one row with two providers — which
//! is what decides whether an uninstall orphans it.

use specmark::verifies;

use super::*;

/// One offer with one `registry` row.
fn offer(key: &str, hash: &str, target: &str) -> RemapPackage {
    let decl: RowDecl =
        toml::from_str(&format!("registry = \"vibespecs\"\nurl = \"{target}\"")).unwrap();
    RemapPackage {
        key: key.to_owned(),
        version: "1.0.0".to_owned(),
        content_hash: hash.to_owned(),
        section: RemapSection {
            description: Some(format!("the network {key} is for")),
            priority: 0,
            rows: vec![decl],
        },
        policy: PolicySection::default(),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
fn an_unanswered_offer_is_the_default_and_asks() {
    let ledger = ConsentLedger::default();
    assert_eq!(
        ledger.state("org.corp/intranet", "sha256:a"),
        OfferState::Offered
    );
    assert!(ledger.state("org.corp/intranet", "sha256:a").asks());
    assert!(!ledger.state("org.corp/intranet", "sha256:a").applies());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
fn a_new_content_hash_of_an_accepted_package_is_a_new_offer() {
    let mut ledger = ConsentLedger::default();
    ledger.accept("org.corp/intranet", "sha256:a", "2026-11-01T00:00:00Z");
    assert!(ledger.state("org.corp/intranet", "sha256:a").applies());
    // The user agreed to the rows they were shown. Different bytes are a
    // different question, and inheriting the yes is exactly the silent
    // redirection the PROP exists to prevent.
    assert_eq!(
        ledger.state("org.corp/intranet", "sha256:b"),
        OfferState::Offered
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
fn a_rejection_is_remembered_so_the_question_is_not_asked_again() {
    let mut ledger = ConsentLedger::default();
    ledger.reject("org.corp/intranet", "sha256:a", "2026-11-01T00:00:00Z");
    let state = ledger.state("org.corp/intranet", "sha256:a");
    assert_eq!(state, OfferState::Rejected);
    assert!(!state.asks(), "a refusal is an answer, not a silence");
    assert!(!state.applies());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
fn disabling_keeps_the_acceptance_and_enabling_restores_it() {
    let mut ledger = ConsentLedger::default();
    ledger.accept("org.corp/intranet", "sha256:a", "2026-11-01T00:00:00Z");
    assert!(ledger.set_enabled("org.corp/intranet", false));
    let off = ledger.state("org.corp/intranet", "sha256:a");
    assert_eq!(off, OfferState::Disabled);
    assert!(!off.applies(), "a disabled package's rows do not apply");
    assert!(!off.asks(), "and the question is not asked again either");
    assert!(ledger.set_enabled("org.corp/intranet", true));
    assert!(ledger.state("org.corp/intranet", "sha256:a").applies());
    // Nothing is recorded about a package the home does not have.
    assert!(!ledger.set_enabled("org.corp/elsewhere", true));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
fn the_ledger_round_trips_through_its_file_and_a_missing_one_is_empty() {
    let home = tempfile::tempdir().unwrap();
    // A home nobody has answered in reads as an empty ledger, not an
    // error: an absent file is exactly "nothing answered yet".
    let fresh = ConsentLedger::load(home.path()).unwrap();
    assert!(fresh.packages.is_empty());

    let mut ledger = fresh;
    ledger.accept("org.corp/intranet", "sha256:a", "2026-11-01T00:00:00Z");
    ledger.reject("org.corp/other", "sha256:b", "2026-11-01T00:00:01Z");
    ledger.store(home.path()).unwrap();
    assert!(consent_path(home.path()).is_file());

    let read = ConsentLedger::load(home.path()).unwrap();
    assert_eq!(read, ledger);
    assert_eq!(read.protocol, CONSENT_PROTOCOL);
    assert!(read.state("org.corp/intranet", "sha256:a").applies());
    assert_eq!(
        read.state("org.corp/other", "sha256:b"),
        OfferState::Rejected
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#uninstall")]
fn forgetting_a_package_turns_its_offer_back_into_a_question() {
    let mut ledger = ConsentLedger::default();
    ledger.accept("org.corp/intranet", "sha256:a", "2026-11-01T00:00:00Z");
    assert!(ledger.forget("org.corp/intranet"));
    assert_eq!(
        ledger.state("org.corp/intranet", "sha256:a"),
        OfferState::Offered,
        "the acceptance is deleted with the package"
    );
    assert!(!ledger.forget("org.corp/intranet"), "and only once");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#providers")]
fn two_packages_offering_the_same_row_are_two_providers_of_one_row() {
    let packages = [
        offer(
            "org.corp/a",
            "sha256:a",
            "https://nexus.corp.example/vibespecs",
        ),
        offer(
            "org.corp/b",
            "sha256:b",
            "https://nexus.corp.example/vibespecs",
        ),
    ];
    let mut ledger = ConsentLedger::default();
    ledger.accept("org.corp/a", "sha256:a", "2026-11-01T00:00:00Z");
    ledger.accept("org.corp/b", "sha256:b", "2026-11-01T00:00:00Z");

    let map = ProviderMap::derive(&packages, &ledger);
    assert_eq!(map.rows.len(), 1, "key, target and scope are equal");
    assert_eq!(
        map.rows[0].providers,
        ["org.corp/a@1.0.0", "org.corp/b@1.0.0"]
    );
    assert!(map.rows[0].is_live());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#providers")]
fn a_row_is_live_only_while_an_accepted_package_offers_it() {
    let packages = [offer(
        "org.corp/a",
        "sha256:a",
        "https://nexus.corp.example/vibespecs",
    )];
    // Installed but unanswered: the row is known and not in force.
    let offered = ProviderMap::derive(&packages, &ConsentLedger::default());
    assert_eq!(offered.rows.len(), 1);
    assert!(offered.rows[0].accepted_providers.is_empty());
    assert!(!offered.rows[0].is_live());

    let mut ledger = ConsentLedger::default();
    ledger.accept("org.corp/a", "sha256:a", "2026-11-01T00:00:00Z");
    assert!(ProviderMap::derive(&packages, &ledger).rows[0].is_live());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#uninstall")]
fn removal_orphans_only_the_rows_no_other_accepted_package_offers() {
    let shared = "https://nexus.corp.example/vibespecs";
    let mut a = offer("org.corp/a", "sha256:a", shared);
    let b = offer("org.corp/b", "sha256:b", shared);
    // `a` carries a second row nobody else has.
    let only_mine: RowDecl =
        toml::from_str("prefix = \"https://github.com/\"\npath = \"/srv/gh\"").unwrap();
    a.section.rows.push(only_mine);
    let packages = [a, b];
    let mut ledger = ConsentLedger::default();
    ledger.accept("org.corp/a", "sha256:a", "2026-11-01T00:00:00Z");
    ledger.accept("org.corp/b", "sha256:b", "2026-11-01T00:00:00Z");

    let map = ProviderMap::derive(&packages, &ledger);
    let (orphaned, surviving) = map.split_on_removal("org.corp/a@1.0.0");
    assert_eq!(orphaned.len(), 1, "{orphaned:?}");
    assert_eq!(orphaned[0].kind, "prefix");
    assert_eq!(surviving.len(), 1, "{surviving:?}");
    assert_eq!(surviving[0].to, shared);

    // Removing `b` orphans nothing: `a` still offers the shared row.
    let (orphaned, surviving) = map.split_on_removal("org.corp/b@1.0.0");
    assert!(orphaned.is_empty());
    assert_eq!(surviving.len(), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#providers")]
fn the_map_round_trips_through_its_file_and_a_missing_one_is_empty() {
    let home = tempfile::tempdir().unwrap();
    assert!(ProviderMap::load(home.path()).rows.is_empty());
    let packages = [offer("org.corp/a", "sha256:a", "https://nexus/vibespecs")];
    let mut ledger = ConsentLedger::default();
    ledger.accept("org.corp/a", "sha256:a", "2026-11-01T00:00:00Z");
    let map = ProviderMap::derive(&packages, &ledger);
    map.store(home.path()).unwrap();
    assert!(providers_path(home.path()).is_file());
    assert_eq!(ProviderMap::load(home.path()), map);
    assert_eq!(map.protocol, PROVIDERS_PROTOCOL);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#package")]
fn a_packages_rows_carry_the_package_as_their_provenance() {
    let package = offer("org.corp/a", "sha256:a", "https://nexus/vibespecs");
    let loaded = package.loaded(Layer::UserPackage).unwrap();
    assert_eq!(loaded.layer, Layer::UserPackage);
    assert_eq!(loaded.source, "org.corp/a@1.0.0");
    assert_eq!(loaded.rows[0].from.source, "org.corp/a@1.0.0");
    assert_eq!(loaded.rows[0].from.layer, Layer::UserPackage);
    // A malformed row names its position the way a remap file's does.
    let mut broken = package;
    broken.section.rows[0].deny = Some(true);
    let error = broken.loaded(Layer::UserPackage).unwrap_err();
    assert!(error.to_string().contains("row 1"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#package")]
fn the_installed_index_holds_one_version_per_key_in_priority_order() {
    let mut index = InstalledRemaps::default();
    let record = |key: &str, priority: i64| {
        let mut package = offer(key, &format!("sha256:{key}"), "https://nexus/vibespecs");
        package.section.priority = priority;
        InstalledRemap {
            package,
            registry: None,
            installed: "2026-11-01T00:00:00Z".to_owned(),
        }
    };
    index.upsert(record("org.corp/low", 0));
    index.upsert(record("org.corp/high", 10));
    index.upsert(record("org.corp/low", 0));
    assert_eq!(index.packages.len(), 2, "one version per key");
    // Higher `priority` wins among accepted packages, and the layer's
    // tie-break is the later row — so the highest priority is loaded last.
    let order: Vec<String> = index.offers().into_iter().map(|p| p.key).collect();
    assert_eq!(order, ["org.corp/low", "org.corp/high"]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#uninstall")]
fn a_kept_row_is_written_as_a_local_table_naming_the_package_it_outlived() {
    let kept = KeptRow {
        identity: RowIdentity {
            kind: "sha256".into(),
            what: "ab".repeat(32),
            to: "/srv/artifacts".into(),
            scope_package: Some("org.example/tool".into()),
            scope_step: Some("fetch".into()),
        },
        reason: Some("the air-gapped artifact store".into()),
        from: "org.corp/intranet@1.0.0".into(),
    };
    let written = kept.to_toml().unwrap();
    // The row has to parse back as a local-layer row, `from` and all:
    // a settings file vibe wrote and then could not read would be the
    // worst possible outcome of keeping a row.
    let file: RemapFileDecl = toml::from_str(&written).unwrap();
    assert_eq!(file.remap.len(), 1);
    let row = file.remap[0]
        .clone()
        .into_row(Provenance::new(Layer::Local, "~/.vibe/settings.toml", 0))
        .unwrap();
    assert_eq!(row.key.kind(), "sha256");
    assert!(matches!(row.target, RemapTarget::Path(_)));
    assert_eq!(row.from_package.as_deref(), Some("org.corp/intranet@1.0.0"));
    assert_eq!(row.reason.as_deref(), Some("the air-gapped artifact store"));
    let scope = row.scope.expect("the scope survives the copy");
    assert_eq!(scope.step.as_deref(), Some("fetch"));

    // A `deny` row keeps its refusal rather than becoming a path.
    let denying = KeptRow {
        identity: RowIdentity {
            kind: "registry".into(),
            what: "vibespecs".into(),
            to: "deny".into(),
            scope_package: None,
            scope_step: None,
        },
        reason: None,
        from: "org.corp/intranet@1.0.0".into(),
    };
    let file: RemapFileDecl = toml::from_str(&denying.to_toml().unwrap()).unwrap();
    assert_eq!(file.remap[0].deny, Some(true));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#policy")]
fn the_strictest_policy_of_the_accepted_packages_applies() {
    let mut strict = offer("org.corp/strict", "sha256:a", "https://nexus/vibespecs");
    strict.policy.unmatched = UnmatchedPolicy::Deny;
    let lenient = offer("org.corp/open", "sha256:b", "https://nexus/vibespecs");
    assert_eq!(
        strictest_policy(std::slice::from_ref(&lenient)),
        UnmatchedPolicy::Allow
    );
    assert_eq!(
        strictest_policy(&[lenient, strict]),
        UnmatchedPolicy::Deny,
        "one `deny` makes the home an allowlist"
    );
}
