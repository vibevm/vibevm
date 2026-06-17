//! `vibe vars` — reconcile the actual runtime variable context with the
//! environment (PROP-019 §2.14). A script reads this to learn the real
//! context (the values vibevm derives from `current_exe`) even when a shell's
//! `$VIBEVM_HOME` is stale. The publish token is deliberately never included.

specmark::scope!("spec://vibevm/common/PROP-019#vars");

use anyhow::{Result, bail};

use crate::cli::VarsArgs;

/// One variable: the value vibevm actually uses, and the raw environment
/// value (PROP-019 §2.14).
pub struct VarRow {
    pub name: &'static str,
    pub actual: String,
    pub env: Option<String>,
}

impl VarRow {
    fn differs(&self) -> bool {
        matches!(&self.env, Some(e) if e != &self.actual)
    }
}

pub fn run(args: VarsArgs, rows: Vec<VarRow>) -> Result<()> {
    let mut full = false;
    let mut diff = false;
    for m in &args.modes {
        match m.as_str() {
            "full" => full = true,
            "diff" => diff = true,
            other => bail!("unknown `vibe vars` mode `{other}` (want `full` and/or `diff`)"),
        }
    }
    print!("{}", render(&rows, full, diff));
    Ok(())
}

/// Render the variable map in the requested shape (PROP-019 §2.14).
fn render(rows: &[VarRow], full: bool, diff: bool) -> String {
    let mut out = String::new();
    if full {
        out.push_str("# ACTUAL\n");
        for r in rows {
            let mark = if diff && r.differs() { " [*]" } else { "" };
            out.push_str(&format!("{}={}{mark}\n", r.name, r.actual));
        }
        out.push_str("\n# ENVIRONMENT\n");
        for r in rows {
            let mark = if diff && r.differs() { " [*]" } else { "" };
            out.push_str(&format!(
                "{}={}{mark}\n",
                r.name,
                r.env.as_deref().unwrap_or("")
            ));
        }
    } else {
        for r in rows {
            if diff && r.differs() {
                out.push_str(&format!(
                    "{}={} [{}]\n",
                    r.name,
                    r.actual,
                    r.env.as_deref().unwrap_or("")
                ));
            } else {
                out.push_str(&format!("{}={}\n", r.name, r.actual));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    fn rows() -> Vec<VarRow> {
        vec![
            VarRow {
                name: "VIBEVM_HOME",
                actual: "/opt/vibevm/versions/branch/main/2".into(),
                env: Some("/old".into()),
            },
            VarRow {
                name: "VIBE_LOG",
                actual: "warn".into(),
                env: None,
            },
        ]
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#vars", r = 1)]
    fn plain_prints_actual_values() {
        let out = render(&rows(), false, false);
        assert_eq!(
            out,
            "VIBEVM_HOME=/opt/vibevm/versions/branch/main/2\nVIBE_LOG=warn\n"
        );
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#vars", r = 1)]
    fn diff_brackets_only_differences() {
        let out = render(&rows(), false, true);
        assert!(out.contains("VIBEVM_HOME=/opt/vibevm/versions/branch/main/2 [/old]"));
        // VIBE_LOG has no env override → no bracket.
        assert!(out.contains("VIBE_LOG=warn\n"));
        assert!(!out.contains("VIBE_LOG=warn ["));
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#vars", r = 1)]
    fn full_diff_has_two_tables_and_stars_differences() {
        let out = render(&rows(), true, true);
        assert!(out.contains("# ACTUAL\n"));
        assert!(out.contains("\n# ENVIRONMENT\n"));
        assert!(out.contains("VIBEVM_HOME=/opt/vibevm/versions/branch/main/2 [*]"));
        assert!(out.contains("VIBEVM_HOME=/old [*]"));
        // The non-differing row carries no star in either table.
        assert!(out.contains("VIBE_LOG=warn\n"));
    }
}
