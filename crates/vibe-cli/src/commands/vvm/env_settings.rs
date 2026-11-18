//! VIBE_SETTINGS in the existing managed shell environment block.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use super::rc_io::{read_rc_file, rebuild, split_block, write_rc_atomic};
use super::{Persisted, RcFilePersister, Shell, fish_quote, posix_quote};
use anyhow::{Result, bail};
use std::path::Path;

impl RcFilePersister {
    fn settings_line(&self, dir: &Path) -> (&'static str, String) {
        let value = dir.display().to_string();
        match self.shell {
            Shell::Fish => (
                "set -gx VIBE_SETTINGS ",
                format!("set -gx VIBE_SETTINGS {}", fish_quote(&value)),
            ),
            _ => (
                "export VIBE_SETTINGS=",
                format!("export VIBE_SETTINGS={}", posix_quote(&value)),
            ),
        }
    }

    pub(super) fn inspect_settings(&self) -> Result<Option<String>> {
        let (text, _) = read_rc_file(&self.rc_path)?;
        let (_, block, _) = split_block(&text)?;
        let prefix = self.settings_line(Path::new("")).0;
        let lines: Vec<_> = block
            .iter()
            .filter_map(|line| line.strip_prefix(prefix))
            .collect();
        if lines.len() > 1 {
            bail!("duplicate VIBE_SETTINGS assignments in managed environment block");
        }
        // The writer uses this exact quoting grammar; do not execute a shell
        // to interpret user-authored expressions.
        Ok(lines.first().map(|value| {
            let value = value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
                .unwrap_or(value);
            match self.shell {
                Shell::Fish => value.replace("\\'", "'").replace("\\\\", "\\"),
                _ => value.replace("'\"'\"'", "'"),
            }
        }))
    }

    pub(super) fn persist_settings(&self, dir: &Path) -> Result<Persisted> {
        let (prefix, line) = self.settings_line(dir);
        self.upsert(prefix, &line)
    }

    pub(super) fn withdraw_settings(&self, dir: &Path) -> Result<Persisted> {
        let (text, permissions) = read_rc_file(&self.rc_path)?;
        let (pre, mut block, post) = split_block(&text)?;
        let (prefix, expected) = self.settings_line(dir);
        let matches: Vec<_> = block
            .iter()
            .filter(|line| line.starts_with(prefix))
            .collect();
        if matches.len() > 1 {
            bail!("duplicate VIBE_SETTINGS assignments in managed environment block");
        }
        if matches.first().is_none_or(|line| **line != expected) {
            return Ok(Persisted::Unchanged);
        }
        block.retain(|line| line != &expected);
        write_rc_atomic(
            &self.rc_path,
            rebuild(
                &pre,
                &block,
                &post,
                if text.contains("\r\n") { "\r\n" } else { "\n" },
            )
            .as_bytes(),
            permissions,
        )?;
        Ok(Persisted::Changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_roundtrip_preserves_foreign_bytes_and_changed_value() {
        let root = tempfile::tempdir().unwrap();
        let rc = root.path().join(".profile");
        std::fs::write(&rc, "# mine\nexport OTHER=yes\n").unwrap();
        let writer = RcFilePersister::new(rc.clone(), Shell::Posix);
        let home = Path::new("/work/settings's home");
        assert_eq!(writer.persist_settings(home).unwrap(), Persisted::Changed);
        assert_eq!(
            writer.inspect_settings().unwrap().as_deref(),
            Some("/work/settings's home")
        );
        assert_eq!(writer.persist_settings(home).unwrap(), Persisted::Unchanged);
        assert_eq!(
            writer.withdraw_settings(Path::new("/different")).unwrap(),
            Persisted::Unchanged
        );
        assert_eq!(writer.withdraw_settings(home).unwrap(), Persisted::Changed);
        assert_eq!(writer.inspect_settings().unwrap(), None);
        assert!(
            std::fs::read_to_string(rc)
                .unwrap()
                .starts_with("# mine\nexport OTHER=yes\n")
        );
    }
}
