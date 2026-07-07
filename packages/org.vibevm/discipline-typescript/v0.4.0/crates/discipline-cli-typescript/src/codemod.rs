//! `discipline-typescript codemod add-cell` — the scaffold-I frame:
//! one parameterized, checked, atomic operation. Creates a cell under
//! the policy's `cells_dir` — the seam (`index.ts`) carrying a
//! file-level `@scope` marker plus a `node:test` smoke test — then
//! post-checks by RUNNING the new cell's tests and rolls the whole
//! cell back on failure. All-or-nothing, so a half-created cell never
//! lands (the Rust twin's `cargo check` post-check, TS-projected).

use std::path::Path;

use anyhow::{Context, Result, bail};

fn seam_source(cell: &str, spec_uri: &str) -> String {
    format!(
        "/** @scope {spec_uri} */\n\
         export function {cell}(): string {{\n\
         \x20 return \"{cell}\";\n\
         }}\n"
    )
}

fn smoke_test_source(cell: &str) -> String {
    format!(
        "import {{ test }} from \"node:test\";\n\
         import assert from \"node:assert/strict\";\n\
         import {{ {cell} }} from \"./index.js\";\n\
         \n\
         test(\"{cell} cell constructs\", () => {{\n\
         \x20 assert.equal(typeof {cell}(), \"string\");\n\
         }});\n"
    )
}

pub fn run_codemod_add_cell(root: &Path, cell: &str, spec_uri: &str) -> Result<()> {
    if !cell
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        || cell.is_empty()
    {
        bail!("codemod add-cell: cell name `{cell}` must be lowercase kebab/snake");
    }
    specmark_grammar_check(spec_uri)?;
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let Some(cells_dir) = config.typescript.cells_dir.clone() else {
        bail!(
            "codemod add-cell: no `cells_dir` in conform.toml [typescript] — set it \
             (e.g. cells_dir = \"src/cells\") so the codemod knows where cells live"
        );
    };
    let seam = &config.typescript.seam;
    let dir = root.join(&cells_dir).join(cell);
    if dir.exists() {
        bail!("codemod add-cell: `{cells_dir}/{cell}` already exists");
    }
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let write = |name: &str, body: String| -> Result<()> {
        std::fs::write(dir.join(name), body)
            .with_context(|| format!("writing {}/{name}", dir.display()))
    };
    let created = || -> Result<()> {
        write(
            &format!("{seam}.ts"),
            seam_source(&cell.replace('-', "_"), spec_uri),
        )?;
        write(
            &format!("{seam}.test.ts"),
            smoke_test_source(&cell.replace('-', "_")),
        )?;
        Ok(())
    };
    if let Err(e) = created() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }

    // Post-check: the new cell's own fast loop must be green from birth.
    let mut cmd = crate::tools::node_command(root);
    cmd.args(["--test", "--test-reporter=tap"]);
    cmd.args(crate::tools::test_globs(&format!("{cells_dir}/{cell}")));
    let out = cmd.output()?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&dir);
        bail!(
            "codemod add-cell: the new cell's smoke test is RED — rolled back. \
             node said:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    eprintln!(
        "codemod add-cell: `{cells_dir}/{cell}` created ({seam}.ts with @scope \
         {spec_uri} + smoke test) and its fast loop is green."
    );
    Ok(())
}

fn specmark_grammar_check(uri: &str) -> Result<()> {
    specmark_grammar::parse_spec_uri(uri)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("codemod add-cell: `{uri}` is not a spec URI: {e}"))
}
