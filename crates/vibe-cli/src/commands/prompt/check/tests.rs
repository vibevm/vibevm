//! `vibe prompt check` as the display language draws it (PROP-067 `#list`,
//! `#refusal`; PROP-076 `#SYS-READ-ONLY-CHECK`).

use super::*;
use std::path::PathBuf;
use vibe_workspace::system_prompt::{ArtifactFacts, CheckVerdict, PromptCheck};

fn present(name: &str, len: usize, sha: &str) -> ArtifactFacts {
    ArtifactFacts {
        path: PathBuf::from("/p/vibevm/vibespecs/boot").join(name),
        present: true,
        len: Some(len),
        sha256: Some(sha.to_owned()),
    }
}

fn answer(verdict: CheckVerdict) -> PromptCheck {
    PromptCheck {
        node_rel: ".".into(),
        opted_in: true,
        verdict,
        system: present("SYSTEM.md", 1209, "0adfc0c6c3cc0000"),
        meta: present("SYSTEM.meta.json", 1305, "0a767ccad2970000"),
        lane: ArtifactFacts {
            path: PathBuf::from("/p/vibevm/vibespecs/boot/AGENT-LANE.md"),
            present: false,
            len: None,
            sha256: None,
        },
        warnings: Vec::new(),
    }
}

#[test]
fn prompt_check_refusal_reads_as_prompt_stale() {
    let answer = answer(CheckVerdict::Stale(vec![
        "the template input `vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md` changed since the prefix was generated".into(),
    ]));
    let refusal = crate::output::refusal_of(&anyhow::anyhow!(refusal_message(&answer)));
    assert_eq!(refusal.code.as_deref(), Some("prompt_stale"));
    assert_eq!(refusal.headline, "the system prompt of . is stale");
    assert_eq!(refusal.fix, ["vibe prompt build"]);
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK")
    );
    assert!(
        refusal
            .why
            .unwrap_or_default()
            .contains("changed since the prefix")
    );
}

#[test]
fn prompt_check_artifacts_are_measured_rows() {
    let answer = answer(CheckVerdict::Equal);
    let rows = artifact_list([&answer.system, &answer.meta, &answer.lane]).render_at(100);
    // The metadata column starts after the widest name and the two-column gap.
    let expected = format!(
        "  {:<18}1209 bytes · 0adfc0c6c3cc\n  {:<18}1305 bytes · 0a767ccad297\n  {:<18}[absent]",
        "SYSTEM.md", "SYSTEM.meta.json", "AGENT-LANE.md"
    );
    assert_eq!(rows, expected);
}

#[test]
fn prompt_check_path_warning_names_the_build() {
    let warning = crate::commands::prompt::path::stale_warning(
        "SYSTEM.md",
        "stale",
        "the template input changed",
        "vibe prompt build",
    );
    assert_eq!(
        warning,
        "! SYSTEM.md is stale: the template input changed\n  vibe prompt build"
    );
}

#[test]
fn prompt_check_all_non_equal_refusals_keep_the_reason_and_node_remedy() {
    for verdict in [
        CheckVerdict::Stale(vec!["first reason".into(), "second reason".into()]),
        CheckVerdict::Missing(vec!["missing artifact".into()]),
        CheckVerdict::Conflict("ownership conflict".into()),
        CheckVerdict::Unavailable("unreadable input".into()),
        CheckVerdict::FreshnessUnverifiable("compile needed".into()),
    ] {
        let mut answer = answer(verdict);
        answer.node_rel = "members/worker".into();
        let refusal = crate::output::refusal_of(&anyhow::anyhow!(refusal_message(&answer)));
        assert_eq!(
            refusal.code,
            Some(format!("prompt_{}", answer.verdict.as_str()))
        );
        assert_eq!(
            refusal.headline,
            format!(
                "the system prompt of members/worker is {}",
                answer.verdict.as_str().replace('_', " ")
            )
        );
        assert_eq!(refusal.fix, [answer.remedy()]);
        assert_eq!(refusal.why, Some(answer.verdict.reasons().join("; ")));
        assert_eq!(
            refusal.see.as_deref(),
            Some("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK")
        );
    }
    let empty = answer(CheckVerdict::Missing(Vec::new()));
    assert!(refusal_message(&empty).ends_with("`vibe prompt build`)"));
}
