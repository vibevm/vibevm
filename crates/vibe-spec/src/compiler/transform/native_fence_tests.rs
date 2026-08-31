//! R5.5 native-manager boundary fences: exact wire-root ownership and no
//! loader/process/Cargo/filesystem dependency in the three selected cells.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use syn::UseTree;
use syn::visit::{self, Visit};

use super::fence_families::{
    NATIVE_IDENTITY_RULES, NATIVE_MANAGER_RULES, NATIVE_SCHEDULE_RULES, offenders,
};

fn flatten_use(tree: &UseTree, mut prefix: Vec<String>, out: &mut Vec<Vec<String>>) {
    match tree {
        UseTree::Name(name) => {
            prefix.push(name.ident.to_string());
            out.push(prefix);
        }
        UseTree::Rename(rename) => {
            prefix.push(rename.ident.to_string());
            out.push(prefix);
        }
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use(&path.tree, prefix, out);
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use(item, prefix.clone(), out);
            }
        }
        UseTree::Glob(_) => out.push(prefix),
    }
}

#[derive(Default)]
struct Paths(Vec<Vec<String>>);

impl<'ast> Visit<'ast> for Paths {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        self.0.push(
            path.segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect(),
        );
        visit::visit_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        flatten_use(&item.tree, Vec::new(), &mut self.0);
        visit::visit_item_use(self, item);
    }
}

fn compiler_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/compiler")
}

fn production_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "tests") {
                    continue;
                }
                walk(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let stem = path.file_stem().unwrap().to_string_lossy();
                if !stem.contains("test") && !stem.contains("vehicle") && stem != "carriage" {
                    files.push(path);
                }
            }
        }
    }
    let mut files = Vec::new();
    walk(root, &mut files);
    files
}

fn contains_root(path: &[String], root: &[&str]) -> bool {
    path.windows(root.len())
        .any(|window| window.iter().map(String::as_str).eq(root.iter().copied()))
}

#[test]
fn only_the_selected_manager_cell_names_compile_native_reply_and_admission_roots() {
    let compiler = compiler_root();
    let roots = [
        ["vibe_wire", "generated", "native", "e1", "compile_reply"].as_slice(),
        ["vibe_wire", "behaviour", "native_compile"].as_slice(),
    ];
    for root in roots {
        let mut owners = BTreeSet::new();
        for file in production_files(&compiler) {
            let source = std::fs::read_to_string(&file).unwrap();
            let syntax = syn::parse_file(&source).unwrap();
            let mut paths = Paths::default();
            paths.visit_file(&syntax);
            if paths.0.iter().any(|path| contains_root(path, root)) {
                owners.insert(
                    file.strip_prefix(&compiler)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
        assert_eq!(
            owners,
            BTreeSet::from(["transform/native_manager.rs".to_string()]),
            "compile-native root `{}` has one production owner",
            root.join("::")
        );
    }
}

#[test]
fn native_cells_refuse_loader_process_cargo_and_filesystem_authority() {
    for (cell, source, rules) in [
        (
            "native_identity.rs",
            include_str!("native_identity.rs"),
            &NATIVE_IDENTITY_RULES,
        ),
        (
            "native_manager.rs",
            include_str!("native_manager.rs"),
            &NATIVE_MANAGER_RULES,
        ),
        (
            "native_schedule.rs",
            include_str!("native_schedule.rs"),
            &NATIVE_SCHEDULE_RULES,
        ),
    ] {
        let found = offenders(source, rules);
        assert!(found.is_empty(), "{cell}: {found:?}");
        for mutant in [
            "use crate::native_loader::NativeLoader;",
            "use std::process::Command;",
            "fn f() { std::fs::read(\"x\").ok(); }",
            "fn f(c: Cargo) {}",
        ] {
            assert!(!offenders(mutant, rules).is_empty(), "{mutant}");
        }
    }
}

#[test]
fn vibe_spec_manifest_keeps_the_native_manager_below_loader_and_lifecycle() {
    let manifest =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .unwrap();
    let parsed = manifest.parse::<toml::Table>().unwrap();
    let dependencies = parsed["dependencies"].as_table().unwrap();
    for forbidden in [
        "vibe-native-loader",
        "vibe-lifecycle",
        "vibe-workspace",
        "vibe-orchestrator",
    ] {
        assert!(
            !dependencies.contains_key(forbidden),
            "vibe-spec must stay below `{forbidden}`"
        );
    }
}
