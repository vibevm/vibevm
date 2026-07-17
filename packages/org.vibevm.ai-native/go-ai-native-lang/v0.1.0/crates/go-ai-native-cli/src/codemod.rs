//! `go-ai-native codemod add-cell` — the scaffold-I frame: one
//! parameterized, checked, atomic operation. Creates a cell package
//! under the policy's `cells_dir` — `doc.go` carrying the package doc
//! block with its `//spec:scope` marker, the cell source with a
//! constructor, and a smoke test with an executed Example — then
//! post-checks by RUNNING the new cell's tests and rolls the whole
//! cell back on failure. All-or-nothing, so a half-created cell never
//! lands.

specmark::scope!("spec://go-ai-native-lang/go/GUIDE-AI-NATIVE-GO#scaffolds");

use std::path::Path;

use anyhow::{Context, Result, bail};

fn doc_source(cell: &str, spec_uri: &str) -> String {
    format!(
        "// Package {cell} is a cell of this project's discipline layout:\n\
         // constructed through New, capabilities injected, no ambient state.\n\
         //\n\
         //spec:scope {spec_uri} r=1\n\
         package {cell}\n"
    )
}

fn cell_source(cell: &str, type_name: &str) -> String {
    format!(
        "package {cell}\n\
         \n\
         // {type_name} is the cell's implementation; construct via New.\n\
         type {type_name} struct{{}}\n\
         \n\
         // New is the blessed construction path.\n\
         func New() *{type_name} {{ return &{type_name}{{}} }}\n\
         \n\
         // Name names the cell.\n\
         func (c *{type_name}) Name() string {{ return \"{cell}\" }}\n"
    )
}

fn smoke_test_source(cell: &str, type_name: &str) -> String {
    format!(
        "package {cell}\n\
         \n\
         import (\n\
         \t\"fmt\"\n\
         \t\"testing\"\n\
         )\n\
         \n\
         func TestNewConstructs(t *testing.T) {{\n\
         \tif New().Name() != \"{cell}\" {{\n\
         \t\tt.Fatalf(\"unexpected cell name\")\n\
         \t}}\n\
         }}\n\
         \n\
         func ExampleNew() {{\n\
         \tfmt.Println(New().Name())\n\
         \t// Output: {cell}\n\
         }}\n\
         \n\
         var _ = New // the constructor is the surface (GUIDE §2)\n\
         \n\
         var _ *{type_name} // keep the type name referenced from tests\n"
    )
}

/// `naiveplanner` → `Naiveplanner`: the computed `{Variant}{Seam}`
/// grammar starts from the package name; the author refines it to the
/// canonical casing when the seam lands.
fn type_name_of(cell: &str) -> String {
    let mut chars = cell.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

pub fn run_codemod_add_cell(root: &Path, cell: &str, spec_uri: &str) -> Result<()> {
    if cell.is_empty()
        || !cell
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        bail!(
            "codemod add-cell: cell name `{cell}` must be a lowercase go package \
             name (letters and digits only — no dashes or underscores)"
        );
    }
    specmark_grammar::parse_spec_uri(spec_uri)
        .map_err(|e| anyhow::anyhow!("codemod add-cell: `{spec_uri}` is not a spec URI: {e}"))?;
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let Some(cells_dir) = config.go.cells_dir.clone() else {
        bail!(
            "codemod add-cell: no `cells_dir` in conform.toml [go] — set it \
             (e.g. cells_dir = \"internal/cells\") so the codemod knows where cells live"
        );
    };
    let dir = root.join(&cells_dir).join(cell);
    if dir.exists() {
        bail!("codemod add-cell: `{cells_dir}/{cell}` already exists");
    }
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let type_name = type_name_of(cell);
    let write = |name: &str, body: String| -> Result<()> {
        std::fs::write(dir.join(name), body)
            .with_context(|| format!("writing {}/{name}", dir.display()))
    };
    let created = || -> Result<()> {
        write("doc.go", doc_source(cell, spec_uri))?;
        write(&format!("{cell}.go"), cell_source(cell, &type_name))?;
        write(&format!("{cell}_test.go"), smoke_test_source(cell, &type_name))?;
        Ok(())
    };
    if let Err(e) = created() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }

    // Post-check: the new cell's own fast loop must be green from
    // birth (go test compiles AND runs, Example included).
    let mut cmd = crate::tools::go_command(root);
    cmd.args(["test", &format!("./{cells_dir}/{cell}/...")]);
    let out = cmd.output()?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&dir);
        bail!(
            "codemod add-cell: the new cell's smoke test is RED — rolled back. \
             go said:\n{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    eprintln!(
        "codemod add-cell: `{cells_dir}/{cell}` created (doc.go with //spec:scope \
         {spec_uri}, {cell}.go with New, smoke test + Example) and its fast loop \
         is green."
    );
    Ok(())
}
