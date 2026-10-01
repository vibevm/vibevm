//! Epoch-two canonical Git commit/tag bytes, independent of ambient Git config.
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};

use super::{PackageGitPlan, PlanError, StagedCommit, invalid};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GitPerson {
    pub name: String,
    pub email: String,
    pub timestamp_seconds: i64,
    pub timezone_offset_minutes: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommitMetadata {
    pub author: GitPerson,
    pub committer: GitPerson,
    /// Exact UTF-8 message bytes, including its final LF.
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TagMetadata {
    pub tagger: GitPerson,
    /// Exact UTF-8 message bytes, including its final LF.
    pub message: String,
}

impl GitPerson {
    fn validate(&self) -> Result<(), PlanError> {
        if self.name.is_empty()
            || self.name.len() > 200
            || self.name.trim() != self.name
            || self.name.contains(['<', '>'])
            || self.name.chars().any(char::is_control)
            || self.email.is_empty()
            || self.email.len() > 254
            || !self.email.is_ascii()
            || self.email.contains(['<', '>', ' ', '\t', '\r', '\n'])
            || self.email.chars().any(char::is_control)
            || self.email.matches('@').count() != 1
            || self.email.starts_with('@')
            || self.email.ends_with('@')
            || self.timestamp_seconds < 0
            || self.timestamp_seconds > 253_402_300_799
            || !(-14 * 60..=14 * 60).contains(&i32::from(self.timezone_offset_minutes))
        {
            return Err(invalid("Git object attribution, timestamp, or timezone"));
        }
        Ok(())
    }

    fn line(&self) -> String {
        let offset = i32::from(self.timezone_offset_minutes);
        let sign = if offset < 0 { '-' } else { '+' };
        let minutes = offset.abs();
        format!(
            "{} <{}> {} {sign}{:02}{:02}",
            self.name,
            self.email,
            self.timestamp_seconds,
            minutes / 60,
            minutes % 60
        )
    }
}

fn valid_message(message: &str) -> bool {
    !message.is_empty()
        && message.len() <= 4096
        && message.ends_with('\n')
        && !message
            .chars()
            .any(|ch| ch == '\0' || ch == '\r' || (ch.is_control() && ch != '\n' && ch != '\t'))
}

pub(crate) enum GitObjectHasher {
    Sha1(Sha1),
    Sha256(Sha256),
}

impl GitObjectHasher {
    pub(crate) fn new(kind: &str, size: u64, len: usize) -> Result<Self, PlanError> {
        if !matches!(kind, "blob" | "tree" | "commit" | "tag") {
            return Err(invalid("Git object type"));
        }
        let header = format!("{kind} {size}\0");
        match len {
            40 => {
                let mut hash = Sha1::new();
                hash.update(header.as_bytes());
                Ok(Self::Sha1(hash))
            }
            64 => {
                let mut hash = Sha256::new();
                hash.update(header.as_bytes());
                Ok(Self::Sha256(hash))
            }
            _ => Err(invalid("Git object algorithm")),
        }
    }
    pub(crate) fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha1(hash) => hash.update(bytes),
            Self::Sha256(hash) => hash.update(bytes),
        }
    }
    pub(crate) fn finish(self) -> String {
        match self {
            Self::Sha1(hash) => format!("{:x}", hash.finalize()),
            Self::Sha256(hash) => format!("{:x}", hash.finalize()),
        }
    }
}

fn object_oid(kind: &str, bytes: &[u8], len: usize) -> Result<String, PlanError> {
    let mut hash = GitObjectHasher::new(kind, bytes.len() as u64, len)?;
    hash.update(bytes);
    Ok(hash.finish())
}

