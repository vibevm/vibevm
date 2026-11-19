//! Stream the same authenticated npm bytes through gzip and tar without extraction.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#npm-runtime");
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::io::Read;
use std::path::Path;
use vibe_core::manifest::{InstallerOptionDecl, NpmRuntimeDecl};

/// No package-size ceiling: only the selected metadata member is retained.
/// Parsed metadata confers no authority until the entire stream is authenticated.
pub fn verify_npm_runtime_archive(
    option: &InstallerOptionDecl,
    binding: &NpmRuntimeDecl,
    archive: &Path,
) -> Result<(), String> {
    verify_npm_runtime_archive_interruptible(option, binding, archive, None)
}

/// Same streaming verifier with the operation's existing interrupt owner.
pub fn verify_npm_runtime_archive_interruptible(
    option: &InstallerOptionDecl,
    binding: &NpmRuntimeDecl,
    archive: &Path,
    interrupt: Option<&dyn crate::Interrupt>,
) -> Result<(), String> {
    verify_scan(option, binding, archive, u64::MAX, u64::MAX, interrupt)
}

struct State {
    hash: Sha256,
    compressed: u64,
    decoded: u64,
}
struct Observed<'a, R> {
    inner: R,
    state: &'a RefCell<State>,
    compressed: bool,
    limit: u64,
    interrupt: Option<&'a dyn crate::Interrupt>,
}
impl<R: Read> Read for Observed<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.interrupt.is_some_and(|i| i.poll(None)) {
            return Err(std::io::Error::other("npm scan interrupted by operator"));
        }
        let n = self.inner.read(buffer)?;
        let mut s = self.state.borrow_mut();
        let total = if self.compressed {
            s.compressed
        } else {
            s.decoded
        };
        let next = total
            .checked_add(n as u64)
            .ok_or_else(|| std::io::Error::other("scan counter overflow"))?;
        if next > self.limit {
            return Err(std::io::Error::other("npm scan policy budget exceeded"));
        }
        if self.compressed {
            s.hash.update(&buffer[..n]);
            s.compressed = next;
        } else {
            s.decoded = next;
        }
        Ok(n)
    }
}
#[cfg(test)]
fn verify_with_limits(
    option: &InstallerOptionDecl,
    binding: &NpmRuntimeDecl,
    archive: &Path,
    compressed_limit: u64,
    decoded_limit: u64,
) -> Result<(), String> {
    verify_scan(
        option,
        binding,
        archive,
        compressed_limit,
        decoded_limit,
        None,
    )
}

