//! Sanitized local-bare parent fetch and bounded retained closure reads.
use crate::package_git_plan::GitObjectHasher;
use crate::package_git_tree::git::LocalGit;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use vibe_safefs::Project;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReadObjectDigest {
    pub oid: String,
    pub kind: String,
    pub size_bytes: u64,
    pub sha256: String,
}

impl LocalGit<'_> {
    pub(crate) fn private_refs(&self, prefix: &str, cap: usize) -> Result<Vec<(String, String)>> {
        if !prefix.starts_with("refs/package-git-parent/") || prefix.contains("..") {
            bail!("invalid private parent-ref prefix");
        }
        let bytes = self.output(
            &["for-each-ref", "--format=%(refname) %(objectname)", prefix],
            &[],
            cap,
        )?;
        let text = std::str::from_utf8(&bytes)?;
        let mut refs = Vec::new();
        for line in text.lines() {
            let (name, oid) = line.split_once(' ').context("private ref row malformed")?;
            if !name.starts_with(prefix)
                || !matches!(name.strip_prefix(prefix), Some("/main" | "/tag"))
            {
                bail!("foreign private parent ref");
            }
            self.oid(format!("{oid}\n").as_bytes())?;
            if refs.iter().any(|(seen, _)| seen == name) {
                bail!("duplicate private parent ref");
            }
            refs.push((name.to_owned(), oid.to_owned()));
        }
        Ok(refs)
    }

    pub(crate) fn fetch_local_bare(
        &self,
        source: &Path,
        selected_tag: Option<&str>,
        private_prefix: &str,
        cap: usize,
    ) -> Result<()> {
        let path = source
            .to_str()
            .context("local bare source path is not Unicode")?;
        if !source.is_absolute() || !private_prefix.starts_with("refs/package-git-parent/") {
            bail!("unsafe local parent fetch address");
        }
        let main = format!("refs/heads/main:{private_prefix}/main");
        let tag = selected_tag.map(|name| format!("{name}:{private_prefix}/tag"));
        let mut args = vec![
            "fetch",
            "--atomic",
            "--no-tags",
            "--no-write-fetch-head",
            "--no-recurse-submodules",
            "--no-auto-gc",
            "--",
            path,
            main.as_str(),
        ];
        if let Some(tag) = tag.as_deref() {
            args.push(tag);
        }
        self.output(&args, &[], cap)?;
        Ok(())
    }

    pub(crate) fn reachable_oids(
        &self,
        parent: &str,
        max_rows: usize,
        cap: usize,
    ) -> Result<Vec<String>> {
        self.oid(format!("{parent}\n").as_bytes())?;
        let bytes = self.output(
            &[
                "rev-list",
                "--objects",
                "--missing=print",
                "--no-object-names",
                parent,
            ],
            &[],
            cap,
        )?;
        let text = std::str::from_utf8(&bytes)?;
        if !text.is_empty() && !text.ends_with('\n') {
            bail!("reachable object list lacks final LF");
        }
        let mut seen = std::collections::BTreeSet::new();
        for line in text.lines() {
            if line.starts_with('?') {
                bail!("reachable parent graph has a missing object");
            }
            self.oid(format!("{line}\n").as_bytes())?;
            if !seen.insert(line.to_owned()) {
                bail!("duplicate reachable Git object");
            }
            if seen.len() > max_rows {
                bail!("reachable Git object count exceeds host-selected cap");
            }
        }
        if !seen.contains(parent) {
            bail!("reachable object list omitted exact parent");
        }
        Ok(seen.into_iter().collect())
    }

    pub(crate) fn object_digest_streamed(
        &self,
        oid: &str,
        max_bytes: u64,
    ) -> Result<ReadObjectDigest> {
        self.oid(format!("{oid}\n").as_bytes())?;
        let kind_bytes = self.output(&["cat-file", "-t", oid], &[], 32)?;
        let kind = std::str::from_utf8(&kind_bytes)?.trim_end_matches('\n');
        if !matches!(kind, "commit" | "tree" | "blob" | "tag")
            || kind_bytes != format!("{kind}\n").into_bytes()
        {
            bail!("reachable Git object type malformed");
        }
        let size_bytes = self.output(&["cat-file", "-s", oid], &[], 32)?;
        let size = std::str::from_utf8(&size_bytes)?
            .strip_suffix('\n')
            .context("Git object size row malformed")?
            .parse::<u64>()?;
        if size > max_bytes {
            bail!("reachable Git object exceeds host-selected byte cap");
        }
        let mut oid_hash = GitObjectHasher::new(kind, size, self.oid_len)?;
        let mut child = self.command(&["cat-file", kind, oid], true)?.spawn()?;
        drop(child.stdin.take());
        let Some(mut stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            bail!("reachable Git object stream absent");
        };
        let mut sha256 = Sha256::new();
        let mut count = 0_u64;
        let mut window = [0_u8; 64 * 1024];
        loop {
            let read = match stdout.read(&mut window) {
                Ok(value) => value,
                Err(error) => {
                    drop(stdout);
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error.into());
                }
            };
            if read == 0 {
                break;
            }
            count = match count.checked_add(read as u64) {
                Some(value) => value,
                None => {
                    drop(stdout);
                    let _ = child.kill();
                    let _ = child.wait();
                    bail!("Git object byte count overflow");
                }
            };
            if count > size || count > max_bytes {
                drop(stdout);
                let _ = child.kill();
                let _ = child.wait();
                bail!("reachable Git object grew beyond exact size");
            }
            sha256.update(&window[..read]);
            oid_hash.update(&window[..read]);
        }
        if !child.wait()?.success() || count != size {
            bail!("reachable Git object short or failed");
        }
        let named = Project::open(self.objects.root_path())?;
        if named.root_correlation_token()? != self.objects.root_correlation_token()? {
            bail!("Git object-root pathname changed after reachable stream");
        }
        let calculated = oid_hash.finish();
        if calculated != oid {
            bail!("reachable Git object bytes do not hash to selected OID");
        }
        Ok(ReadObjectDigest {
            oid: oid.into(),
            kind: kind.into(),
            size_bytes: size,
            sha256: format!("sha256:{:x}", sha256.finalize()),
        })
    }

    pub(crate) fn commit_tree_oid(&self, oid: &str, cap: usize) -> Result<String> {
        let bytes = self.output(&["cat-file", "commit", oid], &[], cap)?;
        let text = std::str::from_utf8(&bytes)?;
        let first = text.lines().next().context("parent commit is empty")?;
        let tree = first
            .strip_prefix("tree ")
            .context("parent commit lacks tree line")?;
        self.oid(format!("{tree}\n").as_bytes())?;
        Ok(tree.into())
    }
}
