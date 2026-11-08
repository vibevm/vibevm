//! Reading a per-package source as Git (PROP-002 `#registry-mirror`): one
//! scratch object store per run; each version's tag is fetched into it, so
//! the mirror copies the tagged commit's full tree by tree object and
//! records everything a reverse mirror needs to recreate the tag — the tag
//! object, the commit's tree, author, committer and message.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// A Git identity line: name, email and the raw date (`<epoch> <tz>`),
/// exactly as Git stores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Person {
    pub name: String,
    pub email: String,
    pub date: String,
}

/// Everything about one source tag that the bytes do not carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TagFacts {
    /// The tag name exactly as in the source.
    pub tag: String,
    /// `lightweight` or `annotated`.
    pub tag_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_object: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tagger: Option<Person>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_message: Option<String>,
    pub commit: String,
    pub tree: String,
    pub author: Person,
    pub committer: Person,
    pub message: String,
}

/// The run's scratch object store.
pub(crate) struct GitStore {
    dir: tempfile::TempDir,
}

impl GitStore {
    pub(crate) fn new() -> Result<Self> {
        let dir = tempfile::tempdir().context("creating a scratch Git store")?;
        let store = GitStore { dir };
        store.git(&["init", "-q", "--bare"])?;
        Ok(store)
    }

    pub(crate) fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The local ref a fetched tag is kept under.
    pub(crate) fn local_ref(repo_key: &str, tag: &str) -> String {
        format!("refs/mirror/{repo_key}/{tag}")
    }

    /// Fetch `tag` of the repository at `url` (already chosen by the remap
    /// Locator and admitted by source policy) and read its facts.
    pub(crate) fn fetch_tag(&self, url: &str, repo_key: &str, tag: &str) -> Result<TagFacts> {
        let local = Self::local_ref(repo_key, tag);
        self.git(&[
            "fetch",
            "-q",
            "--no-tags",
            "--no-write-fetch-head",
            "--",
            url,
            &format!("+refs/tags/{tag}:{local}"),
        ])
        .with_context(|| format!("fetching tag `{tag}` of `{}`", redact(url)))?;
        self.facts(&local, tag)
    }

    /// Fetch every tag and branch of the repository (for `--keep-history`)
    /// and write a bundle of them to `out`.
    pub(crate) fn bundle(&self, url: &str, repo_key: &str, out: &Path) -> Result<()> {
        self.git(&[
            "fetch",
            "-q",
            "--no-write-fetch-head",
            "--",
            url,
            &format!("+refs/tags/*:refs/history/{repo_key}/tags/*"),
            &format!("+refs/heads/*:refs/history/{repo_key}/heads/*"),
        ])
        .with_context(|| format!("fetching the history of `{}`", redact(url)))?;
        self.git(&[
            "bundle",
            "create",
            &out.to_string_lossy(),
            &format!("--glob=refs/history/{repo_key}/*"),
        ])
        .with_context(|| format!("bundling the history of `{}`", redact(url)))?;
        Ok(())
    }

    /// The refs a bundle carries (`<oid> <ref>` lines), to tell whether a
    /// stored bundle already holds this history.
    pub(crate) fn bundle_heads(&self, bundle: &Path) -> Result<String> {
        self.git(&["bundle", "list-heads", &bundle.to_string_lossy()])
    }

    /// Check out `tree` into the absent directory `dest`.
    pub(crate) fn checkout(&self, tree: &str, dest: &Path) -> Result<()> {
        std::fs::create_dir_all(dest).with_context(|| format!("creating `{}`", dest.display()))?;
        let index = self.path().join(format!("checkout-{tree}.index"));
        let work = dest.to_string_lossy();
        self.git_with_index(&index, &["read-tree", tree])?;
        self.git_with_index(
            &index,
            &["--work-tree", &work, "checkout-index", "-a", "-f"],
        )?;
        Ok(())
    }

