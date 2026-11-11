//! A selected Git older than the options this capture prefers.
//!
//! The controlled transport asks the selected Git three questions whose
//! spellings are not as old as Git: `git var GIT_ATTR_SYSTEM` /
//! `GIT_ATTR_GLOBAL` (Git 2.46) name the attribute files it would read, and
//! `--attr-source` / `check-attr --source` (Git 2.40) read the payload's own
//! `.gitattributes` out of the commit rather than out of a working tree that
//! does not hold it yet.
//!
//! **Apple Git 2.39.5 answers none of them.** That is the only `git` a stock
//! macOS carries — `/usr/bin/git`, the Command Line Tools shim — so on a Mac
//! without Homebrew the first of those questions refused every controlled
//! operation outright (`selected Git metadata reader failed; context was not
//! captured`), and a monorepo registry on disk could not be read at all.
//!
//! The fixture here is that Git: a selected installation that answers the
//! three spellings exactly as 2.39.5 does — its usage message and exit 129 —
//! and real Git for everything else. What must still hold is everything the
//! capture is *for*: the global attributes file still reaches the checkout,
//! and a payload that selects `ident`, a filter or a working-tree encoding is
//! still refused before anything is published.
#![cfg(all(test, unix))]
use super::*;
use crate::ShellGit;
use crate::git_backend::GitBackend;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

/// A `git` that is 2.39.5 in exactly the three ways that matter.
fn old_git(directory: &Path) -> PathBuf {
    let shim = directory.join("git");
    std::fs::write(
        &shim,
        "#!/bin/sh\n\
         for a in \"$@\"; do\n\
         \x20 case \"$a\" in\n\
         \x20   --attr-source=*|--source=*)\n\
         \x20     echo 'unknown option:' \"$a\" >&2; exit 129 ;;\n\
         \x20   GIT_ATTR_SYSTEM|GIT_ATTR_GLOBAL)\n\
         \x20     echo 'usage: git var (-l | <variable>)' >&2; exit 129 ;;\n\
         \x20 esac\n\
         done\n\
         exec /usr/bin/git \"$@\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    shim
}

/// Plain Git, for building the fixture repository.
fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=f@example.invalid",
            "-c",
            "core.autocrlf=false",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

struct Old {
    root: tempfile::TempDir,
}

impl Old {
    /// A repository on disk carrying `payload` and `attributes`, plus a home
    /// whose global attributes file says what `global` says.
    fn new(attributes: &str, global: &str) -> Self {
        let old = Old {
            root: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(old.path("home/.config/git")).unwrap();
        std::fs::write(old.path("home/.config/git/attributes"), global).unwrap();
        std::fs::create_dir_all(old.path("bin")).unwrap();
        let work = old.path("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("payload.txt"), "retained fixture\n").unwrap();
        git(&work, &["init", "-q", "-b", "main", "."]);
        git(&work, &["add", "payload.txt"]);
        git(&work, &["commit", "-qm", "the payload"]);
        // The attributes land in a second commit: recording them first would
        // make the fixture's own `git add` apply them to the payload, and the
        // blob has to stay the bytes the test compares against.
        std::fs::write(work.join(".gitattributes"), attributes).unwrap();
        git(&work, &["add", ".gitattributes"]);
        git(&work, &["commit", "-qm", "the attributes"]);
        old
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    /// A controlled backend whose selected Git is the old one.
    fn backend(&self) -> ShellGit {
        let home = self.path("home");
        let selection = NativeConfigSelection::new(home.clone(), home.clone())
            .unwrap()
            .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
            .unwrap();
        let profile = NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new([] as [&str; 0]).unwrap()),
            GitInstallation::trusted(old_git(&self.path("bin")), "/usr/lib/git-core".into())
                .unwrap(),
            TlsTrust::System,
        )
        .with_config_selection(selection);
        ShellGit::new().with_native_admission(Arc::new(profile))
    }

    /// Acquire the repository's `main` and stage it.
    fn stage(
        &self,
        bytes: crate::git_backend::PayloadBytes,
    ) -> Result<NativeMaterialisation, GitError> {
        let git = self.backend();
        let url = format!("file://{}", self.path("work").display());
        let objects = self.path("objects.git");
        let slot = self.path("slot");
        let revision = git.prepare_ref(&url, "main", None, &objects, &slot)?;
        Ok(git
            .stage_native_materialisation(&objects, &revision, bytes)?
            .expect("the controlled backend stages"))
    }
}

/// The reported failure: a Git that cannot be asked where its attribute files
/// are refused the whole operation. It must not — and the global file it could
/// not name must still reach the checkout, because Git's default for it is
/// derivable from inputs this capture already holds.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION")]
fn a_git_that_cannot_name_its_attribute_files_still_captures_a_context() {
    let old = Old::new("# nothing in the tree\n", "payload.txt text eol=crlf\n");
    let staged = old
        .stage(crate::git_backend::PayloadBytes::AsConfigured)
        .expect("an older selected Git is not a failed capture");
    assert_eq!(
        std::fs::read(staged.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\r\n",
        "the global attributes file still decides what the user's slot holds"
    );
}

/// The same Git, staging a tree vibe reads as data: the recorded blob bytes,
/// although neither `--attr-source` nor the payload's own `eol=crlf` could be
/// read the way the newer spelling reads them.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION")]
fn an_older_git_still_stages_a_data_tree_byte_for_byte() {
    let old = Old::new("* text=auto eol=crlf\n", "* text=auto eol=crlf\n");
    let staged = old
        .stage(crate::git_backend::PayloadBytes::Verbatim)
        .expect("an older selected Git stages a data tree");
    assert_eq!(
        std::fs::read(staged.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
}

/// And the refusal the probes exist for still lands, although they can no
/// longer be asked *before* the checkout: an `ident` payload is refused, and
/// the staging that would have carried it never returns one.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION")]
fn an_older_git_still_refuses_an_unsupported_transformation() {
    for attributes in [
        "payload.txt ident\n",
        "payload.txt working-tree-encoding=UTF-16\n",
        "payload.txt filter=canary\n",
    ] {
        let old = Old::new(attributes, "");
        for bytes in [
            crate::git_backend::PayloadBytes::Verbatim,
            crate::git_backend::PayloadBytes::AsConfigured,
        ] {
            let answer = old.stage(bytes);
            assert!(
                matches!(&answer, Err(GitError::NativeContext { .. })),
                "{attributes} under {bytes:?}: {}",
                match &answer {
                    Ok(_) => "admitted".to_owned(),
                    Err(error) => error.to_string(),
                }
            );
        }
    }
}
