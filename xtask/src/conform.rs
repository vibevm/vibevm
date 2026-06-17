//! `cargo xtask conform …` — the Phase 4 conformance-engine gate
//! (ENGINE-CONFORM §5): fact extraction through the content-addressed
//! store, rules-as-queries, SARIF, and the ratchet baseline.

use anyhow::{Context, Result, bail};

use crate::repo_root;

/// The crates under the Class-F/G conform gates (the
/// expand-as-you-conform list — a crate enters when its seams are
/// doctested and its error layer carries REQ edges). vibe-core and
/// vibe-index entered 2026-06-12 (SHRINK-PLAN v0.2's opening move,
/// the re-judge the DBT-0019 closure unblocked); vibe-index was
/// pre-paid by Phase 5's PackageScanner seam work.
///
/// conform-frontend-rust and env-audit joined via CONVERT-PLAN v0.1
/// Phase 0 (0.7): both were audit-verified clean (the frontend's
/// `unwrap` hits are `unwrap_or` combinators and test code; env-audit's
/// two live only in tests), so each flip froze nothing — the v0.2
/// invariant that a flip never widens the baseline holds.
pub(crate) const CONFORM_GATED: &[&str] = &[
    "vibe-core",
    "vibe-index",
    "vibe-install",
    "vibe-resolver",
    "conform-core",
    "conform-frontend-rust",
    "specmap-core",
    "vibe-registry",
    "vibe-workspace",
    "vibe-check",
    "vibe-publish",
    "env-audit",
    "vibe-cli",
    "specmark",
    "specmark-grammar",
    "vibe-mcp",
];

/// Crates deliberately *outside* `CONFORM_GATED`, each paired with the
/// reason it has not (yet) flipped. Unlike `specmap-ratchet.json`,
/// which records its dispositions, the conform gate kept its
/// exemptions silent — and a silent exemption reads as a bug while a
/// recorded one reads as a decision (CONVERT-PLAN v0.1 §1 item 0.1).
/// This table is the checklist the remaining phases drain;
/// `every_crate_is_gated_or_exempt` keeps it honest against the
/// workspace so a new crate cannot slip in unclassified.
pub(crate) const CONFORM_EXEMPT: &[(&str, &str)] = &[
    (
        "vibe-graph",
        "M0 stub, no code yet — the task-graph runner per VIBEVM-SPEC §5 is unbuilt; \
         nothing to gate until it lands",
    ),
    (
        "vibe-llm",
        "M0 stub, no code yet — providers land in the v1.5 LLM milestone per \
         VIBEVM-SPEC §10.4; nothing to gate until then",
    ),
    (
        "vibe-wire",
        "generated code (JTD-schema codegen output), excluded by PROP-014 §2.3; the \
         generator input under schemas/ is the taggable unit instead",
    ),
    (
        "xtask",
        "internal developer tooling — panics are acceptable at the developer's own \
         console; stays exempt on the record (CONVERT-PLAN Phase 5, 5.3)",
    ),
];

/// Class-G `pub-doctest` activates on the foundation crate first
/// (CONVERT-PLAN v0.1 §2 item 1.4): vibe-core's whole public *type*
/// surface is gated for compiled examples — its pre-existing doc-debt
/// frozen and shrunk from there. This is a strictly wider lens than
/// `seam-has-doctest` (which sees lib.rs items + traits), so it gets its
/// own list and grows crate-by-crate as later phases convert each.
pub(crate) const GATED_PUB_DOCTEST: &[&str] = &[
    "vibe-core",
    // Widened by the Discipline Sweep §1b: these four are already at zero
    // type-coverage gap (the `health` collector's promotion candidates), so
    // gating them locks the coverage in with no drain.
    "conform-core",
    "conform-frontend-rust",
    "env-audit",
    "specmark-grammar",
    // Drained and armed by the Discipline Sweep (P4): vibe-mcp's 27 public
    // types each gained a compiled doctest, so the gate flips at zero gap
    // (§1b/§1c). vibe-cli is deliberately NOT here — it is a bin crate with
    // no lib target, so `cargo test --doc` cannot compile its examples;
    // gating it would enforce uncompiled prose, the opposite of the rule's
    // "teach by a compiled example" contract (Law 2). The fix for vibe-cli is
    // a lib target or visibility tightening — an owner-level call, recorded
    // in the sweep REPORT and terraform/registry/debt.json.
    "vibe-mcp",
];

