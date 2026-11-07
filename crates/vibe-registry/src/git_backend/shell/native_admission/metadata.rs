//! Bounded metadata capture; authored configuration is data, never execution authority.
use super::*;
impl NativeState {
    fn owned_origin(&self, repo: &Path) -> Result<Option<String>, GitError> {
        if self
            .owned
            .lock()
            .map_err(|_| unsupported("owned source lock poisoned"))?
            .contains_key(repo)
        {
            self.with_owned(repo, |owned| Ok(Some(owned.origin.clone())))
        } else {
            Ok(None)
        }
    }
    fn local_config(&self, repo: &Path) -> Result<Vec<(String, String)>, GitError> {
        // Linked worktrees, bare external stores and alternate object stores need
        // their own proof. Do not follow a .git file into another authority.
        let git = repo.join(".git");
        if !git.is_dir()
            || std::fs::symlink_metadata(&git)
                .map_err(io)?
                .file_type()
                .is_symlink()
            || git.join("commondir").exists()
            || git.join("objects/info/alternates").exists()
            || git.join("info/attributes").exists()
        {
            return Err(unsupported(
                "warm repository layout or attributes require a separate capture profile",
            ));
        }
        let config = Context::new(&self.profile)?.config(&git.join("config"))?;
        for (key, _) in &config {
            let allowed = matches!(
                key.as_str(),
                "core.repositoryformatversion"
                    | "core.filemode"
                    | "core.bare"
                    | "core.logallrefupdates"
                    | "core.ignorecase"
                    | "core.precomposeunicode"
                    | "extensions.objectformat"
                    | "remote.origin.url"
                    | "remote.origin.fetch"
                    | "remote.origin.tagopt"
                    | "user.name"
                    | "user.email"
            ) || (key.starts_with("branch.")
                && (key.ends_with(".remote") || key.ends_with(".merge")))
                || (key.starts_with("url.") && key.ends_with(".insteadof"));
            if !allowed {
                return Err(unsupported(
                    "warm includes, transforms, helpers or other uncaptured config",
                ));
            }
        }
        Ok(config)
    }
    pub(in crate::git_backend::shell) fn origin(
        &self,
        repo: &Path,
        _effective: bool,
    ) -> Result<String, GitError> {
        if let Some(origin) = self.owned_origin(repo)? {
            return Ok(origin);
        }
        // The selected installation may have a different compiled system config
        // path. Guessing /etc or ignoring includes would change warm meaning.
        // N2 must supply complete captured ambient authority before enabling it.
        Err(unsupported(
            "warm transport context requires complete config capture; raw origin is not an effective endpoint",
        ))
    }
    pub(in crate::git_backend::shell) fn head(
        &self,
        repo: &Path,
    ) -> Result<Option<String>, GitError> {
        if self.owned_origin(repo)?.is_some() {
            return self.with_owned(repo, |_| self.read_head(repo));
        }
        self.local_config(repo)?;
        self.read_head(&repo.join(".git"))
    }
    fn read_head(&self, git_dir: &Path) -> Result<Option<String>, GitError> {
        let value = text(
            Context::new(&self.profile)?
                .run(&["rev-parse", "--verify", "HEAD"], Some(git_dir), None)?
                .stdout,
        )?;
        if !full_oid(&value) {
            return Err(unsupported("HEAD is not a full object identity"));
        }
        Ok(Some(value))
    }
}
