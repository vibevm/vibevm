//! Unit tests for the redirect command family (PROP-002 §2.4.2).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

use super::sync::{build_target_fetch_url, derive_target_token_env, inject_token_into_url};
use super::update::{
    build_redirect_update_commit_msg, compute_updated_redirect_section, diff_redirect_sections,
};
use super::{build_redirect_readme, parse_target_auth};
use crate::cli::RegistryRedirectUpdateArgs;
use std::path::PathBuf;
use vibe_core::manifest::{AuthKind, RedirectSection, RefPolicy};

// -----------------------------------------------------------------
// redirect / redirect-sync helpers (PROP-002 §2.4.2)
// -----------------------------------------------------------------

#[test]
fn parse_target_auth_canonical_spellings() {
    assert!(matches!(parse_target_auth(None).unwrap(), AuthKind::None));
    assert!(matches!(
        parse_target_auth(Some("none")).unwrap(),
        AuthKind::None
    ));
    assert!(matches!(
        parse_target_auth(Some("token-env")).unwrap(),
        AuthKind::TokenEnv
    ));
    assert!(matches!(
        parse_target_auth(Some("credential-helper")).unwrap(),
        AuthKind::CredentialHelper
    ));
    assert!(matches!(
        parse_target_auth(Some("ssh")).unwrap(),
        AuthKind::Ssh
    ));
}

#[test]
fn parse_target_auth_rejects_unknown() {
    let err = parse_target_auth(Some("oauth")).unwrap_err();
    assert!(err.to_string().contains("unknown --target-auth"));
}

#[test]
fn build_redirect_readme_includes_pkgref_and_target() {
    let r = build_redirect_readme(
        "flow:internal-helper",
        "https://gitlab.acme.example/flows/internal-helper",
        None,
    );
    assert!(r.contains("flow:internal-helper"));
    assert!(r.contains("https://gitlab.acme.example/flows/internal-helper"));
    assert!(r.contains("vibe-redirect.toml"));
}

#[test]
fn build_redirect_readme_includes_description_when_present() {
    let r = build_redirect_readme(
        "flow:x",
        "https://example.invalid/x",
        Some("delegated to acme-corp"),
    );
    assert!(r.contains("delegated to acme-corp"));
}

#[test]
fn derive_target_token_env_uppercase_and_underscore() {
    assert_eq!(
        derive_target_token_env("https://gitlab.acme.example/x").as_deref(),
        Some("VIBEVM_TARGET_TOKEN_GITLAB_ACME_EXAMPLE")
    );
    assert_eq!(
        derive_target_token_env("https://gitverse.ru/y").as_deref(),
        Some("VIBEVM_TARGET_TOKEN_GITVERSE_RU")
    );
}

#[test]
fn inject_token_passes_through_ssh_form() {
    let url = "git@github.com:vibespecs/flow-wal.git";
    assert_eq!(inject_token_into_url(url, "secret"), url);
}

#[test]
fn inject_token_skips_already_credentialed_https() {
    let url = "https://existing:cred@github.com/x/y";
    assert_eq!(inject_token_into_url(url, "newtoken"), url);
}

#[test]
fn inject_token_embeds_into_https() {
    let url = "https://github.com/vibespecs/flow-wal.git";
    let out = inject_token_into_url(url, "abc123");
    assert_eq!(
        out,
        "https://x-access-token:abc123@github.com/vibespecs/flow-wal.git"
    );
}

#[test]
fn build_target_fetch_url_none_passes_through() {
    let section = RedirectSection {
        target_url: "https://example.invalid/x".into(),
        ref_policy: RefPolicy::PassThroughTag,
        pinned_ref: None,
        auth: AuthKind::None,
        token_env: None,
        description: None,
    };
    let out = build_target_fetch_url("https://example.invalid/x", &section).unwrap();
    assert_eq!(out, "https://example.invalid/x");
}

