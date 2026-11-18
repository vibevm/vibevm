//! Opening a monorepo under the controlled transport: reuse first, ask the
//! endpoint second, capture last (PROP-073 `#NATIVE-PREPARATION`).
//!
//! The decision this module makes is the whole of the cost difference
//! between a closed network that works and one that does not. Before it,
//! every command captured the repository from scratch — clone, object walk,
//! path probe, checkout, copy — and more than once per command. After it, a
//! command that finds nothing changed performs **one** `ls-remote`, or none
//! at all inside the freshness window, and reads the tree it already has.
//!
//! Three questions gate a reuse, and all three must answer yes:
//!
//! * **Is the endpoint still at the object we hold?** One `ls-remote`,
//!   through the same admitted candidate chain a capture uses, compared
//!   against the object id the state records. Inside the freshness window —
//!   or under `--offline`, which forbids the contact — the question is
//!   skipped and the held object is taken as current, exactly as the
//!   ordinary transport's clone and the template version list do
//!   (PROP-075 `#TEMPLATE-CACHE`). `--refresh` always asks.
//! * **Is the tree we hold the tree we recorded?** Its per-file sha256
//!   witness, recomputed. This is not a timer: a tree that was edited,
//!   truncated or half-written is not reused at any age.
//! * **Would the admission probe still ask the same question?** The
//!   attribute and conversion inputs the probe reads, digested. An edited
//!   global `gitattributes`, a changed `core.autocrlf` or a replaced Git is
//!   a different question, and a tree admitted under the old one is
//!   re-probed rather than inherited.
//!
//! A no to any of them costs work and never skips a check: the endpoint
//! moved or the tree is gone, and the capture runs — with a `fetch` of the
//! new objects when the durable object store is this capability's own, and
//! with a full clone and a full `fsck --full` when it is not.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::RegistryError;
use crate::git_backend::GitBackend;

use super::acquire::native_snapshot;
use super::native_cache::{NativeCache, NativeSnapshotState, hex, tree_digest, within_window};

/// A durable snapshot this process may read.
#[derive(Debug, Clone)]
pub(super) struct Retained {
    pub root: PathBuf,
    pub tree: PathBuf,
    pub commit: String,
    pub served_url: String,
}

/// Whether the endpoint has to be asked, and what the answer was.
#[derive(Clone)]
enum Endpoint {
    /// Not asked: inside the freshness window, or `--offline`.
    NotAsked,
    /// Asked, and it advertises this object at this namespace.
    Advertised { object: String, namespace: String },
    /// Asked, and every admitted candidate refused the question. The
    /// capture below asks again through its own chain and reports the real
    /// failure, rather than this one being read as "unchanged".
    Unknown,
}

