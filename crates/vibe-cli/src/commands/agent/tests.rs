//! The human views of `vibe agent list` and `vibe agent presets`, pinned
//! at a fixed width (PROP-067 `#list`, `#heading`, `#trailer`, `#width`).
//! Glyphs asserted here: `✗` (a definition with a problem), `└` (the
//! children of a row) and `✻` (the trailer).

use super::*;

const WIDTH: usize = 80;

/// The registry's own refusal for a definition naming an unknown preset
/// (PROP-064 `#agent-definition`), as `vibe_app` writes it into `problem`.
const BROKEN: &str = "agent `broken` cannot be used: preset `nope` is not built in; presets: \
    claude-code, codex, opencode, qwen, hermes, pi, gemini, cursor \
    (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
    fix: give the definition a command and both argument templates, or name a preset)";

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
                "problem": BROKEN,
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

/// `vibe agent list` of the fixture at 80 columns, exactly.
const GOLDEN_80: &str = "\
── launchers ───────────────────────────────────────────────────────────────────
  claude    claude    claude    [built-in]
  codex     codex     codex     [built-in]
  opencode  opencode  opencode  [built-in]
  qwen      qwen      qwen      [built-in]
  hermes    hermes    hermes    [built-in]
  pi        pi        pi        [built-in]
  cc2       claude    cc2       config /u/.claude2  [/u/.vibe/agents.toml]
  starts a launcher with this project's boot lane
  vibe agent <name>

── agent definitions ───────────────────────────────────────────────────────────
  reviewer  preset claude-code  [default]
  └ interactive claude
  └ unattended  claude -p {prompt}
✗ broken
  └ preset `nope` is not built in
  └ fix  give the definition a command and both argument templates, or name a
         preset

── trust ───────────────────────────────────────────────────────────────────────
  trusted    org.vibevm.*          [base]
  trusted    org.example           [user]
  untrusted  org.vibevm.ai-native  overrides base  [user]
  written    org.example           trusted_groups in agents.toml
  package-authored code of a trusted group runs unasked unless an untrusted ent…

✻ 7 launchers · 2 agent definitions · default reviewer · /u/.vibe/agents.toml";

#[test]
fn agent_list_renders_sections_as_measured_lists() {
    let out = render_list(&listing(), WIDTH, WIDTH);
    assert_eq!(out, GOLDEN_80, "\n{out}");
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

    // The kind, command and metadata columns start at one offset in every
    // launcher row.
    for word_at in [1, 2, 3] {
        let offsets: Vec<usize> = lines[1..8]
            .iter()
            .map(|line| word_offset(line, word_at))
            .collect();
        assert!(offsets.iter().all(|at| *at == offsets[0]), "{out}");
    }

    // A broken definition is the `✗` row and two children: what is wrong
    // without the repeated lead or the preset roster, and the fix; no spec
    // address.
    let broken = lines
        .iter()
        .position(|line| *line == "✗ broken")
        .expect("the broken row");
    let block: Vec<&str> = lines[broken + 1..]
        .iter()
        .take_while(|line| !line.is_empty())
        .copied()
        .collect();
    assert_eq!(
        block.iter().filter(|line| line.starts_with("  └ ")).count(),
        2,
        "{out}"
    );
    assert!(!out.contains("violates"), "{out}");
    assert!(!out.contains("cannot be used"), "{out}");
    assert!(!out.contains("presets:"), "{out}");

    for line in &lines {
        assert!(
            console::measure_text_width(line) <= WIDTH,
            "{line:?} runs past {WIDTH}\n{out}"
        );
    }
}

/// The display column where the `n`th (0-based) whitespace-separated word
/// of `line` starts.
fn word_offset(line: &str, n: usize) -> usize {
    let mut column = 0;
    let mut words = 0;
    let mut in_word = false;
    for c in line.chars() {
        if c != ' ' && !in_word {
            if words == n {
                return column;
            }
            words += 1;
        }
        in_word = c != ' ';
        column += console::measure_text_width(c.encode_utf8(&mut [0; 4]));
    }
    panic!("no word {n} in {line:?}")
}

#[test]
fn at_forty_columns_a_launchers_metadata_moves_to_its_own_line() {
    let out = render_list(&listing(), 40, 40);
    let lines: Vec<&str> = out.lines().collect();
    let cc2 = lines
        .iter()
        .position(|line| line.starts_with("  cc2 "))
        .expect("the cc2 row");
    assert_eq!(lines[cc2], "  cc2       claude    cc2", "{out}");
    assert!(
        lines[cc2 + 1] == format!("{}config /u/.claude…", " ".repeat(22)),
        "{out}"
    );
    assert_eq!(lines[1], "  claude    claude    claude", "{out}");
    assert_eq!(lines[2], format!("{}[built-in]", " ".repeat(22)), "{out}");
    // The fix wraps under its own text rather than running past the edge.
    assert!(
        out.contains("  └ fix  give the definition a command\n         and both"),
        "{out}"
    );
    // Every line but the trailer, which writes its path in full.
    assert!(
        lines[..lines.len() - 1]
            .iter()
            .all(|line| console::measure_text_width(line) <= 40),
        "{out}"
    );
}

#[test]
fn a_section_without_rows_says_none() {
    let mut listing = listing();
    listing.agents.clear();
    listing.default = None;
    let out = render_list(&listing, WIDTH, WIDTH);
    assert!(out.contains("── agent definitions ─"), "{out}");
    assert!(out.contains("─\n  none\n\n── trust ─"), "{out}");
    assert!(out.ends_with("0 agent definitions · default none · /u/.vibe/agents.toml"));
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
        narrow.contains("  starts a launcher with this project's…\n  vibe agent <name>\n"),
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
    assert!(
        wide.contains(
            "  └ fix  give the definition a command and both argument templates, or name a preset\n"
        ),
        "{wide}"
    );
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
    assert!(
        lines
            .iter()
            .all(|line| console::measure_text_width(line) <= WIDTH)
    );
}

#[test]
fn agent_edit_ends_with_its_own_trailer() {
    let out = render_edited(&listing(), "added agent myag", WIDTH, WIDTH);
    let (body, last) = out.rsplit_once("\n\n").unwrap();
    assert_eq!(last, "✻ added agent myag · /u/.vibe/agents.toml");
    assert_eq!(body, GOLDEN_80.rsplit_once("\n\n").unwrap().0);
}

#[test]
fn launcher_refusals_carry_codes() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let unknown = launchers::resolve(&settings, &AgentsConfig::default(), "nosuch").unwrap_err();
    let refusal = output::refusal_of(&unknown);
    assert_eq!(refusal.code.as_deref(), Some("launcher_unknown"));
    assert_eq!(refusal.headline, "nosuch is not an agent launcher");
    assert!(
        refusal
            .why
            .as_deref()
            .unwrap_or_default()
            .starts_with("this machine has claude, codex"),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE")
    );

    let config: AgentsConfig =
        toml::from_str("[launchers.bad]\nkind = \"nope\"\ncommand = [\"x\"]\n").unwrap();
    let kind = launchers::resolve_all(&settings, &config).unwrap_err();
    let refusal = output::refusal_of(&kind);
    assert_eq!(refusal.code.as_deref(), Some("launcher_kind_unknown"));
    assert_eq!(
        refusal.headline,
        "[launchers.bad] names kind nope, which this build does not know"
    );
    assert!(
        refusal
            .why
            .as_deref()
            .unwrap_or_default()
            .starts_with("the kinds are claude, codex"),
        "{refusal:?}"
    );
}