impl PackageGitPlan {
    fn commit_bytes_unchecked(&self) -> Result<Option<Vec<u8>>, PlanError> {
        let StagedCommit::New { parent_oid, .. } = &self.staged.commit else {
            if self.staged.commit_metadata.is_some() {
                return Err(invalid("reused main cannot carry a new commit metadata"));
            }
            return Ok(None);
        };
        let metadata = self
            .staged
            .commit_metadata
            .as_ref()
            .ok_or_else(|| invalid("new commit metadata absent"))?;
        metadata.author.validate()?;
        metadata.committer.validate()?;
        if !valid_message(&metadata.message) {
            return Err(invalid("commit message bytes"));
        }
        let mut body = format!("tree {}\n", self.staged.git_tree_oid);
        if let Some(parent) = parent_oid {
            body.push_str(&format!("parent {parent}\n"));
        }
        body.push_str(&format!(
            "author {}\ncommitter {}\n\n",
            metadata.author.line(),
            metadata.committer.line()
        ));
        body.push_str(&metadata.message);
        Ok(Some(body.into_bytes()))
    }

    fn tag_bytes_unchecked(&self) -> Result<Option<Vec<u8>>, PlanError> {
        if self.staged.annotated_tag_oid.is_none() {
            if self.staged.tag_metadata.is_some() {
                return Err(invalid("absent tag object carries metadata"));
            }
            return Ok(None);
        }
        let metadata = self
            .staged
            .tag_metadata
            .as_ref()
            .ok_or_else(|| invalid("annotated tag metadata absent"))?;
        metadata.tagger.validate()?;
        if !valid_message(&metadata.message) {
            return Err(invalid("tag message bytes"));
        }
        let name = self
            .version_tag_ref
            .strip_prefix("refs/tags/")
            .ok_or_else(|| invalid("selected tag ref"))?;
        let body = format!(
            "object {}\ntype commit\ntag {name}\ntagger {}\n\n{}",
            self.staged.tag_target_commit_oid,
            metadata.tagger.line(),
            metadata.message
        );
        Ok(Some(body.into_bytes()))
    }

    pub(super) fn validate_object_bytes(&self) -> Result<(), PlanError> {
        if matches!(&self.staged.commit, StagedCommit::ReusedMain { oid }
            if matches!(&self.before_refs.version_tag, super::TagRef::Present { peeled_commit_oid, .. } if peeled_commit_oid == oid)
                && self.staged.annotated_tag_oid.is_some())
        {
            return Err(invalid(
                "exact peeled no-op cannot manufacture a tag object",
            ));
        }
        if let Some(bytes) = self.commit_bytes_unchecked()?
            && object_oid("commit", &bytes, self.oid_hex_len)? != self.staged.commit.oid()
        {
            return Err(invalid("staged commit OID differs from canonical bytes"));
        }
        if let Some(bytes) = self.tag_bytes_unchecked()?
            && self.staged.annotated_tag_oid.as_deref()
                != Some(object_oid("tag", &bytes, self.oid_hex_len)?.as_str())
        {
            return Err(invalid(
                "staged annotated tag OID differs from canonical bytes",
            ));
        }
        Ok(())
    }

    /// Exact bounded raw object bytes for later local staging, never a Git effect.
    pub(crate) fn canonical_commit_bytes(&self) -> Result<Option<Vec<u8>>, PlanError> {
        self.validate()?;
        self.commit_bytes_unchecked()
    }

    pub(crate) fn canonical_tag_bytes(&self) -> Result<Option<Vec<u8>>, PlanError> {
        self.validate()?;
        self.tag_bytes_unchecked()
    }

    /// Pure authoring helper; the final plan still validates all identities.
    pub(crate) fn derive_staged_object_ids(&mut self) -> Result<(), PlanError> {
        if let Some(bytes) = self.commit_bytes_unchecked()? {
            let oid = object_oid("commit", &bytes, self.oid_hex_len)?;
            if let StagedCommit::New { oid: selected, .. } = &mut self.staged.commit {
                *selected = oid;
            }
        }
        self.staged.tag_target_commit_oid = self.staged.commit.oid().into();
        if let Some(bytes) = self.tag_bytes_unchecked()? {
            self.staged.annotated_tag_oid = Some(object_oid("tag", &bytes, self.oid_hex_len)?);
        }
        self.validate()
    }
}