#[test]
fn build_target_fetch_url_token_env_demands_var_set() {
    let section = RedirectSection {
        target_url: "https://example.invalid/x".into(),
        ref_policy: RefPolicy::PassThroughTag,
        pinned_ref: None,
        auth: AuthKind::TokenEnv,
        token_env: Some("VIBEVM_TEST_DEFINITELY_UNSET_TOKEN_VAR".into()),
        description: None,
    };
    let err = build_target_fetch_url("https://example.invalid/x", &section).unwrap_err();
    assert!(
        err.to_string()
            .contains("VIBEVM_TEST_DEFINITELY_UNSET_TOKEN_VAR")
    );
}

// -----------------------------------------------------------------
// compute_updated_redirect_section + helpers — partial update for
// `vibe registry redirect-update` (PROP-002 §2.4.2)
// -----------------------------------------------------------------

fn baseline_pass_through() -> RedirectSection {
    RedirectSection {
        target_url: "https://github.com/old/flow-wal".into(),
        ref_policy: RefPolicy::PassThroughTag,
        pinned_ref: None,
        auth: AuthKind::None,
        token_env: None,
        description: Some("old description".into()),
    }
}

fn baseline_pinned() -> RedirectSection {
    RedirectSection {
        target_url: "https://github.com/old/flow-wal".into(),
        ref_policy: RefPolicy::Pinned,
        pinned_ref: Some("v0.3.0".into()),
        auth: AuthKind::None,
        token_env: None,
        description: None,
    }
}

fn empty_update_args() -> RegistryRedirectUpdateArgs {
    RegistryRedirectUpdateArgs {
        pkgref: "flow:wal".into(),
        to: None,
        registry: None,
        ref_policy: None,
        pinned_ref: None,
        target_auth: None,
        target_token_env: None,
        description: None,
        clear_description: false,
        trust_redirect: false,
        resync: false,
        path: PathBuf::from("."),
        dry_run: false,
    }
}

#[test]
fn compute_update_only_description_change_detected() {
    let mut args = empty_update_args();
    args.description = Some("new description".into());
    let (new, changes) = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
    assert_eq!(new.description.as_deref(), Some("new description"));
    assert_eq!(new.target_url, "https://github.com/old/flow-wal");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "description");
    assert!(!changes.iter().any(|c| c.requires_trust()));
}

#[test]
fn compute_update_clear_description_drops_field() {
    let mut args = empty_update_args();
    args.clear_description = true;
    let (new, changes) = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
    assert_eq!(new.description, None);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "description");
    assert_eq!(changes[0].before.as_deref(), Some("old description"));
    assert_eq!(changes[0].after, None);
}

#[test]
fn compute_update_target_url_change_flags_trust() {
    let mut args = empty_update_args();
    args.to = Some("https://forgejo.example/x/y".into());
    let (new, changes) = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
    assert_eq!(new.target_url, "https://forgejo.example/x/y");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "target_url");
    assert!(changes[0].requires_trust());
}

#[test]
fn compute_update_switch_to_pinned_requires_pinned_ref() {
    let mut args = empty_update_args();
    args.ref_policy = Some("pinned".into());
    // No --pinned-ref, no current pinned_ref → reject.
    let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
    assert!(
        err.to_string().contains("requires `--pinned-ref"),
        "expected pinned-ref-required hint, got: {err}"
    );
}

#[test]
fn compute_update_switch_to_pinned_uses_explicit_ref() {
    let mut args = empty_update_args();
    args.ref_policy = Some("pinned".into());
    args.pinned_ref = Some("v1.2.3".into());
    let (new, changes) = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
    assert!(matches!(new.ref_policy, RefPolicy::Pinned));
    assert_eq!(new.pinned_ref.as_deref(), Some("v1.2.3"));
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().all(|c| c.requires_trust()));
}

#[test]
fn compute_update_switch_to_pass_through_clears_pinned_ref() {
    let mut args = empty_update_args();
    args.ref_policy = Some("pass-through-tag".into());
    let (new, changes) = compute_updated_redirect_section(&baseline_pinned(), &args).unwrap();
    assert!(matches!(new.ref_policy, RefPolicy::PassThroughTag));
    assert_eq!(new.pinned_ref, None);
    // Two changes: ref_policy + pinned_ref (was Some, now None).
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().all(|c| c.requires_trust()));
}

