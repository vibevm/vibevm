//! Bringing the per-package clone to a ref — the refresh / source
//! switch machinery (PROP-010 §2.6). Split from `fetch.rs` along that
//! seam when the intent split outgrew the combined file: this file
//! owns HOW a copy on disk is brought to a ref; `fetch.rs` owns WHEN
//! each intent applies (the position in the primary-then-mirrors
//! chain) and everything built on top of the clone.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use crate::git_failure::GitOperation;

/// How one bring-the-clone-to-ref attempt relates to the copy already
/// on disk — PROP-010 §2.6: **a refresh and a source switch are
/// different operations**, and the code must tell them apart before it
/// touches anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BringIntent {
    /// The working copy at the destination predates this attempt (an
    /// earlier fetch put it there; `update` pulls from its own
    /// recorded origin): update it IN PLACE. A failure — the network
    /// blinked, the ref went missing — surfaces the error and the copy
    /// stays exactly where it was; it is never deleted. Any failover
    /// afterwards runs as a [`BringIntent::SwitchSource`], which
    /// replaces the copy only after the new clone has fully succeeded.
    RefreshExisting,
    /// This attempt serves a DIFFERENT source than the copy on disk —
    /// the next mirror in the chain, another registry: clone into a
    /// temporary sibling and swap it in only on success. A failed or
    /// interrupted switch leaves the previous copy exactly as it was.
    SwitchSource,
}

