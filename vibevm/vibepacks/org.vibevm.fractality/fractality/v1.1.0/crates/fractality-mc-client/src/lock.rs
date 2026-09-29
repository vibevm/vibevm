//! The mission-control lockfile (`<home>/mc.lock`, plan D10).
//!
//! The daemon binds `127.0.0.1:0` and publishes where it landed:
//! `{schema, port, pid, bearer, started_ts_ms}` as TOML. The bearer is
//! required on every API call (defense against other local users); it
//! rotates on every daemon start, which is safe because every consumer
//! re-reads the lockfile on reconnect.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// On-disk shape of `mc.lock`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    /// Lockfile schema; this build writes and reads `1`.
    pub schema: u32,
    pub port: u16,
    pub pid: u32,
    pub bearer: String,
    pub started_ts_ms: u64,
}

impl Lockfile {
    pub const FILE_NAME: &'static str = "mc.lock";

    pub fn path(home: &Utf8Path) -> Utf8PathBuf {
        home.join(Self::FILE_NAME)
    }

    /// Reads the lockfile; `Ok(None)` when absent.
    pub fn read(home: &Utf8Path) -> Result<Option<Self>, String> {
        let path = Self::path(home);
        let text = match std::fs::read_to_string(path.as_std_path()) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("reading `{path}`: {e}")),
        };
        let lock: Lockfile = toml::from_str(&text).map_err(|e| format!("parsing `{path}`: {e}"))?;
        if lock.schema != 1 {
            return Err(format!(
                "lockfile `{path}` speaks schema {} but this build speaks 1; \
                 versions of fractality binaries on this machine diverge",
                lock.schema
            ));
        }
        Ok(Some(lock))
    }

    /// Writes the lockfile. On Unix the file is chmod 0600; on Windows the
    /// user-profile ACL is the boundary for v0.1 (hardening: DEF-10).
    pub fn write(&self, home: &Utf8Path) -> Result<(), String> {
        std::fs::create_dir_all(home.as_std_path())
            .map_err(|e| format!("creating `{home}`: {e}"))?;
        let path = Self::path(home);
        let text = toml::to_string_pretty(self).map_err(|e| format!("rendering lockfile: {e}"))?;
        std::fs::write(path.as_std_path(), text).map_err(|e| format!("writing `{path}`: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                path.as_std_path(),
                std::fs::Permissions::from_mode(0o600),
            );
        }
        Ok(())
    }

    /// Removes the lockfile; missing is fine (idempotent stop).
    pub fn remove(home: &Utf8Path) -> Result<(), String> {
        let path = Self::path(home);
        match std::fs::remove_file(path.as_std_path()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("removing `{path}`: {e}")),
        }
    }
}

/// Mints a fresh bearer: two ULIDs back to back (~160 bits of entropy
/// from the stack's own RNG — no extra dependency for a localhost
/// same-user boundary).
pub fn mint_bearer() -> String {
    format!("{}{}", ulid::Ulid::new(), ulid::Ulid::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_home(tag: &str) -> Utf8PathBuf {
        let dir = std::env::temp_dir().join(format!("fractality-lock-{tag}-{}", ulid::Ulid::new()));
        Utf8PathBuf::from_path_buf(dir).expect("temp dir is utf-8")
    }

    #[test]
    fn lockfile_round_trips_and_removes_idempotently() {
        let home = scratch_home("rt");
        let lock = Lockfile {
            schema: 1,
            port: 51234,
            pid: 4242,
            bearer: mint_bearer(),
            started_ts_ms: 1_751_800_000_000,
        };
        lock.write(&home).expect("writes");
        let back = Lockfile::read(&home).expect("reads").expect("present");
        assert_eq!(lock, back);
        Lockfile::remove(&home).expect("removes");
        assert!(Lockfile::read(&home).expect("reads again").is_none());
        Lockfile::remove(&home).expect("second remove is fine");
        std::fs::remove_dir_all(home.as_std_path()).ok();
    }

    #[test]
    fn foreign_schema_is_a_loud_error() {
        let home = scratch_home("schema");
        std::fs::create_dir_all(home.as_std_path()).expect("mkdir");
        std::fs::write(
            Lockfile::path(&home).as_std_path(),
            "schema = 2\nport = 1\npid = 1\nbearer = \"b\"\nstarted_ts_ms = 1\n",
        )
        .expect("write");
        let err = Lockfile::read(&home).expect_err("schema 2 must refuse");
        assert!(err.contains("schema 2"));
        std::fs::remove_dir_all(home.as_std_path()).ok();
    }

    #[test]
    fn bearers_are_long_and_unique() {
        let a = mint_bearer();
        let b = mint_bearer();
        assert_eq!(a.len(), 52);
        assert_ne!(a, b);
    }
}