/// Files where reading the ambient environment is sanctioned (the
/// `ambient-env` rule, R-001 projection — CONVERT-PLAN v0.1 §5.2): the
/// env-promotion composition root, the per-crate config-resolution
/// points (user-config / cache-root / token / git-binary / index-URL
/// discovery), and the surfaces whose job *is* to read or display env.
/// A gated crate reading env in any *other* file fails the gate;
/// extending this list is a deliberate, reviewed decision, exactly like
/// `CONFORM_GATED`. The fn-grain `#[spec(deviates, reason)]` escape
/// remains for an inherent-env domain read in a non-root file (e.g. the
/// resolver's `[activation]` predicate evaluation).
pub(crate) const ENV_ROOTS: &[&str] = &[
    "crates/vibe-cli/src/main.rs",
    "crates/vibe-cli/src/output.rs",
    "crates/vibe-cli/src/commands/show/config.rs",
    "crates/vibe-core/src/user_config.rs",
    "crates/vibe-index/src/scanner/git_cli.rs",
    "crates/vibe-publish/src/post_hook.rs",
    "crates/vibe-publish/src/token.rs",
    "crates/vibe-registry/src/git_backend/shell.rs",
    "crates/vibe-registry/src/git_package_registry/mod.rs",
    "crates/vibe-registry/src/index_client.rs",
    "crates/vibe-registry/src/registry_cache.rs",
];

/// The standing rule set, constructed in one place so `conform check`
/// and `conform freeze` can never drift apart.
struct ConformRules {
    flag_sites: conform_core::rules::FlagSites,
    isolation: conform_core::rules::CellIsolation,
    unsafe_gate: conform_core::rules::UnsafeGate,
    seam_doctests: conform_core::rules::SeamHasDoctest,
    pub_doctest: conform_core::rules::PubDoctest,
    err_req: conform_core::rules::ErrorEnumCitesReq,
    cell_oracle: conform_core::rules::CellHasOracle,
    err_msg: conform_core::rules::ErrorMessageCitesReq,
    file_len: conform_core::rules::FileLength,
    no_unwrap: conform_core::rules::NoUnwrapInDomain,
    ambient_env: conform_core::rules::AmbientEnv,
}

impl ConformRules {
    fn new() -> Self {
        use conform_core::rules;
        Self {
            flag_sites: rules::FlagSites {
                registry_file: "crates/vibe-cli/src/registry.rs",
                gated_crate: "vibe-cli",
            },
            isolation: rules::CellIsolation,
            // The AUD-0016 posture (owner-directed via SHRINK-PLAN
            // v0.2, 2026-06-12): env-audit is THE designated audit
            // crate — it owns the workspace's env-mutation unsafety
            // behind a safe serialized guard. Production boundaries
            // that cannot move (startup promotion, FFI) testify via
            // fn-grain #[spec(deviates, reason)], which the v5 facts
            // see. The list still grows only by owner decision.
            unsafe_gate: rules::UnsafeGate {
                audit_crates: &["env-audit"],
            },
            seam_doctests: rules::SeamHasDoctest {
                gated_crates: CONFORM_GATED,
            },
            pub_doctest: rules::PubDoctest {
                gated_crates: GATED_PUB_DOCTEST,
            },
            err_req: rules::ErrorEnumCitesReq {
                gated_crates: CONFORM_GATED,
            },
            // Class D (adopt-v0.3 Phase 4): self-scoping — gates
            // exactly the crates that declare #[cell] manifests.
            cell_oracle: rules::CellHasOracle,
            // The 2026-06-12 depth-program additions (audit finding
            // 2026-06-12-08): the Class-F message-grammar half, the
            // guide §2 file-length budget, and the §6 unwrap ban —
            // each landed ratcheted (pre-existing findings frozen via
            // `conform freeze`, shrink-only from there).
            err_msg: rules::ErrorMessageCitesReq {
                gated_crates: CONFORM_GATED,
            },
            file_len: rules::FileLength { max_lines: 600 },
            no_unwrap: rules::NoUnwrapInDomain {
                gated_crates: CONFORM_GATED,
            },
            // CONVERT-PLAN v0.1 §5.2 — the R-001 projection onto env
            // access: env-audit owns env mutation, ENV_ROOTS records the
            // sanctioned reader files, and an inherent-env domain read
            // elsewhere testifies fn-grain (the resolver's activation
            // predicate eval). Born ratcheted like its budget siblings.
            ambient_env: rules::AmbientEnv {
                gated_crates: CONFORM_GATED,
                audit_crates: &["env-audit"],
                roots: ENV_ROOTS,
            },
        }
    }