    fn facts(&self, local: &str, tag: &str) -> Result<TagFacts> {
        let kind = self.git(&["cat-file", "-t", local])?;
        let (tag_kind, tag_object, tagger, tag_message) = if kind == "tag" {
            let object = self.git(&["rev-parse", local])?;
            let body = self.git_raw(&["cat-file", "tag", &object])?;
            let (header, message) = body.split_once("\n\n").unwrap_or((body.as_str(), ""));
            let tagger = header
                .lines()
                .find_map(|line| line.strip_prefix("tagger "))
                .map(parse_person);
            (
                "annotated".to_string(),
                Some(object),
                tagger,
                Some(message.to_string()),
            )
        } else {
            ("lightweight".to_string(), None, None, None)
        };
        let commit = self.git(&["rev-parse", &format!("{local}^{{commit}}")])?;
        let tree = self.git(&["rev-parse", &format!("{local}^{{tree}}")])?;
        let body = self.git_raw(&["cat-file", "commit", &commit])?;
        let (header, message) = body.split_once("\n\n").unwrap_or((body.as_str(), ""));
        let person = |field: &str| {
            header
                .lines()
                .find_map(|line| line.strip_prefix(field))
                .map(parse_person)
        };
        let (Some(author), Some(committer)) = (person("author "), person("committer ")) else {
            bail!("commit {commit} of tag `{tag}` has no author or committer line");
        };
        Ok(TagFacts {
            tag: tag.to_string(),
            tag_kind,
            tag_object,
            tagger,
            tag_message,
            commit,
            tree,
            author,
            committer,
            message: message.to_string(),
        })
    }

    fn git(&self, args: &[&str]) -> Result<String> {
        Ok(self.git_raw(args)?.trim().to_string())
    }

    fn git_raw(&self, args: &[&str]) -> Result<String> {
        run(Command::new("git")
            .arg("--git-dir")
            .arg(self.path())
            .args(args))
    }

    fn git_with_index(&self, index: &PathBuf, args: &[&str]) -> Result<String> {
        run(Command::new("git")
            .arg("--git-dir")
            .arg(self.path())
            .args(args)
            .env("GIT_INDEX_FILE", index))
    }
}

fn run(command: &mut Command) -> Result<String> {
    let output = command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .output()
        .context("spawning git")?;
    if !output.status.success() {
        bail!(
            "git failed: {}",
            redact(String::from_utf8_lossy(&output.stderr).trim())
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn redact(text: &str) -> String {
    vibe_publish::git_publish::redact_credentials(text)
}

/// `Name <email> 1699999999 +0300` → [`Person`].
fn parse_person(line: &str) -> Person {
    let (name, rest) = line.split_once(" <").unwrap_or((line, ""));
    let (email, date) = rest.split_once("> ").unwrap_or((rest, ""));
    Person {
        name: name.to_string(),
        email: email.trim_end_matches('>').to_string(),
        date: date.to_string(),
    }
}

/// The host of a Git url (`scheme://[user@]host[:port]/…` or scp
/// `[user@]host:path`), lower-cased; `None` for a local path.
pub(crate) fn url_host(url: &str) -> Option<String> {
    let rest = match url.split_once("://") {
        Some((scheme, rest)) if scheme != "file" => rest,
        Some(_) => return None,
        None => {
            let (host, path) = url.split_once(':')?;
            if host.contains('/') || path.is_empty() || host.len() < 2 {
                return None;
            }
            host
        }
    };
    let authority = rest.split('/').next().unwrap_or(rest);
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = host.split(':').next().unwrap_or(host);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_read_from_every_git_url_form() {
        assert_eq!(
            url_host("https://github.com/vibespecs/x.git").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            url_host("ssh://git@corp.example:2222/vibe.git").as_deref(),
            Some("corp.example")
        );
        assert_eq!(
            url_host("git@GitVerse.ru:vibespecs/x.git").as_deref(),
            Some("gitverse.ru")
        );
        assert_eq!(url_host("file:///srv/x.git"), None);
        assert_eq!(url_host("/srv/x.git"), None);
    }

    #[test]
    fn identity_lines_keep_the_raw_date() {
        let person = parse_person("Oleg Chirukhin <o@example.invalid> 1762600000 +0300");
        assert_eq!(person.name, "Oleg Chirukhin");
        assert_eq!(person.email, "o@example.invalid");
        assert_eq!(person.date, "1762600000 +0300");
    }
}
