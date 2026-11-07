//! Rebase managed child locators when a composed parent moves their metadata.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;

fn relative(from: &Path, to: &Path) -> Result<PathBuf, GitError> {
    let ancestor = from
        .ancestors()
        .find(|ancestor| to.starts_with(ancestor))
        .ok_or_else(|| unsupported("child layout has no common owned ancestor"))?;
    let mut result = PathBuf::new();
    for _ in from
        .strip_prefix(ancestor)
        .map_err(|_| unsupported("invalid child layout root"))?
        .components()
    {
        result.push("..");
    }
    result.push(
        to.strip_prefix(ancestor)
            .map_err(|_| unsupported("invalid child layout target"))?,
    );
    Ok(result)
}
impl NativeState {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW"
    )]
    pub(super) fn relocate_child_layout(
        &self,
        worktree: &Path,
        git_dir: &Path,
        child: &gitlinks::Child,
    ) -> Result<(), GitError> {
        let locator = relative(worktree, git_dir)?;
        std::fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", locator.display()),
        )
        .map_err(io)?;
        let worktree_relative = relative(git_dir, worktree)?;
        let config = git_dir.join("config");
        Context::new(&self.profile)?.run(
            &[
                "config",
                "--no-includes",
                "--file",
                config
                    .to_str()
                    .ok_or_else(|| unsupported("non-UTF-8 child config path"))?,
                "core.worktree",
                worktree_relative
                    .to_str()
                    .ok_or_else(|| unsupported("non-UTF-8 child worktree path"))?,
            ],
            None,
            None,
        )?;
        let descendants = self.with_owned(&child.objects, |owned| Ok(owned.children.clone()))?;
        for descendant in descendants {
            if descendant.absorbed {
                self.relocate_child_layout(
                    &worktree.join(&descendant.path),
                    &git_dir.join("modules").join(&descendant.name),
                    &descendant,
                )?;
            }
        }
        Ok(())
    }
}