    fn refs(&self) -> Vec<&dyn conform_core::Rule> {
        vec![
            &self.flag_sites,
            &self.isolation,
            &self.unsafe_gate,
            &self.seam_doctests,
            &self.pub_doctest,
            &self.err_req,
            &self.cell_oracle,
            &self.err_msg,
            &self.file_len,
            &self.no_unwrap,
            &self.ambient_env,
        ]
    }
}

pub(crate) fn run_conform_check(baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    use conform_core::{
        ExtractionLog, Frontend, Rule, Store, baseline, check, count_by_rule, sarif,
    };
    use conform_frontend_rust::RustFrontend;

    let root = repo_root()?;
    let store = Store::at_repo(&root);
    let mut log = ExtractionLog::default();
    let frontend = RustFrontend;
    let facts = store.extract_workspace(&root, &frontend, &mut log)?;
    eprintln!(
        "xtask conform: extracted {} file(s), {} cached (producer {}-{}).",
        log.extracted.len(),
        log.cached,
        Frontend::id(&frontend),
        Frontend::version(&frontend),
    );

    let rules = ConformRules::new();
    let rule_refs: Vec<&dyn Rule> = rules.refs();

    let findings = check(&rule_refs, &facts, scope);
    let report = sarif::render(&rule_refs, &findings);
    let sarif_path = root.join("target").join("conform").join("report.sarif");
    if let Some(parent) = sarif_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&sarif_path, &report)?;

    let base = baseline::load(&root.join(baseline_rel))?;
    let (new, stale) = baseline::diff(&base, &findings);
    for f in &new {
        eprintln!(
            "  conform: NEW {} {}:{} — {}",
            f.rule, f.file, f.line, f.message
        );
    }
    for fp in &stale {
        eprintln!("  conform: baseline entry no longer fires — prune it: {fp}");
    }
    let counts = count_by_rule(&findings);
    eprintln!(
        "xtask conform check: {} finding(s) in scope {} ({:?}), {} frozen in baseline, {} new; SARIF at {}.",
        findings.len(),
        scope.unwrap_or("<workspace>"),
        counts,
        base.findings.len(),
        new.len(),
        sarif_path
            .strip_prefix(&root)
            .unwrap_or(&sarif_path)
            .display()
    );
    eprintln!(
        "xtask conform: {} crate(s) gated, {} exempt — see CONFORM_EXEMPT for the why of each.",
        CONFORM_GATED.len(),
        CONFORM_EXEMPT.len(),
    );
    if !new.is_empty() {
        bail!("conform: {} new finding(s) against the baseline", new.len());
    }
    Ok(())
}

