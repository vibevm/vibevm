//! `go-ai-native-specmap` — mint/`--check` the PROP-014 index over a
//! Go tree, `--gate` for orphan-coverage-only. Flag-compatible with
//! the sibling specmap binaries so wrappers treat the three uniformly.

use std::path::PathBuf;

use anyhow::Result;

fn main() -> Result<()> {
    let mut check = false;
    let mut gate = false;
    let mut path = PathBuf::from(".");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--gate" => gate = true,
            "--path" => {
                let Some(value) = args.next() else {
                    anyhow::bail!("--path needs a value");
                };
                path = PathBuf::from(value);
            }
            other => anyhow::bail!(
                "unknown flag `{other}` — usage: go-ai-native-specmap [--check | --gate] [--path <dir>]"
            ),
        }
    }
    if gate {
        go_ai_native_specmap::run_gate(&path)
    } else {
        go_ai_native_specmap::run_specmap_go(&path, check)
    }
}