#[test]
fn compute_update_pinned_ref_alone_on_pinned_stub() {
    let mut args = empty_update_args();
    args.pinned_ref = Some("v0.4.0".into());
    // Current is pinned with v0.3.0 — flag bumps to v0.4.0 without
    // touching policy.
    let (new, changes) = compute_updated_redirect_section(&baseline_pinned(), &args).unwrap();
    assert!(matches!(new.ref_policy, RefPolicy::Pinned));
    assert_eq!(new.pinned_ref.as_deref(), Some("v0.4.0"));
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "pinned_ref");
    assert!(changes[0].requires_trust());
}

#[test]
fn compute_update_rejects_pinned_ref_on_pass_through() {
    let mut args = empty_update_args();
    args.pinned_ref = Some("v1.0.0".into());
    let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
    assert!(err.to_string().contains("--pinned-ref is only meaningful"));
}

#[test]
fn compute_update_auth_flip_clears_token_env() {
    let with_token = RedirectSection {
        target_url: "https://x/y".into(),
        ref_policy: RefPolicy::PassThroughTag,
        pinned_ref: None,
        auth: AuthKind::TokenEnv,
        token_env: Some("VIBEVM_TARGET_TOKEN_X".into()),
        description: None,
    };
    let mut args = empty_update_args();
    args.target_auth = Some("none".into());
    let (new, changes) = compute_updated_redirect_section(&with_token, &args).unwrap();
    assert!(matches!(new.auth, AuthKind::None));
    assert_eq!(new.token_env, None);
    // Both auth and token_env appear in the diff — operator-side
    // metadata, not trust-required.
    assert!(changes.iter().any(|c| c.field == "auth"));
    assert!(changes.iter().any(|c| c.field == "token_env"));
    assert!(!changes.iter().any(|c| c.requires_trust()));
}

#[test]
fn compute_update_rejects_token_env_without_matching_auth() {
    let mut args = empty_update_args();
    args.target_token_env = Some("WHATEVER".into());
    // Current auth is None; flag not provided → token_env not
    // meaningful.
    let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
    assert!(
        err.to_string()
            .contains("--target-token-env is only meaningful")
    );
}

#[test]
fn compute_update_rejects_empty_to() {
    let mut args = empty_update_args();
    args.to = Some("   ".into());
    let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
    assert!(err.to_string().contains("--to must be a non-empty"));
}

#[test]
fn compute_update_no_op_returns_empty_changes() {
    let args = empty_update_args();
    let (new, changes) = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
    assert_eq!(new, baseline_pass_through());
    assert!(changes.is_empty());
}

#[test]
fn diff_redirect_sections_emits_field_names_in_canonical_order() {
    let before = baseline_pass_through();
    let mut after = baseline_pass_through();
    after.target_url = "new".into();
    after.description = Some("new".into());
    let changes = diff_redirect_sections(&before, &after);
    // Canonical iteration order: target_url, ref_policy, pinned_ref,
    // auth, token_env, description. With two fields touched the
    // order must be target_url first, description last.
    assert_eq!(changes[0].field, "target_url");
    assert_eq!(changes[1].field, "description");
}

#[test]
fn redirect_update_commit_msg_highlights_target_url_change() {
    let changes = vec![
        super::RedirectChangeEntry {
            field: "target_url",
            before: Some("https://old/x".into()),
            after: Some("https://new/x".into()),
        },
        super::RedirectChangeEntry {
            field: "description",
            before: None,
            after: Some("delegated".into()),
        },
    ];
    let msg = build_redirect_update_commit_msg("flow:wal", &changes);
    assert!(msg.contains("retarget flow:wal"));
    assert!(msg.contains("https://new/x"));
}

#[test]
fn redirect_update_commit_msg_lists_fields_when_no_target_change() {
    let changes = vec![
        super::RedirectChangeEntry {
            field: "auth",
            before: Some("none".into()),
            after: Some("token-env".into()),
        },
        super::RedirectChangeEntry {
            field: "token_env",
            before: None,
            after: Some("VAR".into()),
        },
    ];
    let msg = build_redirect_update_commit_msg("flow:wal", &changes);
    assert!(msg.contains("update marker for flow:wal"));
    assert!(msg.contains("auth, token_env"));
}
