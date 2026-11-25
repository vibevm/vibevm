//! Pure synthetic goldens for the site report's planned and performed steps.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use super::*;
use vibe_wire::generated::self_apply_report::{FileChange, Imported, ProfileInfo, SkipReason};

#[test]
fn self_apply_dry_run_renders_sections() {
    let dry = SelfApplyReport {
        protocol: "vibe-self-apply/1".into(),
        ok: true,
        command: "self:apply".into(),
        action: ApplyAction::DryRun,
        changed: true,
        profile: Some(ProfileInfo {
            name: Some("corp".into()),
            source: "/srv/site/site.toml".into(),
            sha256: format!("c6d1ff6669e2{}", "a".repeat(52)),
            stored: "/u/.vibe/site-profile.toml".into(),
            applied_at: None,
        }),
        files: ["/u/.vibe/site-profile.toml", "/u/.vibe/site-profile.receipt.toml"]
            .into_iter()
            .map(|path| FileChange {
                change: FileChangeKind::Create,
                path: path.into(),
                diff: None,
            })
            .collect(),
        imported: Some(Imported {
            executable: "/srv/vibe".into(),
            selector: "1.0.12".into(),
            reused: false,
            planned: true,
        }),
        px: Some(PxStep {
            needed: false,
            outcome: PxOutcome::NotNeeded,
            store: "/srv/iroot/opt/px".into(),
            asset: None,
            program: None,
            fix: None,
            planned: true,
        }),
        path: Some(PathReport {
            shim_dir: "/srv/iroot/opt/bin".into(),
            planned: Some(PathPlan::WouldSkip),
            target: Some("/u/.bashrc".into()),
            outcome: None,
            was_on_durable_path: None,
            on_durable_path: None,
            consent: None,
            skip_reason: None,
            changed: None,
            activation_hint: None,
            fix_hint: None,
            durable_environment: None,
        }),
        next_steps: vec!["in each project resolved before this profile, move the locked sources onto the site's registries: `vibe update --sources-only`".into()],
        effective: None,
        where_: None,
        warnings: vec![],
    };
    assert_eq!(
        render(&dry, 100, 100),
        "○ site profile corp would be applied · sha256 c6d1ff6669e2 · /srv/site/site.toml\n\
         ○ create  /u/.vibe/site-profile.toml\n\
         ○ create  /u/.vibe/site-profile.receipt.toml\n\
         ○ import  the running vibe as 1.0.12 · /srv/vibe\n\
         ○ PATH    would skip /srv/iroot/opt/bin · /u/.bashrc\n\n  \
         in each project resolved before this profile, move the locked sources onto the site's registries\n  \
         vibe update --sources-only"
    );
    assert_eq!(trailer_what(&dry), None);
    assert_eq!(headline_text(&dry), "site profile corp would be applied");

    let mut applied = dry.clone();
    applied.action = ApplyAction::Apply;
    let imported = applied.imported.as_mut().unwrap();
    imported.selector = "tag:1.0.12#1".into();
    imported.planned = false;
    let px = applied.px.as_mut().unwrap();
    px.outcome = PxOutcome::Missing;
    px.planned = false;
    let path = applied.path.as_mut().unwrap();
    path.planned = None;
    path.target = None;
    path.outcome = Some(PathFixOutcome::Skipped);
    path.skip_reason = Some(SkipReason::ProfileNever);
    applied.next_steps.clear();
    assert_eq!(
        render(&applied, 100, 100),
        "✓ create  /u/.vibe/site-profile.toml\n\
         ✓ create  /u/.vibe/site-profile.receipt.toml\n\
         ✓ import  the running vibe as tag:1.0.12#1 · /srv/vibe\n\
         ✗ px      not installed\n\
         ○ PATH    left as it is · /srv/iroot/opt/bin not on it"
    );
    assert_eq!(trailer_what(&applied), Some("applied site profile corp".into()));
}
