//! `rust-ai-native codemod …` — scaffolded edit operations (discipline
//! card scaffold-i-codemods): a recurring multi-file change offered as
//! one parameterized, checked, atomic operation.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Pure content generation for `codemod add-cell` — split out so the
/// templates are unit-testable without touching a filesystem.
mod add_cell {
    /// `sat_solver` → `SatSolver`.
    pub fn pascal(cell: &str) -> String {
        cell.split('_')
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect()
    }

    pub fn module_source(cell: &str, seam: &str, variant: &str, spec_uri: &str) -> String {
        let ty = pascal(cell);
        format!(
            "//! `{ty}` — the `{variant}` {seam} cell.\n\
             //!\n\
             //! Scaffolded by `cargo codemod add-cell`; the seam\n\
             //! implementation is the author's next edit. The `#[cell]`\n\
             //! manifest and the REQ edge are present from birth so the\n\
             //! selection registry and the specmap see the cell\n\
             //! immediately.\n\
             \n\
             use specmark::{{cell, spec}};\n\
             \n\
             #[cell(seam = \"{seam}\", variant = \"{variant}\")]\n\
             #[spec(implements = \"{spec_uri}\")]\n\
             pub struct {ty};\n"
        )
    }

    pub fn smoke_test_source(crate_ident: &str, cell: &str) -> String {
        let ty = pascal(cell);
        format!(
            "//! Smoke reference for the `{cell}` cell — the seed the\n\
             //! `cell-has-oracle` rule requires; replace with the real\n\
             //! differential/characterization oracle as the cell grows\n\
             //! behavior (card scaffold-d).\n\
             \n\
             use {crate_ident}::{cell}::{ty};\n\
             \n\
             #[test]\n\
             fn {cell}_cell_constructs() {{\n    let _cell = {ty};\n}}\n"
        )
    }

    /// Insert `pub mod <cell>;` into lib.rs at the alphabetical
    /// position inside the first contiguous `pub mod` block. Returns
    /// `None` when no such block exists (the codemod then refuses
    /// rather than guessing).
    pub fn insert_pub_mod(lib_source: &str, cell: &str) -> Option<String> {
        let lines: Vec<&str> = lib_source.lines().collect();
        let first = lines
            .iter()
            .position(|l| l.trim().starts_with("pub mod "))?;
        let mut end = first;
        while end < lines.len() && lines[end].trim().starts_with("pub mod ") {
            end += 1;
        }
        let decl = format!("pub mod {cell};");
        if lines[first..end].iter().any(|l| l.trim() == decl) {
            return None; // already registered — refuse, do not duplicate
        }
        let mut insert_at = end;
        for (i, line) in lines.iter().enumerate().take(end).skip(first) {
            if line.trim() > decl.as_str() {
                insert_at = i;
                break;
            }
        }
        let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        out.insert(insert_at, decl);
        let mut joined = out.join("\n");
        if lib_source.ends_with('\n') {
            joined.push('\n');
        }
        Some(joined)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn pascal_cases_snake_names() {
            assert_eq!(pascal("sat"), "Sat");
            assert_eq!(pascal("sat_solver"), "SatSolver");
        }

        #[test]
        fn module_source_carries_cell_and_spec() {
            let src = module_source("sat", "DepSolver", "sat", "spec://p/d#a");
            assert!(src.contains("#[cell(seam = \"DepSolver\", variant = \"sat\")]"));
            assert!(src.contains("#[spec(implements = \"spec://p/d#a\")]"));
            assert!(src.contains("pub struct Sat;"));
        }

        #[test]
        fn insert_is_alphabetical_and_idempotent_refusing() {
            let lib = "pub mod alpha;\npub mod gamma;\n\npub use alpha::A;\n";
            let out = insert_pub_mod(lib, "beta").unwrap();
            assert_eq!(
                out,
                "pub mod alpha;\npub mod beta;\npub mod gamma;\n\npub use alpha::A;\n"
            );
            assert!(
                insert_pub_mod(&out, "beta").is_none(),
                "duplicate must refuse"
            );
        }

        #[test]
        fn insert_appends_after_block_when_last() {
            let lib = "pub mod alpha;\npub mod beta;\nrest";
            let out = insert_pub_mod(lib, "zeta").unwrap();
            assert!(out.starts_with("pub mod alpha;\npub mod beta;\npub mod zeta;"));
        }
    }
}

/// `cargo codemod add-cell` — the card-I prototype: one checked,
/// atomic, multi-file operation. Writes the module, registers it in
/// lib.rs, seeds the smoke test, then runs `cargo check -p <crate>`;
/// any failure rolls every write back.
pub fn run_codemod_add_cell(
    root: &Path,
    crate_dir_rel: &str,
    cell: &str,
    seam: &str,
    variant: &str,
    spec_uri: &str,
) -> Result<()> {
    let crate_dir = root.join(crate_dir_rel);
    let crate_name = crate_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .context("crate dir has no terminal component")?;
    let crate_ident = crate_name.replace('-', "_");

    if !cell
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        || cell.is_empty()
    {
        bail!("--cell must be snake_case ASCII, got `{cell}`");
    }
    if !spec_uri.starts_with("spec://") {
        bail!("--spec-uri must be a spec:// unit URI, got `{spec_uri}`");
    }

    let lib_path = crate_dir.join("src").join("lib.rs");
    let module_path = crate_dir.join("src").join(format!("{cell}.rs"));
    let test_path = crate_dir.join("tests").join(format!("{cell}_smoke.rs"));

    if module_path.exists() {
        bail!("`{}` already exists — refusing", module_path.display());
    }
    if test_path.exists() {
        bail!("`{}` already exists — refusing", test_path.display());
    }
    let lib_before = std::fs::read_to_string(&lib_path)
        .with_context(|| format!("reading {}", lib_path.display()))?;
    let lib_after = add_cell::insert_pub_mod(&lib_before, cell).with_context(|| {
        format!(
            "no `pub mod` block found in {} (or `{cell}` already registered)",
            lib_path.display()
        )
    })?;

    // All content is computed; now write atomically-by-rollback.
    std::fs::write(
        &module_path,
        add_cell::module_source(cell, seam, variant, spec_uri),
    )?;
    if let Some(parent) = test_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&test_path, add_cell::smoke_test_source(&crate_ident, cell))?;
    std::fs::write(&lib_path, &lib_after)?;

    eprintln!(
        "codemod add-cell: wrote {}, {}, registered in lib.rs — post-check…",
        module_path
            .strip_prefix(root)
            .unwrap_or(&module_path)
            .display(),
        test_path.strip_prefix(root).unwrap_or(&test_path).display(),
    );
    let check = Command::new("cargo")
        .args(["check", "-p", &crate_name, "--all-targets"])
        .current_dir(root)
        .status()
        .context("spawning cargo check")?;
    if !check.success() {
        // Roll back: the operation is all-or-nothing.
        let _ = std::fs::remove_file(&module_path);
        let _ = std::fs::remove_file(&test_path);
        std::fs::write(&lib_path, &lib_before)?;
        bail!("post-check failed — all three writes rolled back; the tree is as before");
    }
    eprintln!(
        "codemod add-cell: ok. Next edits: implement the seam on `{}`, replace the \
         smoke test with the real oracle (card scaffold-d), and run \
         `cargo fast-loop --cell {crate_name}`.",
        add_cell::pascal(cell)
    );
    Ok(())
}
