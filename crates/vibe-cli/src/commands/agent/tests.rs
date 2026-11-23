//! The human views of `vibe agent list` and `vibe agent presets`, pinned
//! at a fixed width (PROP-067 `#list`, `#heading`, `#trailer`, `#width`).
//! Glyphs asserted here: `✗` (a definition with a problem), `└` (the
//! children of a row) and `✻` (the trailer).

use super::*;

const WIDTH: usize = 80;

fn listing() -> AgentList {
    let builtin = |name: &str| {
        serde_json::json!({
            "name": name, "kind": name, "command": [name], "source": "built-in",
        })
    };
    serde_json::from_value(serde_json::json!({
        "protocol": "vibe.agent-list/1",
        "path": "/u/.vibe/agents.toml",
        "default": "reviewer",
        "trusted_groups": ["org.example"],
        "trust": {
            "trusted": [
                { "pattern": "org.vibevm.*", "source": "base" },
                { "pattern": "org.example", "source": "user" },
            ],
            "untrusted": [
                { "pattern": "org.vibevm.ai-native", "source": "user", "overrides": ["base"] },
            ],
        },
        "agents": [
            {
                "name": "reviewer", "preset": "claude-code", "command": "claude",
                "interactive": [], "unattended": ["-p", "{prompt}"], "default": true,
            },
            {
                "name": "broken", "interactive": [], "unattended": [], "default": false,
                "problem": "unknown preset `nope`",
            },
        ],
        "launchers": [
            builtin("claude"), builtin("codex"), builtin("opencode"), builtin("qwen"),
            builtin("hermes"), builtin("pi"),
            {
                "name": "cc2", "kind": "claude", "command": ["cc2"],
                "config_dir": "/u/.claude2", "source": "/u/.vibe/agents.toml",
            },
        ],
    }))
    .unwrap()
}

fn columns_of(line: &str, word: &str) -> usize {
    line.find(word)
        .map(|byte| line[..byte].chars().count())
        .unwrap_or_else(|| panic!("`{word}` in {line:?}"))
}

#[test]
fn agent_list_renders_sections_as_measured_lists() {
    let out = render_list(&listing(), WIDTH, WIDTH);
    let lines: Vec<&str> = out.lines().collect();

    // Sections open with their rules, launchers first.
    let rules: Vec<&str> = lines
        .iter()
        .filter(|line| line.starts_with("── "))
        .copied()
        .collect();
    assert_eq!(rules.len(), 3, "{out}");
    assert!(rules[0].starts_with("── launchers ─"), "{out}");
    assert!(rules[1].starts_with("── agent definitions ─"), "{out}");
    assert!(rules[2].starts_with("── trust ─"), "{out}");
    assert!(!out.contains("external registration"), "{out}");

    // The kind column starts at one offset in every launcher row, and so
    // does the source state word.
    let launcher_rows: Vec<&str> = lines[1..8].to_vec();
    let kinds: Vec<usize> = launcher_rows
        .iter()
        .map(|line| {
            let name = line.split_whitespace().next().unwrap();
            let rest = &line[2 + name.len()..];
            2 + name.len() + (rest.len() - rest.trim_start().len())
        })
        .collect();
    assert!(kinds.iter().all(|kind| *kind == kinds[0]), "{out}");
    let sources: Vec<usize> = launcher_rows
        .iter()
        .map(|line| columns_of(line, "["))
        .collect();
    assert!(sources.iter().all(|at| *at == sources[0]), "{out}");
    assert!(launcher_rows[0].ends_with("[built-in]"), "{out}");
    assert!(
        launcher_rows[6].contains("cc2 · config /u/.claude2"),
        "{out}"
    );
    assert!(
        launcher_rows[6].ends_with("[/u/.vibe/agents.toml]"),
        "{out}"
    );
    assert!(
        lines[8].starts_with("  vibe agent <name> starts one"),
        "{out}"
    );

    // Definitions: `[default]`, the preset, the two argvs as children, and
    // the problem row in the failed gutter.
    assert!(out.contains("preset claude-code  [default]"), "{out}");
    assert!(out.contains("  └ unknown preset `nope`"), "{out}");
    assert!(out.contains("└ interactive claude\n"), "{out}");
    assert!(out.contains("└ unattended  claude -p {prompt}"), "{out}");
    assert!(
        out.lines().any(|line| line.starts_with("✗ broken")),
        "{out}"
    );

    // Trust: one row per entry, the explanation one dim hint.
    assert!(out.contains("org.vibevm.*"), "{out}");
    assert!(
        out.lines()
            .any(|line| line.starts_with("  untrusted") && line.contains("overrides base  [user]")),
        "{out}"
    );
    assert!(
        out.lines()
            .any(|line| line.starts_with("  written") && line.contains("org.example")),
        "{out}"
    );

    // The trailer replaces the summary line; the path in full.
    assert_eq!(
        lines.last().copied(),
        Some("✻ 7 launchers · 2 agent definitions · default reviewer · /u/.vibe/agents.toml"),
    );
    for line in &lines {
        assert!(
            console::measure_text_width(line) <= WIDTH,
            "{line:?} runs past {WIDTH}\n{out}"
        );
    }
}

#[test]
fn a_hint_is_cut_at_the_viewport_and_an_external_registration_has_its_own_rule() {
    let mut listing = listing();
    listing.external = serde_json::from_value(serde_json::json!({
        "command": "my-agent", "args": ["--run"],
    }))
    .unwrap();
    let narrow = render_list(&listing, 40, 40);
    assert!(narrow.contains("── external registration ─"), "{narrow}");
    assert!(
        narrow.contains("VIBE_AGENT_COMMAND  my-agent --run"),
        "{narrow}"
    );
    assert!(
        narrow.contains("  vibe agent <name> starts one with thi…\n"),
        "{narrow}"
    );
    // Every line but the trailer, which writes its path in full.
    let body: Vec<&str> = narrow.lines().collect();
    assert!(
        body[..body.len() - 1]
            .iter()
            .all(|line| console::measure_text_width(line) <= 40),
        "{narrow}"
    );
    // A redirected stream is not cut.
    let wide = render_list(&listing, usize::MAX / 4, WIDTH);
    assert!(wide.contains("with this project's boot lane\n"), "{wide}");
}

#[test]
fn agent_presets_render_as_a_list_with_a_trailer() {
    let table: AgentPresets = serde_json::from_value(serde_json::json!({
        "protocol": "vibe.agent-presets/1",
        "placeholders": ["{prompt}", "{cwd}"],
        "presets": [
            { "name": "claude-code", "harness": "claude", "command": "claude",
              "interactive": [], "unattended": ["-p", "{prompt}"] },
            { "name": "codex", "harness": "codex", "command": "codex",
              "interactive": [], "unattended": ["exec", "{prompt}"] },
        ],
    }))
    .unwrap();
    let out = render_presets(&table, WIDTH);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "  claude-code  claude");
    assert_eq!(lines[1], "  └ interactive claude");
    assert_eq!(lines[2], "  └ unattended  claude -p {prompt}");
    assert_eq!(lines[3], "  codex        codex");
    assert_eq!(
        lines.last().copied(),
        Some("✻ 2 agent presets · placeholders {prompt} {cwd}"),
    );
    assert!(lines.iter().all(|line| line.chars().count() <= WIDTH));
}