impl GitPerPackageRegistry {
    /// Cache publication uses this existing bring owner. The verified candidate
    /// is copied beside the target before a cooperative move-aside/rename; no
    /// Git command runs against restored user configuration.
    pub(super) fn publish_native_cache(
        &self,
        stage: &crate::git_backend::shell::native_admission::NativeMaterialisation,
        destination: &Path,
    ) -> Result<(), RegistryError> {
        let destination = std::path::absolute(destination).map_err(|source| RegistryError::Io {
            path: destination.into(),
            source,
        })?;
        let refuse = |reason: String| RegistryError::PreparedInPlace {
            path: destination.clone(),
            reason,
        };
        if stage.destination() != destination {
            return Err(refuse(
                "cache candidate belongs to another destination".into(),
            ));
        }
        stage.validate_before_publication()?;
        let parent = destination
            .parent()
            .ok_or_else(|| refuse("cache destination has no parent".into()))?;
        fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
            path: parent.into(),
            source,
        })?;
        let temporary = tempfile::Builder::new()
            .prefix(".native-cache-")
            .tempdir_in(parent)
            .map_err(|source| RegistryError::Io {
                path: parent.into(),
                source,
            })?;
        let candidate = temporary.path().join("candidate");
        stage.copy_for_cache(&candidate)?;
        // copy_for_cache revalidates the exact old source/destination after the
        // potentially long copy and hash, immediately before publication.
        let backup = temporary.path().join("previous");
        let existed = match fs::symlink_metadata(&destination) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(source) => {
                return Err(RegistryError::Io {
                    path: destination.clone(),
                    source,
                });
            }
        };
        if existed {
            fs::rename(&destination, &backup).map_err(|source| RegistryError::Io {
                path: destination.clone(),
                source,
            })?;
        }
        let publish = if fs::symlink_metadata(&destination).is_ok() {
            Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "cache destination acquired another occupant",
            ))
        } else {
            fs::rename(&candidate, &destination)
        };
        if let Err(error) = publish {
            if existed {
                if fs::symlink_metadata(&destination).is_ok() {
                    let kept = temporary.keep();
                    return Err(refuse(format!(
                        "cache publication failed: {error}; rollback conflict; replacement preserved, backup retained at {}",
                        kept.display()
                    )));
                }
                if let Err(undo) = fs::rename(&backup, &destination) {
                    let kept = temporary.keep();
                    return Err(refuse(format!(
                        "cache publication failed: {error}; restore failed: {undo}; backup retained at {}",
                        kept.display()
                    )));
                }
            }
            return Err(RegistryError::Io {
                path: destination,
                source: error,
            });
        }
        Ok(())
    }
    /// Bring the per-package clone at `clone_dir` to `refname`,
    /// dispatching on how this attempt relates to the copy already on
    /// disk. Used by [`Self::ensure_clone_against_sources`] and the
    /// mirror-fallback variants of [`Self::fetch`] /
    /// [`Self::refresh_package`].
    ///
    /// Three shapes, three behaviours:
    ///
    /// - **A working copy exists and this is its refresh** —
    ///   [`BringIntent::RefreshExisting`]: `update` in place, and a
    ///   failure is returned, never repaired by destruction
    ///   (PROP-010 §2.6, `REFRESH-HAPPENS-IN-PLACE`).
    /// - **A working copy exists and this attempt is a different
    ///   source** — [`BringIntent::SwitchSource`]:
    ///   [`Self::switch_source_at`] clones beside and swaps on
    ///   success (`A-SOURCE-SWITCH-CLONES-BESIDE-AND-SWAPS`).
    /// - **The place is empty** — neither refresh nor switch: a plain
    ///   [`Self::bootstrap_fresh_at`] straight into place. An absent
    ///   copy is not an operation on a copy at all.
    pub(super) fn bring_clone_to_ref(
        &self,
        operation: &GitOperation,
        url: &str,
        refname: &str,
        clone_dir: &Path,
        intent: BringIntent,
    ) -> Result<(), RegistryError> {
        self.backend.preflight_materialisation()?;
        let working_copy = clone_dir.join(".git").exists();
        match (intent, working_copy) {
            (BringIntent::RefreshExisting, true) => {
                // REFRESH-HAPPENS-IN-PLACE: a failed refresh is
                // retried or repaired where the copy stands. It is not
                // grounds for destroying anything — not here, and not
                // as a side effect of a later attempt against another
                // source.
                if let Err(e) = self.backend.update(clone_dir, refname) {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        url = %url,
                        error = %e,
                        "update on existing clone failed; the copy stays — \
                         failover, if any, is a source switch"
                    );
                    // The refresh pulls from the clone's own recorded
                    // origin, so git names it `origin` and nothing else:
                    // the endpoint an operator can act on is the one
                    // this chain chose, which only the funnel knows.
                    return Err(self.git_failure(operation.clone(), url, e));
                }
                Ok(())
            }
            // An empty place is neither a refresh nor a switch —
            // bootstrap straight into it. (Debris without `.git` from
            // a prior failed bootstrap is not a copy and is cleaned by
            // `bootstrap_fresh_at` itself.)
            (_, false) => self.bootstrap_fresh_at(operation, url, refname, clone_dir),
            (BringIntent::SwitchSource, true) => {
                self.switch_source_at(operation, url, refname, clone_dir)
            }
        }
    }

    /// Bootstrap a fresh clone into `dest`, which must be empty or
    /// absent — an empty place is neither a refresh nor a switch, and
    /// half-populated debris from a prior failed bootstrap (a dir with
    /// no `.git`) is not a copy, so it is cleaned here.
    ///
    /// PROP-002 §2.2.1 — under `auth = "token-env"` the bootstrap is
    /// performed with a credentialised URL, then the recorded origin
    /// URL is rewritten to the plain (token-free) form so the
    /// freshly-cloned `.git/config` does NOT carry the token on disk.
    /// Subsequent `update` calls hit the plain origin (and 401 on a
    /// still-private host); the retry path handles that as a source
    /// switch — clone beside, swap on success — never as a delete. The
    /// token only ever lives in memory and inside the spawned
    /// `git clone` process.
    fn bootstrap_fresh_at(
        &self,
        operation: &GitOperation,
        url: &str,
        refname: &str,
        dest: &Path,
    ) -> Result<(), RegistryError> {
        if dest.exists() {
            fs::remove_dir_all(dest).map_err(|source| RegistryError::Io {
                path: dest.to_path_buf(),
                source,
            })?;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let plain_url = strip_git_plus_prefix(url);
        let fetch_url = self.selected_credentialed_url(plain_url)?;
        self.backend
            .bootstrap(&fetch_url, refname, dest)
            .map_err(|error| self.git_failure(operation.clone(), plain_url, error))?;
        if self.effective_token_value().is_some() {
            self.backend.set_remote_url(dest, "origin", plain_url)?;
        }
        Ok(())
    }

    /// Switch the source of the working copy at `dest` to `url` — the
    /// ONLY operation that downloads from scratch, and even it clones
    /// into a temporary sibling `<dest>.switch-tmp-<pid>` (same parent
    /// directory, so the closing `rename` stays on one volume and is
    /// atomic) and replaces the previous copy only after the new clone
    /// has fully succeeded (PROP-010 §2.6,
    /// `A-SOURCE-SWITCH-CLONES-BESIDE-AND-SWAPS`).
    ///
    /// A failed or interrupted switch removes the temp and leaves the
    /// previous copy exactly as it was — «delete and re-download» as
    /// the response to any hiccup is what the ten-gigabyte dependency
    /// cannot afford (`THE-TEN-GIGABYTE-TEST`). The swap itself is
    /// remove-then-rename (a rename over an existing directory is
    /// refused outright), so the instant between the two is the only
    /// moment the destination is absent.
    fn switch_source_at(
        &self,
        operation: &GitOperation,
        url: &str,
        refname: &str,
        dest: &Path,
    ) -> Result<(), RegistryError> {
        let no_parent = || RegistryError::Io {
            path: dest.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "switch destination has no parent directory",
            ),
        };
        let parent = dest.parent().ok_or_else(no_parent)?.to_path_buf();
        let file = dest.file_name().ok_or_else(no_parent)?.to_string_lossy();
        fs::create_dir_all(&parent).map_err(|source| RegistryError::Io {
            path: parent.clone(),
            source,
        })?;
        let tmp = parent.join(format!("{file}.switch-tmp-{}", std::process::id()));
        let switched = self
            .bootstrap_fresh_at(operation, url, refname, &tmp)
            .and_then(|()| {
                // The new clone fully succeeded — NOW, and only now, the
                // previous copy goes.
                if dest.exists() {
                    fs::remove_dir_all(dest).map_err(|source| RegistryError::Io {
                        path: dest.to_path_buf(),
                        source,
                    })?;
                }
                fs::rename(&tmp, dest).map_err(|source| RegistryError::Io {
                    path: dest.to_path_buf(),
                    source,
                })
            });
        if switched.is_err() {
            // The switch failed: the temp is debris; the previous copy
            // never left.
            let _ = fs::remove_dir_all(&tmp);
        }
        switched
    }
}