/// `cargo xtask conform freeze` — rewrite the baseline to the current
/// finding set. The legal moments: a NEW rule landing (its
/// pre-existing findings freeze once), and a re-freeze after work
/// that shrank the set. The diff review is the guard: outside a
/// new-rule landing the file may only shrink.
pub(crate) fn run_conform_freeze(baseline_rel: &str) -> Result<()> {
    use conform_core::{ExtractionLog, Rule, Store, check, count_by_rule};
    use conform_frontend_rust::RustFrontend;

    let root = repo_root()?;
    let store = Store::at_repo(&root);
    let mut log = ExtractionLog::default();
    let frontend = RustFrontend;
    let facts = store.extract_workspace(&root, &frontend, &mut log)?;
    let rules = ConformRules::new();
    let rule_refs: Vec<&dyn Rule> = rules.refs();
    let findings = check(&rule_refs, &facts, None);
    let counts = count_by_rule(&findings);
    let mut fps: Vec<&str> = findings.iter().map(|f| f.fingerprint.as_str()).collect();
    fps.sort_unstable();
    fps.dedup();
    let body = serde_json::json!({ "schema": 1, "findings": fps });
    let mut text = serde_json::to_string_pretty(&body).context("serialising baseline")?;
    text.push('\n');
    let path = root.join(baseline_rel);
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    eprintln!(
        "xtask conform freeze: {} fingerprint(s) frozen ({:?}) at {}.",
        fps.len(),
        counts,
        baseline_rel
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{CONFORM_EXEMPT, CONFORM_GATED};

    /// Every workspace crate is classified exactly once — gated by
    /// `CONFORM_GATED` or exempt-with-a-reason by `CONFORM_EXEMPT`,
    /// never both and never neither. This is the checker that turns
    /// the exemption *table* into an enforced *invariant*: add a crate
    /// and forget to place it, or delete one and leave a phantom entry,
    /// and this fails (CONVERT-PLAN v0.1 §1 item 0.1).
    #[test]
    fn every_crate_is_gated_or_exempt() {
        let gated: BTreeSet<&str> = CONFORM_GATED.iter().copied().collect();
        let exempt: BTreeSet<&str> = CONFORM_EXEMPT.iter().map(|(c, _)| *c).collect();

        assert_eq!(
            gated.len(),
            CONFORM_GATED.len(),
            "CONFORM_GATED carries a duplicate crate name"
        );
        assert_eq!(
            exempt.len(),
            CONFORM_EXEMPT.len(),
            "CONFORM_EXEMPT carries a duplicate crate name"
        );

        let both: Vec<&str> = gated.intersection(&exempt).copied().collect();
        assert!(both.is_empty(), "crates both gated and exempt: {both:?}");

        for (crate_name, reason) in CONFORM_EXEMPT {
            assert!(
                !reason.trim().is_empty(),
                "{crate_name} is exempt without a recorded reason — the one thing this \
                 table exists to forbid"
            );
        }

        // Coverage against the real workspace: every crate dir under
        // `crates/` is named in exactly one set, and every listed name
        // except the workspace-root `xtask` is a real crate (no typos).
        let root = crate::repo_root().expect("repo root");
        let crates_dir = root.join("crates");
        let mut on_disk: BTreeSet<String> = BTreeSet::new();
        for entry in std::fs::read_dir(&crates_dir).expect("read crates/") {
            let entry = entry.expect("dir entry");
            if entry.file_type().expect("file type").is_dir()
                && entry.path().join("Cargo.toml").exists()
            {
                on_disk.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
        for c in &on_disk {
            assert!(
                gated.contains(c.as_str()) || exempt.contains(c.as_str()),
                "crate `{c}` is neither gated nor exempt — classify it in conform.rs"
            );
        }
        for c in gated.union(&exempt) {
            if *c == "xtask" {
                continue; // the tooling crate lives at the workspace root, not under crates/
            }
            assert!(
                on_disk.contains(*c),
                "`{c}` is listed in conform.rs but has no crates/{c} directory — typo?"
            );
        }
    }
}