/// Open `url` at `refname` from the durable cache under `cache_root`,
/// capturing it when no reuse holds.
pub(super) fn open(
    url: &str,
    candidates: &[String],
    refname: &str,
    cache_root: &Path,
    backend: Arc<dyn GitBackend>,
    freshness_secs: u64,
) -> Result<super::GitMonorepoRegistry, RegistryError> {
    open_with_posture(
        url,
        candidates,
        refname,
        cache_root,
        backend,
        freshness_secs,
        posture(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn open_with_posture(
    url: &str,
    candidates: &[String],
    refname: &str,
    cache_root: &Path,
    backend: Arc<dyn GitBackend>,
    freshness_secs: u64,
    posture: Posture,
) -> Result<super::GitMonorepoRegistry, RegistryError> {
    // Revalidate through this caller's backend: a process-global memo keyed
    // only by URL would leak a previous operation's admission/configuration
    // into another context in a long-lived server.
    let progress = crate::git_backend::transfers::current()
        .progress
        .unwrap_or_default();
    let task = progress.task(format!(
        "Opening registry {}",
        vibe_core::progress::transfer::safe_remote(url)
    ));
    task.detail("waiting for registry cache lock");
    // Different locked revisions must not repeatedly overwrite one state's
    // freshness record or force new generations for the same pinned commit.
    use sha2::Digest;
    let reference_key = hex(&sha2::Sha256::digest(refname.as_bytes()));
    let reference_root = cache_root.join(format!("ref-{reference_key}"));
    let cache = NativeCache::at(&reference_root, url)?;
    // Serialize state validation/publication as well as object writes. Trees
    // are immutable generations and remain readable after this guard drops.
    let _lock = cache.lock()?;
    task.detail("validating retained registry snapshot and freshness");
    let mut asked = None;
    if let Some(retained) = reuse(
        &cache,
        refname,
        candidates,
        backend.as_ref(),
        freshness_secs,
        posture,
        &mut asked,
    )? {
        task.detail("cache hit: verified snapshot unchanged; no clone or fetch");
        task.finish();
        return super::GitMonorepoRegistry::from_retained(
            url, refname, backend, retained, cache_root, candidates,
        );
    }
    if posture == Posture::Offline {
        return Err(RegistryError::MalformedMeta {
            path: cache.root().to_path_buf(),
            reason: "offline registry snapshot is absent or no longer valid; refresh it online before retrying --offline".into(),
        });
    }
    // The namespace this run already read from the advertisement, so the
    // capture does not ask for it a second time.
    let namespace = match &asked {
        Some(Endpoint::Advertised { namespace, .. }) => Some(namespace.clone()),
        _ => None,
    };
    task.detail("cache miss or changed registry: acquiring and verifying snapshot");
    let result =
        crate::git_backend::transfers::observed(Some(&task.progress()), None, None, || {
            capture(
                &cache,
                url,
                candidates,
                refname,
                backend,
                namespace.as_deref(),
                cache_root,
            )
        });
    match &result {
        Ok(_) => task.finish(),
        Err(error) => task.fail(error.to_string()),
    }
    result
}

/// The durable snapshot, when all three reuse questions answer yes.
#[allow(clippy::too_many_arguments)]
fn reuse(
    cache: &NativeCache,
    refname: &str,
    candidates: &[String],
    backend: &dyn GitBackend,
    freshness_secs: u64,
    posture: Posture,
    known: &mut Option<Endpoint>,
) -> Result<Option<Retained>, RegistryError> {
    let Some(state) = cache.state(refname) else {
        return Ok(None);
    };
    if state.served_url != cache.url() && !candidates.contains(&state.served_url) {
        return Ok(None);
    }
    // The bytes we hold came from this candidate; the operation's own policy
    // must still admit it, or they are not bytes this run may read.
    if backend
        .preflight_selected_address(&state.served_url)
        .is_err()
    {
        tracing::debug!(target: "vibe_registry::monorepo", url = %cache.url(), "the retained snapshot's serving endpoint is no longer admitted");
        return Ok(None);
    }
    let endpoint = match known.clone() {
        // Asked once per run: the second look, under the write lock, is
        // about this process racing another, not about the endpoint moving
        // again in the meantime.
        Some(endpoint) => endpoint,
        None => {
            let asked = ask(cache, &state, candidates, backend, freshness_secs, posture)?;
            *known = Some(asked.clone());
            asked
        }
    };
    match &endpoint {
        Endpoint::NotAsked => {}
        Endpoint::Advertised { object, .. } if *object == state.object => {}
        Endpoint::Advertised { object, .. } => {
            tracing::info!(target: "vibe_registry::monorepo", url = %cache.url(), held = %state.object, advertised = %object, "the registry endpoint moved; refreshing the snapshot");
            return Ok(None);
        }
        Endpoint::Unknown => return Ok(None),
    }
    let Some(digest) = tree_digest(&cache.tree(&state))? else {
        tracing::debug!(target: "vibe_registry::monorepo", url = %cache.url(), "the retained snapshot tree is absent");
        return Ok(None);
    };
    if digest != state.tree_digest {
        tracing::warn!(target: "vibe_registry::monorepo", url = %cache.url(), "the retained snapshot tree does not match its record; recapturing");
        return Ok(None);
    }
    match backend.native_admission_inputs()? {
        Some(inputs) if hex(&inputs) == state.admission => {}
        Some(_) => {
            tracing::info!(target: "vibe_registry::monorepo", url = %cache.url(), "the attribute inputs of the admission probe changed; recapturing");
            return Ok(None);
        }
        // A backend with no admission probe has no verdict to remember, and
        // nothing the recorded value could be compared against.
        None => return Ok(None),
    }
    if matches!(endpoint, Endpoint::Advertised { .. }) {
        cache.touch(&state);
    }
    tracing::debug!(target: "vibe_registry::monorepo", url = %cache.url(), commit = %state.commit, "reusing the retained monorepo snapshot");
    Ok(Some(Retained {
        root: cache.root().to_path_buf(),
        tree: cache.tree(&state),
        commit: state.commit,
        served_url: state.served_url,
    }))
}

/// Ask the endpoint for the object it advertises, unless this posture or the
/// freshness window says not to.
fn ask(
    cache: &NativeCache,
    state: &NativeSnapshotState,
    candidates: &[String],
    backend: &dyn GitBackend,
    freshness_secs: u64,
    posture: Posture,
) -> Result<Endpoint, RegistryError> {
    match posture {
        // A posture that forbids the network reads what is held, whatever
        // its age — the template cache's rule, for the same reason.
        Posture::Offline => return Ok(Endpoint::NotAsked),
        Posture::Cached if within_window(state, freshness_secs) => {
            tracing::debug!(target: "vibe_registry::monorepo", url = %cache.url(), "the retained snapshot is inside its freshness window");
            return Ok(Endpoint::NotAsked);
        }
        Posture::Cached | Posture::Refresh => {}
    }
    let mut first_error = None;
    for candidate in std::iter::once(cache.url()).chain(candidates.iter().map(String::as_str)) {
        match backend.native_advertised_tip(candidate, &state.reference) {
            Ok(Some((object, namespace))) => {
                return Ok(Endpoint::Advertised { object, namespace });
            }
            // No controlled transport: nothing to ask, and nothing a
            // recorded object could be compared against.
            Ok(None) => return Ok(Endpoint::NotAsked),
            Err(error) => {
                tracing::debug!(target: "vibe_registry::monorepo", url = %candidate, %error, "the registry endpoint could not be asked");
                first_error.get_or_insert(error);
            }
        }
    }
    match first_error {
        Some(_) => Ok(Endpoint::Unknown),
        None => Ok(Endpoint::NotAsked),
    }
}

/// Capture the repository into the durable cache and record what it holds.
#[allow(clippy::too_many_arguments)]
fn capture(
    cache: &NativeCache,
    url: &str,
    candidates: &[String],
    refname: &str,
    backend: Arc<dyn GitBackend>,
    namespace: Option<&str>,
    cache_root: &Path,
) -> Result<super::GitMonorepoRegistry, RegistryError> {
    let mut first_error = None;
    for candidate in std::iter::once(url).chain(candidates.iter().map(String::as_str)) {
        tracing::info!(target: "vibe_registry::monorepo", url = %candidate, reference = %refname, "capturing monorepo snapshot");
        // Only the candidate the advertisement came from may reuse its
        // namespace; a fallback candidate advertises its own.
        let namespace = (candidate == cache.url()).then_some(namespace).flatten();
        match native_snapshot(
            backend.as_ref(),
            candidate,
            refname,
            cache.root(),
            true,
            namespace,
        ) {
            Ok(captured) => {
                let digest = tree_digest(&captured.tree)?
                    .ok_or_else(|| RegistryError::MissingRoot(captured.tree.clone()))?;
                // A backend with no admission probe records no verdict, and
                // a state with no verdict is never reused.
                let admission = backend
                    .native_admission_inputs()?
                    .map(|inputs| hex(&inputs))
                    .unwrap_or_default();
                cache.write_state(&NativeSnapshotState {
                    schema: super::native_cache::SCHEMA,
                    url: normalize_url(url),
                    reference: refname.to_owned(),
                    refname: captured.refname.clone(),
                    served_url: candidate.to_owned(),
                    object: captured.object.clone(),
                    commit: captured.commit.clone(),
                    generation: captured
                        .tree
                        .parent()
                        .and_then(Path::file_name)
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| RegistryError::MissingRoot(captured.tree.clone()))?
                        .to_owned(),
                    tree_digest: digest,
                    admission,
                    checked_at: vibe_core::timestamp::now_utc(),
                })?;
                let retained = Retained {
                    root: cache.root().to_path_buf(),
                    tree: captured.tree,
                    commit: captured.commit,
                    served_url: candidate.to_owned(),
                };
                return super::GitMonorepoRegistry::from_retained(
                    url, refname, backend, retained, cache_root, candidates,
                );
            }
            Err(error) => {
                tracing::warn!(target: "vibe_registry", url = %candidate, %error, "registry snapshot candidate failed");
                first_error.get_or_insert(error);
            }
        }
    }
    Err(first_error.unwrap_or_else(|| RegistryError::MissingRoot(cache.root().to_path_buf())))
}

pub(super) use crate::registry_cache::normalize_url;
pub use crate::templates::Posture;

/// This invocation's cache posture, read from the one layer that already
/// resolved the whole `--offline` / `VIBE_OFFLINE` / `[net] offline` ladder
/// and the `--refresh` flag beside it (PROP-075 `#TEMPLATE-CACHE`). An
/// invocation that installed no such layer is `Cached`, which is the
/// default posture everywhere.
fn posture() -> Posture {
    crate::templates::active()
        .map(|layer| layer.posture())
        .unwrap_or_default()
}
