//! Sanitized, local-only Git blob/tree object construction and byte readback.
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_safefs::{Project, StreamOutcome};

use crate::package_git_operation::journal::TreeFile;
use crate::package_git_payload::FileRow;

pub(crate) struct LocalGit<'a> {
    pub(crate) binary: &'a Path,
    pub(crate) objects: &'a Project,
    pub(crate) oid_len: usize,
}

impl<'a> LocalGit<'a> {
    /// Probe SHA-256 support in a disposable repository before the retained
    /// object root is created. An init failure later is partial-state trouble,
    /// never evidence that the host simply lacks this format.
    pub(crate) fn probe_sha256(binary: &Path) -> Result<bool> {
        let scratch = tempfile::tempdir()?;
        let template = scratch.path().join("empty-template");
        std::fs::create_dir(&template)?;
        let mut command = Command::new(binary);
        command
            .env_clear()
            .current_dir(scratch.path())
            .env("LC_ALL", "C")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                scratch.path().join(".no-global-config"),
            )
            .env("GIT_TEMPLATE_DIR", &template)
            .env("HOME", scratch.path())
            .env("USERPROFILE", scratch.path())
            .env("GIT_TERMINAL_PROMPT", "0")
            .arg("init")
            .arg("--bare")
            .arg("--object-format=sha256")
            .arg(".")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(parent) = binary.parent() {
            command.env("PATH", parent);
        }
        #[cfg(windows)]
        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        Ok(command.status()?.success())
    }
    pub(crate) fn new(binary: &'a Path, objects: &'a Project, oid_len: usize) -> Result<Self> {
        if !binary.is_absolute() || !matches!(oid_len, 40 | 64) {
            bail!("trusted Git binary must be absolute and object algorithm selected");
        }
        let metadata = std::fs::symlink_metadata(binary)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            bail!("trusted Git binary is not a regular non-link file");
        }
        Ok(Self {
            binary,
            objects,
            oid_len,
        })
    }

    pub(crate) fn command(&self, args: &[&str], initialized: bool) -> Result<Command> {
        let named = Project::open(self.objects.root_path())?;
        if named.root_correlation_token()? != self.objects.root_correlation_token()? {
            bail!("Git object-root pathname changed");
        }
        let mut command = Command::new(self.binary);
        command
            .env_clear()
            .current_dir(self.objects.root_path())
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env(
                "GIT_CONFIG_GLOBAL",
                self.objects.root_path().join(".no-global-config"),
            )
            .env(
                "GIT_TEMPLATE_DIR",
                self.objects.root_path().join("empty-template"),
            )
            .env("HOME", self.objects.root_path())
            .env("USERPROFILE", self.objects.root_path());
        if let Some(parent) = self.binary.parent() {
            command.env("PATH", parent);
        }
        #[cfg(windows)]
        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        if initialized {
            command.arg(format!("--git-dir={}", self.objects.root_path().display()));
        }
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        Ok(command)
    }

    fn finish(&self, mut child: std::process::Child, output_cap: usize) -> Result<Vec<u8>> {
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Git stdout pipe absent");
        };
        let mut output = Vec::new();
        if let Err(error) = stdout
            .take((output_cap as u64).saturating_add(1))
            .read_to_end(&mut output)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
        if output.len() > output_cap {
            let _ = child.kill();
            let _ = child.wait();
            bail!("local Git output exceeded explicit control cap");
        }
        let status = child.wait()?;
        if !status.success() {
            bail!("local Git command failed (exit {status})");
        }
        let named = Project::open(self.objects.root_path())?;
        if named.root_correlation_token()? != self.objects.root_correlation_token()? {
            bail!("Git object-root pathname changed after subprocess");
        }
        Ok(output)
    }

    pub(crate) fn output(&self, args: &[&str], input: &[u8], cap: usize) -> Result<Vec<u8>> {
        let mut child = self.command(args, true)?.spawn()?;
        let Some(mut stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Git stdin pipe absent");
        };
        if let Err(error) = stdin.write_all(input) {
            drop(stdin);
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
        drop(stdin);
        self.finish(child, cap)
    }

    pub(crate) fn oid(&self, output: &[u8]) -> Result<String> {
        let text = std::str::from_utf8(output)?;
        let Some(oid) = text.strip_suffix('\n') else {
            bail!("Git returned no single OID line");
        };
        if oid.len() != self.oid_len
            || !oid
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        {
            bail!("Git returned a malformed or foreign-format OID");
        }
        Ok(oid.into())
    }

    /// Read-only occupancy check. A prior orphaned object cannot be silently
    /// attributed to this staging attempt merely because its bytes match.
    pub(crate) fn object_present(&self, oid: &str) -> Result<bool> {
        self.oid(format!("{oid}\n").as_bytes())?;
        let line = self.output(
            &["cat-file", "--batch-check"],
            format!("{oid}\n").as_bytes(),
            128,
        )?;
        if line == format!("{oid} missing\n").into_bytes() {
            return Ok(false);
        }
        let text = std::str::from_utf8(&line)?;
        if !text.starts_with(&format!("{oid} ")) || !text.ends_with('\n') {
            bail!("Git object occupancy response malformed");
        }
        Ok(true)
    }

    pub(crate) fn write_control_object(&self, kind: &str, bytes: &[u8]) -> Result<String> {
        if !matches!(kind, "commit" | "tag") || bytes.len() > 16 * 1024 {
            bail!("unsupported or oversized Git control object");
        }
        self.oid(&self.output(&["hash-object", "-t", kind, "-w", "--stdin"], bytes, 128)?)
    }

    pub(crate) fn verify_control_object(
        &self,
        kind: &str,
        oid: &str,
        expected: &[u8],
    ) -> Result<String> {
        if !matches!(kind, "commit" | "tag") || expected.len() > 16 * 1024 {
            bail!("unsupported or oversized Git control object");
        }
        self.oid(format!("{oid}\n").as_bytes())?;
        if self.output(&["cat-file", "-t", oid], &[], 32)? != format!("{kind}\n").into_bytes() {
            bail!("retained Git control object has wrong type");
        }
        if self.output(&["cat-file", "-s", oid], &[], 32)?
            != format!("{}\n", expected.len()).into_bytes()
        {
            bail!("retained Git control object has wrong size");
        }
        let actual = self.output(&["cat-file", kind, oid], &[], expected.len())?;
        let hash = format!("sha256:{:x}", Sha256::digest(&actual));
        if actual != expected {
            bail!("retained Git control object bytes differ from e2 plan");
        }
        Ok(hash)
    }

    pub(super) fn init(&self) -> Result<()> {
        let root = self.objects.root_dir()?;
        if !self.objects.child_names_bounded(&root, 1)?.is_empty() {
            bail!("run-scoped Git object root is occupied; never restage");
        }
        self.objects.dir(&["empty-template"], true)?;
        let template = self.objects.root_path().join("empty-template");
        let mut command = self.command(&[], false)?;
        command
            .arg("init")
            .arg("--bare")
            .arg(format!("--template={}", template.display()));
        if self.oid_len == 64 {
            command.arg("--object-format=sha256");
        }
        command.arg(".");
        let mut child = command.spawn()?;
        drop(child.stdin.take());
        self.finish(child, 4096)?;
        Ok(())
    }

    pub(super) fn hash_file(
        &self,
        vault: &Project,
        file: &FileRow,
        max_file_bytes: u64,
    ) -> Result<TreeFile> {
        let hex = file
            .sha256
            .strip_prefix("sha256:")
            .context("vault file digest prefix absent")?;
        let (mut reader, completion) = vault.open_verified_stream_bounded(
            &format!("payload/{}", file.path),
            hex,
            file.size_bytes,
            max_file_bytes,
        )?;
        let mut child = self
            .command(&["hash-object", "--no-filters", "-w", "--stdin"], true)?
            .spawn()?;
        let Some(mut stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Git stdin pipe absent");
        };
        let sent = match std::io::copy(&mut reader, &mut stdin) {
            Ok(value) => value,
            Err(error) => {
                drop(stdin);
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        };
        drop(stdin);
        let object = self.oid(&self.finish(child, 128)?)?;
        if !matches!(completion.outcome(), StreamOutcome::Complete { bytes, sha256 }
            if bytes == file.size_bytes && sha256 == hex && sent == bytes)
        {
            bail!("held payload stream did not complete exact file bytes");
        }
        let mode = if file.unix_mode.is_some_and(|bits| bits & 0o111 != 0) {
            "100755"
        } else {
            "100644"
        };
        Ok(TreeFile {
            path: file.path.clone(),
            blob_oid: object,
            git_mode: mode.into(),
            size_bytes: file.size_bytes,
            sha256: file.sha256.clone(),
        })
    }

    pub(super) fn make_tree(&self, files: &[TreeFile]) -> Result<String> {
        let mut root = Node::default();
        for file in files {
            root.insert(file)?;
        }
        root.write(self)
    }

    pub(super) fn verify(
        &self,
        receipt_files: &[TreeFile],
        root_oid: &str,
        max_file_bytes: u64,
        output_cap: usize,
    ) -> Result<()> {
        if self.oid(&format!("{root_oid}\n").into_bytes())? != root_oid {
            bail!("root OID invalid");
        }
        if self.output(&["cat-file", "-t", root_oid], &[], 16)? != b"tree\n" {
            bail!("retained root Git object is not a tree");
        }
        let listing = self.output(
            &["ls-tree", "-rz", "--full-tree", root_oid],
            &[],
            output_cap,
        )?;
        let mut actual = Vec::new();
        for record in listing.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let tab = record
                .iter()
                .position(|b| *b == b'\t')
                .context("Git tree entry lacks tab")?;
            let head = std::str::from_utf8(&record[..tab])?;
            let path = std::str::from_utf8(&record[tab + 1..])?;
            let mut parts = head.split_whitespace();
            let mode = parts.next().context("Git mode absent")?;
            if parts.next() != Some("blob") {
                bail!("Git tree contains non-blob recursive entry");
            }
            let oid = parts.next().context("Git blob OID absent")?;
            if parts.next().is_some() {
                bail!("Git tree entry has extra fields");
            }
            self.oid(&format!("{oid}\n").into_bytes())?;
            actual.push((path.to_owned(), mode.to_owned(), oid.to_owned()));
        }
        actual.sort();
        let mut expected: Vec<_> = receipt_files
            .iter()
            .map(|f| (f.path.clone(), f.git_mode.clone(), f.blob_oid.clone()))
            .collect();
        expected.sort();
        if actual != expected {
            bail!("retained Git tree paths/modes/OIDs differ from receipt");
        }
        for file in receipt_files {
            self.verify_blob(file, max_file_bytes)?;
        }
        Ok(())
    }

    fn verify_blob(&self, file: &TreeFile, max_file_bytes: u64) -> Result<()> {
        if file.size_bytes > max_file_bytes {
            bail!("Git blob exceeds selected file cap");
        }
        let mut child = self
            .command(&["cat-file", "blob", &file.blob_oid], true)?
            .spawn()?;
        drop(child.stdin.take());
        let Some(mut stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Git blob stdout pipe absent");
        };
        let mut hash = Sha256::new();
        let mut bytes = 0_u64;
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
            bytes = bytes
                .checked_add(read as u64)
                .context("Git blob byte count overflow")?;
            if bytes > file.size_bytes || bytes > max_file_bytes {
                let _ = child.kill();
                let _ = child.wait();
                bail!("Git blob grew beyond exact expected bytes");
            }
            hash.update(&window[..read]);
        }
        if !child.wait()?.success() {
            bail!("Git blob read failed");
        }
        let named = Project::open(self.objects.root_path())?;
        if named.root_correlation_token()? != self.objects.root_correlation_token()? {
            bail!("Git object-root pathname changed after blob subprocess");
        }
        if bytes != file.size_bytes || format!("sha256:{:x}", hash.finalize()) != file.sha256 {
            bail!("retained Git blob bytes differ from vaulted payload");
        }
        Ok(())
    }
}