fn verify_scan(
    option: &InstallerOptionDecl,
    binding: &NpmRuntimeDecl,
    archive: &Path,
    compressed_limit: u64,
    decoded_limit: u64,
    interrupt: Option<&dyn crate::Interrupt>,
) -> Result<(), String> {
    let expected = binding.input_digest(option)?;
    let file = std::fs::File::open(archive).map_err(|_| "cannot open npm runtime input")?;
    let state = RefCell::new(State {
        hash: Sha256::new(),
        compressed: 0,
        decoded: 0,
    });
    let input = Observed {
        inner: file,
        state: &state,
        compressed: true,
        limit: compressed_limit,
        interrupt,
    };
    let decoded = Observed {
        inner: flate2::read::MultiGzDecoder::new(input),
        state: &state,
        compressed: false,
        limit: decoded_limit,
        interrupt,
    };
    let mut tar = tar::Archive::new(decoded);
    let mut metadata = None;
    for entry in tar.entries().map_err(|e| format!("npm scan failed: {e}"))? {
        let mut entry = entry.map_err(|e| format!("npm scan failed: {e}"))?;
        if entry.path_bytes().as_ref() != b"package/package.json" {
            continue;
        }
        if metadata.is_some()
            || !entry.header().entry_type().is_file()
            || entry.size() > 1024 * 1024
        {
            return Err("npm package metadata must be one bounded regular file".into());
        }
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("npm scan failed: {e}"))?;
        if bytes.len() > 1024 * 1024 {
            return Err("npm metadata member bound exceeded".into());
        }
        metadata = Some(bytes);
    }
    // Tar stops at end markers. Drain gzip fully for CRC, truncation and all
    // compressed bytes, including concatenated members; never reopen the file.
    let mut decoded = tar.into_inner();
    let mut tail = [0u8; 65536];
    loop {
        let read = decoded
            .read(&mut tail)
            .map_err(|e| format!("npm scan failed: {e}"))?;
        if read == 0 {
            break;
        }
        if tail[..read].iter().any(|byte| *byte != 0) {
            return Err("npm tarball has non-padding data after its end markers".into());
        }
    }
    drop(decoded);
    let observed = state.into_inner();
    if format!("{:x}", observed.hash.finalize()) != expected.to_ascii_lowercase() {
        return Err("npm runtime input digest mismatch".into());
    }
    binding.verify_package_json(&metadata.ok_or("npm tarball lacks package/package.json")?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive(path: &Path, name: &str, json: &[u8], duplicate: bool) -> String {
        let encoder = flate2::write::GzEncoder::new(
            std::fs::File::create(path).unwrap(),
            flate2::Compression::fast(),
        );
        let mut tar = tar::Builder::new(encoder);
        for _ in 0..if duplicate { 2 } else { 1 } {
            let mut header = tar::Header::new_gnu();
            header.set_size(json.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, name, json).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap();
        format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
    }
    fn option(digest: &str) -> InstallerOptionDecl {
        toml::from_str(&format!("id='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\n[npm_runtime]\nstep='npm'\npackage='test-package'\nversion='1.0.0'\nnode_range='>=22.0.0'\n[[steps]]\nid='npm'\ntitle='Npm input'\nkind='archive'\nformat='tar_gz'\nsource='package.tgz'\nsha256='{digest}'\n")).unwrap()
    }
    #[test]
    fn concatenated_tar_cannot_hide_duplicate_metadata_after_end_markers() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("package.tgz");
        let metadata =
            br#"{"name":"test-package","version":"1.0.0","engines":{"node":">=22.0.0"}}"#;
        archive(&path, "package/package.json", metadata, false);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.extend_from_within(..);
        std::fs::write(&path, &bytes).unwrap();
        let selected = option(&format!("{:x}", Sha256::digest(&bytes)));
        assert!(
            verify_npm_runtime_archive(&selected, selected.npm_runtime.as_ref().unwrap(), &path)
                .is_err()
        );
    }

    #[test]
    fn streaming_interrupt_refuses_before_metadata_authority() {
        struct Stop;
        impl crate::Interrupt for Stop {
            fn poll(&self, _: Option<u32>) -> bool {
                true
            }
        }
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("package.tgz");
        let metadata =
            br#"{"name":"test-package","version":"1.0.0","engines":{"node":">=22.0.0"}}"#;
        let digest = archive(&path, "package/package.json", metadata, false);
        let selected = option(&digest);
        let fault = verify_npm_runtime_archive_interruptible(
            &selected,
            selected.npm_runtime.as_ref().unwrap(),
            &path,
            Some(&Stop),
        )
        .unwrap_err();
        assert!(fault.contains("interrupted"), "{fault}");
    }

    #[test]
    fn stream_counters_cross_four_gib_and_refuse_overflow() {
        for compressed in [true, false] {
            let state = RefCell::new(State {
                hash: Sha256::new(),
                compressed: u32::MAX as u64 - 2,
                decoded: u32::MAX as u64 - 2,
            });
            let mut reader = Observed {
                inner: std::io::repeat(0).take(8),
                state: &state,
                compressed,
                limit: u64::MAX,
                interrupt: None,
            };
            let mut buffer = [0; 8];
            assert_eq!(reader.read(&mut buffer).unwrap(), 8);
            assert_eq!(
                if compressed {
                    state.borrow().compressed
                } else {
                    state.borrow().decoded
                },
                u32::MAX as u64 + 6
            );
            if compressed {
                state.borrow_mut().compressed = u64::MAX;
            } else {
                state.borrow_mut().decoded = u64::MAX;
            }
            let mut reader = Observed {
                inner: std::io::repeat(0),
                state: &state,
                compressed,
                limit: u64::MAX,
                interrupt: None,
            };
            assert!(reader.read(&mut buffer).is_err());
        }
    }

    #[test]
    #[ignore = "explicit multi-GiB streaming proof; no giant committed fixture"]
    fn generated_multi_gib_npm_stream() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("large.tgz");
        let encoder = flate2::write::GzEncoder::new(
            std::fs::File::create(&path).unwrap(),
            flate2::Compression::fast(),
        );
        let mut tar = tar::Builder::new(encoder);
        let size = (1u64 << 32) + 1024;
        let mut header = tar::Header::new_gnu();
        header.set_size(size);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(
            &mut header,
            "package/large-generated-data",
            std::io::repeat(0).take(size),
        )
        .unwrap();
        let json = br#"{"name":"test-package","version":"1.0.0","engines":{"node":">=22.0.0"}}"#;
        let mut header = tar::Header::new_gnu();
        header.set_size(json.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "package/package.json", &json[..])
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let mut input = std::fs::File::open(&path).unwrap();
        let mut hash = Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = input.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
        }
        let selected = option(&format!("{:x}", hash.finalize()));
        verify_npm_runtime_archive(&selected, selected.npm_runtime.as_ref().unwrap(), &path)
            .unwrap();
    }

    #[test]
    #[ignore = "explicit exact upstream input supplied by supervisor"]
    fn pinned_qwen_npm_archive() {
        let path = std::path::PathBuf::from(
            std::env::var_os("VIBE_TEST_NPM_ARCHIVE").expect("explicit private archive path"),
        );
        let mut selected =
            option("32ca2695069210bae82410a8712d8658a7da5779ecbfcd67160adce415a4e1d9");
        let binding = selected.npm_runtime.as_mut().unwrap();
        binding.package = "@qwen-code/qwen-code".into();
        binding.version = "0.28.0".into();
        verify_npm_runtime_archive(&selected, selected.npm_runtime.as_ref().unwrap(), &path)
            .unwrap();
    }

    #[test]
    fn compressed_decoded_limits_and_corrupt_gzip_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("package.tgz");
        let metadata =
            br#"{"name":"test-package","version":"1.0.0","engines":{"node":">=22.0.0"}}"#;
        let digest = archive(&path, "package/package.json", metadata, false);
        let selected = option(&digest);
        let binding = selected.npm_runtime.as_ref().unwrap();
        let raw = std::fs::read(&path).unwrap();
        assert!(
            verify_with_limits(&selected, binding, &path, raw.len() as u64 - 1, 65536)
                .unwrap_err()
                .contains("policy budget")
        );
        assert!(
            verify_with_limits(&selected, binding, &path, 65536, 512)
                .unwrap_err()
                .contains("policy budget")
        );
        verify_with_limits(&selected, binding, &path, raw.len() as u64, 65536).unwrap();
        for cut in [raw.len() - 1, raw.len() / 2] {
            std::fs::write(&path, &raw[..cut]).unwrap();
            let bad = option(&format!("{:x}", Sha256::digest(&raw[..cut])));
            assert!(
                verify_with_limits(&bad, bad.npm_runtime.as_ref().unwrap(), &path, 65536, 65536)
                    .is_err()
            );
        }
        let mut corrupt = raw;
        let n = corrupt.len();
        corrupt[n - 8] ^= 1;
        std::fs::write(&path, &corrupt).unwrap();
        let bad = option(&format!("{:x}", Sha256::digest(&corrupt)));
        assert!(
            verify_with_limits(&bad, bad.npm_runtime.as_ref().unwrap(), &path, 65536, 65536)
                .is_err()
        );
    }

    #[test]
    fn verified_npm_metadata_requires_digest_identity_and_unique_regular_member() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("package.tgz");
        let metadata =
            br#"{"name":"test-package","version":"1.0.0","engines":{"node":">=22.0.0"}}"#;
        let digest = archive(&path, "package/package.json", metadata, false);
        let selected = option(&digest);
        let binding = selected.npm_runtime.as_ref().unwrap();
        verify_npm_runtime_archive(&selected, binding, &path).unwrap();
        let mut wrong = binding.clone();
        wrong.package = "different".into();
        assert!(verify_npm_runtime_archive(&selected, &wrong, &path).is_err());
        let selected_bad = option(&"0".repeat(64));
        assert!(
            verify_npm_runtime_archive(
                &selected_bad,
                selected_bad.npm_runtime.as_ref().unwrap(),
                &path
            )
            .is_err()
        );
        for (name, duplicate) in [
            ("package/package.json", true),
            ("elsewhere/package.json", false),
        ] {
            let digest = archive(&path, name, metadata, duplicate);
            let selected = option(&digest);
            assert!(
                verify_npm_runtime_archive(
                    &selected,
                    selected.npm_runtime.as_ref().unwrap(),
                    &path
                )
                .is_err()
            );
        }
    }
}
