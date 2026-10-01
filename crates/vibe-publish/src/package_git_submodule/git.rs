//! Bounded local-only raw Git object reader for a held submodule Git dir.
use std::io::{self, BufReader, Read};
use std::path::Path;
use std::process::ChildStdout;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use crate::package_git_plan::GitObjectHasher;
use crate::package_git_tree::git::LocalGit;

pub(crate) struct RawWitness {
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

struct HashedRead<'a> {
    inner: BufReader<&'a mut ChildStdout>,
    expected: u64,
    seen: u64,
    sha256: Sha256,
    oid: GitObjectHasher,
}

impl Read for HashedRead<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.seen == self.expected {
            let mut extra = [0_u8; 1];
            if self.inner.read(&mut extra)? != 0 {
                return Err(io::Error::other("Git raw object grew beyond declared size"));
            }
            return Ok(0);
        }
        let remaining = usize::try_from(self.expected - self.seen).unwrap_or(usize::MAX);
        let wanted = out.len().min(64 * 1024).min(remaining);
        let n = self.inner.read(&mut out[..wanted])?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Git raw object truncated",
            ));
        }
        self.seen += n as u64;
        self.sha256.update(&out[..n]);
        self.oid.update(&out[..n]);
        Ok(n)
    }
}

impl LocalGit<'_> {
    /// The callback must consume the complete object. Its reader allocates a
    /// fixed 64 KiB buffer and hashes Git framing and raw bytes as delivered.
    pub(crate) fn with_raw_object<T>(
        &self,
        kind: &str,
        oid: &str,
        max_bytes: u64,
        consume: impl FnOnce(&mut dyn Read) -> Result<T>,
    ) -> Result<(T, RawWitness)> {
        self.oid(format!("{oid}\n").as_bytes())?;
        if self.output(&["cat-file", "-t", oid], &[], 32)? != format!("{kind}\n").into_bytes() {
            bail!("Git submodule object has wrong type");
        }
        let size_row = self.output(&["cat-file", "-s", oid], &[], 32)?;
        let size = std::str::from_utf8(&size_row)?
            .strip_suffix('\n')
            .context("Git object size row malformed")?
            .parse::<u64>()?;
        if size > max_bytes {
            bail!("Git submodule object exceeds host-selected cap");
        }
        let oid_hash = GitObjectHasher::new(kind, size, self.oid_len)?;
        let mut child = self.command(&["cat-file", kind, oid], true)?.spawn()?;
        drop(child.stdin.take());
        let Some(mut stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Git submodule object stream absent");
        };
        let mut reader = HashedRead {
            inner: BufReader::with_capacity(64 * 1024, &mut stdout),
            expected: size,
            seen: 0,
            sha256: Sha256::new(),
            oid: oid_hash,
        };
        let value = match consume(&mut reader) {
            Ok(value) => value,
            Err(error) => {
                drop(reader);
                drop(stdout);
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let mut extra = [0_u8; 1];
        let end = match reader.read(&mut extra) {
            Ok(value) => value,
            Err(error) => {
                drop(reader);
                drop(stdout);
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        };
        if end != 0 {
            drop(reader);
            drop(stdout);
            let _ = child.kill();
            let _ = child.wait();
            bail!("Git object consumer stopped before complete raw bytes");
        }
        if reader.seen != size {
            drop(reader);
            drop(stdout);
            let _ = child.kill();
            let _ = child.wait();
            bail!("Git raw object short despite end-of-stream");
        }
        let HashedRead {
            inner,
            sha256,
            oid: oid_hash,
            ..
        } = reader;
        drop(inner);
        let calculated = oid_hash.finish();
        let sha256 = format!("sha256:{:x}", sha256.finalize());
        drop(stdout);
        if !child.wait()?.success() {
            bail!("Git raw object child failed");
        }
        let named = Project::open(self.objects.root_path())?;
        if named.root_correlation_token()? != self.objects.root_correlation_token()? {
            bail!("submodule Git-dir pathname changed after raw object read");
        }
        if calculated != oid {
            bail!("raw Git object bytes do not match selected OID");
        }
        Ok((value, RawWitness { size, sha256 }))
    }
}

pub(super) fn trusted_binary(binary: &Path) -> bool {
    binary.is_absolute()
        && std::fs::symlink_metadata(binary)
            .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
}