#[derive(Default)]
struct Node {
    files: BTreeMap<String, (String, String)>,
    directories: BTreeMap<String, Node>,
}
impl Node {
    fn insert(&mut self, file: &TreeFile) -> Result<()> {
        let mut parts = file.path.split('/').peekable();
        let mut node = self;
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                if node
                    .files
                    .insert(part.into(), (file.git_mode.clone(), file.blob_oid.clone()))
                    .is_some()
                {
                    bail!("duplicate Git tree file");
                }
            } else {
                node = node.directories.entry(part.into()).or_default();
            }
        }
        Ok(())
    }
    fn write(&self, git: &LocalGit<'_>) -> Result<String> {
        let mut entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for (name, (mode, oid)) in &self.files {
            entries.push((
                name.as_bytes().to_vec(),
                format!("{mode} blob {oid}\t{name}\0").into_bytes(),
            ));
        }
        for (name, child) in &self.directories {
            let oid = child.write(git)?;
            let mut key = name.as_bytes().to_vec();
            key.push(b'/');
            entries.push((key, format!("040000 tree {oid}\t{name}\0").into_bytes()));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let bytes: Vec<u8> = entries.into_iter().flat_map(|(_, row)| row).collect();
        git.oid(&git.output(&["mktree", "-z"], &bytes, 128)?)
    }
}
