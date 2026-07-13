//! Registry-redirect stubs (PROP-002 §2.4.2) — probing a served tag
//! for `vibe-redirect.toml`, following the marker to its target with
//! the hop-limit guard, and fetching redirect-resolved content.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

use super::*;

/// Probe the `(kind, name)` slot in `reg` at `tag` for a
/// `vibe-redirect.toml` marker. Returns `Some(parsed)` when the
/// marker exists; `None` when the file is absent (the common case
/// — non-stub package). Surfaces parse errors and other I/O errors
/// directly. Cheap: one extra `git archive` call per registry-walk
/// success.
pub(super) fn try_fetch_redirect(
    backend: &Arc<dyn GitBackend>,
    reg: &GitPackageRegistry,
    resolved: &ResolvedPackage,
    tag: &str,
) -> Result<Option<RedirectFile>, RegistryError> {
    try_fetch_redirect_for_url(backend, reg, &resolved.group, &resolved.name, tag)
}

/// Lower-level form of `try_fetch_redirect`: take an already-built
/// `GitPackageRegistry` plus `(kind, name, ref)` and probe its repo
/// for a `vibe-redirect.toml`. Used both for the initial probe at
/// the stub layer and the hop-limit check at the target.
///
/// Two-path read shape — same idea as `fetch_dep_manifest`:
///
/// 1. `git archive --remote=<url> <ref> -- vibe-redirect.toml` is the
///    cheap, no-clone read. Works against `file://` and the handful
///    of hosts that expose `upload-archive`.
/// 2. When the host refuses `upload-archive` (GitHub, by design) the
///    archive call returns `ArchiveUnsupported`. Fall back to a
///    shallow clone of the repo at `refname` and read the file from
///    the working tree. The clone directory is the same the install
///    pipeline would use later, so this is also pre-warming.
///
/// Returns `Ok(None)` when neither path finds the marker — the common
/// "non-stub package" case where `vibe-redirect.toml` simply isn't
/// part of the package payload.
fn try_fetch_redirect_for_url(
    backend: &Arc<dyn GitBackend>,
    reg: &GitPackageRegistry,
    group: &Group,
    name: &str,
    refname: &str,
) -> Result<Option<RedirectFile>, RegistryError> {
    let plain_url = reg.package_repo_url(group, name)?;
    let fetch_url = reg.credentialed_url(&plain_url);
    let bytes = match backend.fetch_file_at_ref(
        strip_git_plus_prefix(&fetch_url),
        refname,
        RedirectFile::FILENAME,
    ) {
        Ok(b) => b,
        Err(crate::git_backend::GitError::FileNotFoundInRef { .. }) => return Ok(None),
        Err(crate::git_backend::GitError::ArchiveUnsupported { .. }) => {
            // Host refuses `upload-archive` — the GitHub case. Fall
            // back to a shallow clone and read the marker from the
            // clone's working tree. `refresh_package` reuses the
            // existing per-package clone if present; for a fresh
            // bucket it bootstraps. The token-discipline (M1.14)
            // applies — if the registry has `auth = "token-env"`,
            // the clone uses the credentialed URL and immediately
            // scrubs `.git/config` after bootstrap.
            reg.refresh_package(group, name, refname)?;
            let marker_path = reg
                .package_clone_dir(group, name)
                .join(RedirectFile::FILENAME);
            if !marker_path.exists() {
                return Ok(None);
            }
            std::fs::read(&marker_path).map_err(|source| RegistryError::Io {
                path: marker_path,
                source,
            })?
        }
        Err(other) => return Err(other.into()),
    };
    let r = parse_redirect_bytes(&bytes).map_err(|e| RegistryError::MalformedMeta {
        path: PathBuf::from(format!("{plain_url}@{refname}:{}", RedirectFile::FILENAME)),
        reason: e.to_string(),
    })?;
    Ok(Some(r))
}

impl MultiRegistryResolver {
    /// Follow a `vibe-redirect.toml` marker found in a registry stub
    /// repo (PROP-002 §2.4.2). The stub registry served a tag; we
    /// re-resolve against the redirect's `target_url` at the
    /// pass-through-tag (default — same tag as the stub) or the
    /// `pinned_ref` (when `ref_policy = "pinned"`). Hop limit = 1:
    /// if the target is itself a stub, raise
    /// `RedirectChainNotAllowed`.
    pub(super) fn follow_redirect(
        &self,
        pkgref: &PackageRef,
        stub_resolved: &ResolvedPackage,
        stub_reg: &GitPackageRegistry,
        redirect: &RedirectFile,
        stub_tag: &str,
    ) -> Result<MultiResolution, RegistryError> {
        let target_url = redirect.redirect.target_url.clone();
        // The wire parser rejects pinned-without-pinned_ref, but a
        // RedirectSection is also constructible programmatically (pub
        // fields), so the mismatch is reachable — diagnose, don't panic.
        let target_ref = match redirect.redirect.ref_policy {
            RefPolicy::PassThroughTag => stub_tag.to_string(),
            RefPolicy::Pinned => redirect.redirect.pinned_ref.clone().ok_or_else(|| {
                RegistryError::MalformedMeta {
                    path: PathBuf::from("vibe-redirect.toml"),
                    reason: format!(
                        "registry stub for `{}/{}` declares ref_policy=pinned but \
                         carries no pinned_ref",
                        stub_resolved.group, pkgref.name
                    ),
                }
            })?,
        };
        // Identity is `(group, name)`; the stub-resolved package carries
        // the group the registry resolved by (PROP-008).
        let group = &stub_resolved.group;
        let synthetic_name = format!("redirect-target-{}-{}", group, pkgref.name);
        let target_reg = GitPackageRegistry::open_single_package(
            &synthetic_name,
            &target_url,
            &target_ref,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            redirect.redirect.auth,
            redirect.redirect.token_env.as_deref(),
        )?;
        // Hop limit = 1: target cannot itself be a stub. Probe
        // `vibe-redirect.toml` at the target ref BEFORE attempting to
        // read the target's `vibe.toml` — a stub-only repo
        // carries only the marker, so reading the manifest first
        // would return `FileNotFoundInRef` and the chain detection
        // would never fire. Marker-first preserves the policy contract
        // independent of what the target's content shape happens to
        // be.
        let target_redirect = try_fetch_redirect_for_url(
            &self.backend,
            &target_reg,
            group,
            &stub_resolved.name,
            &target_ref,
        )?;
        if target_redirect.is_some() {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!(
                    "{target_url}@{target_ref}:{}",
                    RedirectFile::FILENAME
                )),
                reason: format!(
                    "redirect chain not allowed: stub `{}` redirects to `{target_url}` which is itself a stub (hop limit = 1, PROP-002 §2.4.2)",
                    stub_reg.package_repo_url(group, &stub_resolved.name)?,
                ),
            });
        }
        let target_manifest =
            target_reg.fetch_manifest_at_ref(group, pkgref.name.as_str(), &target_ref)?;
        let target_meta =
            target_manifest
                .require_package()
                .map_err(|e| RegistryError::MalformedMeta {
                    path: PathBuf::from(format!(
                        "{target_url}@{target_ref}:{}",
                        Manifest::FILENAME
                    )),
                    reason: e.to_string(),
                })?;
        // Sanity: the target's `[package]` must declare the same
        // `(group, name)` identity the consumer asked for. Mismatch =
        // org owner pointed at the wrong target, refuse to install.
        // `kind` is metadata (PROP-008 §2.3) — not compared.
        if &target_meta.group != group || target_meta.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{target_url}@{target_ref}:{}", Manifest::FILENAME)),
                reason: format!(
                    "redirect target for `{}/{}` declares `{}/{}` — refusing to install",
                    group, pkgref.name, target_meta.group, target_meta.name
                ),
            });
        }
        let stub_url = stub_reg.package_repo_url(group, &stub_resolved.name)?;
        let resolved = ResolvedPackage {
            group: group.clone(),
            name: pkgref.name.to_string(),
            version: target_meta.version.clone(),
            source_dir: self.redirect_clone_dir(group, pkgref.name.as_str()),
        };
        Ok(MultiResolution {
            resolved,
            // Registry name from the stub layer — that's the surface
            // the consumer's `vibe.toml` `[[registry]]` named.
            registry_name: Some(stub_reg.name().to_string()),
            source_url: target_url,
            source_ref: Some(target_ref),
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            via_redirect: Some(stub_url),
            redirect_target_auth: redirect.redirect.auth,
            redirect_target_token_env: redirect.redirect.token_env.clone(),
        })
    }

    /// Where redirect-followed clones live —
    /// `<cache_root>/__redirects__/<group>.<name>/clone/`. Distinct
    /// from registry-served, override, and git-source clones so
    /// re-resolutions across modes do not share state. Keyed by
    /// `(group, name)` identity (PROP-008).
    fn redirect_clone_dir(&self, group: &Group, name: &str) -> PathBuf {
        self.cache_root
            .join("__redirects__")
            .join(format!("{group}_{name}"))
            .join("clone")
    }

    /// Fetch a redirect-resolved package — the target's content lives at
    /// `resolution.source_url`, fetched with auth from
    /// `resolution.redirect_target_auth` / `redirect_target_token_env`.
    /// The stub URL is preserved as `via_redirect` on the lockfile entry
    /// for diagnostic / auditing.
    pub(super) fn fetch_via_redirect(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let group = &resolution.resolved.group;
        let name = resolution.resolved.name.as_str();
        let target_url = resolution.source_url.clone();
        let refname =
            resolution
                .source_ref
                .clone()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: PathBuf::from(format!("{target_url}:{}", Manifest::FILENAME)),
                    reason:
                        "redirect resolution carries no source_ref — internal invariant violated"
                            .to_string(),
                })?;

        // Synthesise a single-package registry on the target URL with
        // the redirect's declared auth, so the M1.14 token-injection
        // + scrub-from-`.git/config` discipline applies here too.
        let synthetic_name = format!("redirect-target-{group}-{name}");
        let target_reg = GitPackageRegistry::open_single_package(
            &synthetic_name,
            &target_url,
            &refname,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            resolution.redirect_target_auth,
            resolution.redirect_target_token_env.as_deref(),
        )?;
        let plain_url = target_reg.package_repo_url(group, name)?;
        let credentialed = target_reg.credentialed_url(&plain_url);

        let clone_dir = self.redirect_clone_dir(group, name);
        ensure_clone_at(self.backend.as_ref(), &credentialed, &refname, &clone_dir)?;
        if credentialed != plain_url {
            self.backend
                .set_remote_url(&clone_dir, "origin", &plain_url)
                .ok();
        }

        let dest = project_cache
            .join(group.as_str())
            .join(name)
            .join(format!("v{}", resolution.resolved.version));
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|source| RegistryError::Io {
                path: dest.clone(),
                source,
            })?;
        }
        copy_dir_excluding_git(&clone_dir, &dest)?;
        let manifest_path = dest.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }
        let content_hash = compute_content_hash(&dest)?;

        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group: group.clone(),
                name: name.to_string(),
                version: resolution.resolved.version.clone(),
                source_dir: clone_dir,
            },
            cache_dir: dest,
            manifest,
            content_hash,
            source_uri: plain_url,
            // Lockfile records the stub registry's name so the entry
            // is associated with the registry the consumer's
            // `vibe.toml` named.
            registry_name: resolution.registry_name.clone(),
            source_ref: Some(refname),
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            via_redirect: resolution.via_redirect.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::multi_registry_resolver::test_support::*;

    #[test]
    fn resolve_follows_registry_redirect_pass_through_tag() {
        // M1.16 PROP-002 §2.4.2: a registry stub repo carries
        // vibe-redirect.toml at its root. The resolver detects the
        // marker, follows it, and returns a MultiResolution carrying
        // the target URL in source_url and the stub URL in via_redirect.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // Stub repo at the registry: tag v0.3.0 has a vibe-redirect.toml
        // pointing at the target URL. NO vibe.toml. The stub-side URL is
        // composed by the registry's `Fqdn` naming — `<group>.<name>`.
        let stub_url = "git@host:org-stub/org.vibevm_internal.git";
        fake.seed_tags(stub_url, vec!["v0.3.0".into()]);
        fake.seed_file(
            stub_url,
            "v0.3.0",
            "vibe-redirect.toml",
            br#"[redirect]
target_url = "git@host:external/flow-internal.git"
"#
            .to_vec(),
        );
        // Target repo at the external host: tag v0.3.0 has a real
        // vibe.toml. The target URL is verbatim from the marker — a
        // single-package registry, naming bypassed.
        let target_url = "git@host:external/flow-internal.git";
        fake.seed_tags(target_url, vec!["v0.3.0".into()]);
        fake.seed_file(
            target_url,
            "v0.3.0",
            "vibe.toml",
            manifest_text("internal", "flow", "0.3.0").into_bytes(),
        );
        let r = build_resolver(
            cache.path(),
            vec![registry_section("stub-org", "git@host:org-stub")],
            vec![],
            vec![],
            fake,
        );
        let p = PackageRef::parse("org.vibevm/internal").unwrap();
        let m = r.resolve(&p).expect("redirect-follow must succeed");
        assert_eq!(
            m.via_redirect.as_deref(),
            Some(stub_url),
            "via_redirect must carry the stub URL"
        );
        assert_eq!(
            m.source_url, target_url,
            "source_url must carry the target URL"
        );
        assert_eq!(m.source_ref.as_deref(), Some("v0.3.0"));
        assert!(!m.is_git_source);
        assert!(!m.overridden);
        assert_eq!(m.registry_name.as_deref(), Some("stub-org"));
    }

    #[test]
    fn resolve_redirect_chain_rejected_at_hop_two() {
        // Two stubs in sequence: the first redirects to a URL that is
        // itself a stub. Per PROP-002 §2.4.2 hop limit = 1, this is
        // rejected with `RedirectChainNotAllowed` (surfaced as
        // MalformedMeta with chain-not-allowed reason).
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // `stub_a` is the entry registry's per-package URL — composed by
        // `Fqdn` (`<group>.<name>`). `stub_b` is the verbatim target of
        // stub_a's marker; the resolver opens it as a single-package
        // registry, so its URL shape is arbitrary.
        let stub_a = "git@host:org-a/org.vibevm_foo.git";
        let stub_b = "git@host:org-b/flow-foo.git";
        fake.seed_tags(stub_a, vec!["v1.0.0".into()]);
        fake.seed_file(
            stub_a,
            "v1.0.0",
            "vibe-redirect.toml",
            format!("[redirect]\ntarget_url = \"{stub_b}\"\n").into_bytes(),
        );
        // Hop limit probe: target URL has its own vibe-redirect.toml
        // at the same tag. Chain rejected.
        fake.seed_file(
            stub_b,
            "v1.0.0",
            "vibe-redirect.toml",
            br#"[redirect]
target_url = "git@host:org-c/flow-foo.git"
"#
            .to_vec(),
        );
        // The target's vibe.toml is also seeded (some hop-2
        // detectors only check the redirect file; ours also fetches
        // the manifest first, so seed it to avoid noise).
        fake.seed_file(
            stub_b,
            "v1.0.0",
            "vibe.toml",
            manifest_text("foo", "flow", "1.0.0").into_bytes(),
        );
        let r = build_resolver(
            cache.path(),
            vec![registry_section("a", "git@host:org-a")],
            vec![],
            vec![],
            fake,
        );
        let p = PackageRef::parse("org.vibevm/foo").unwrap();
        let err = r.resolve(&p).expect_err("redirect chain must reject");
        let msg = err.to_string();
        assert!(
            msg.contains("redirect chain not allowed") || msg.contains("hop limit"),
            "expected hop-limit rejection, got: {msg}"
        );
    }

    #[test]
    fn resolve_redirect_pinned_uses_pinned_ref() {
        // ref_policy = "pinned" + pinned_ref = "v1.0.0": stub at
        // any tag should resolve to target's v1.0.0, regardless of
        // the stub's own tag.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // `stub_url` is registry-composed via `Fqdn` (`<group>.<name>`);
        // `target_url` is verbatim from the marker.
        let stub_url = "git@host:org-stub/org.vibevm_pinned.git";
        let target_url = "git@host:external/flow-pinned.git";
        // Stub has v9.9.9 tag (irrelevant — pinned overrides).
        fake.seed_tags(stub_url, vec!["v9.9.9".into()]);
        fake.seed_file(
            stub_url,
            "v9.9.9",
            "vibe-redirect.toml",
            br#"[redirect]
target_url = "git@host:external/flow-pinned.git"
ref_policy = "pinned"
pinned_ref = "v1.0.0"
"#
            .to_vec(),
        );
        // Target has v1.0.0 (the pinned ref).
        fake.seed_file(
            target_url,
            "v1.0.0",
            "vibe.toml",
            manifest_text("pinned", "flow", "1.0.0").into_bytes(),
        );
        let r = build_resolver(
            cache.path(),
            vec![registry_section("stub-org", "git@host:org-stub")],
            vec![],
            vec![],
            fake,
        );
        let p = PackageRef::parse("org.vibevm/pinned").unwrap();
        let m = r.resolve(&p).expect("pinned redirect must succeed");
        assert_eq!(m.source_ref.as_deref(), Some("v1.0.0"));
        assert_eq!(m.resolved.version.to_string(), "1.0.0");
        assert_eq!(m.via_redirect.as_deref(), Some(stub_url));
    }

    #[test]
    fn resolve_redirect_target_name_mismatch_rejected() {
        // Stub redirects to a target whose vibe.toml declares a
        // different `(group, name)` identity. Refuse — pulling code
        // under the wrong pkgref slot would silently misroute on disk.
        // `kind` is metadata (PROP-008 §2.3); the identity check that
        // catches this is the `name` mismatch.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let stub_url = "git@host:org-stub/org.vibevm_internal.git";
        let target_url = "git@host:external/some-other-pkg.git";
        fake.seed_tags(stub_url, vec!["v0.1.0".into()]);
        fake.seed_file(
            stub_url,
            "v0.1.0",
            "vibe-redirect.toml",
            format!("[redirect]\ntarget_url = \"{target_url}\"\n").into_bytes(),
        );
        fake.seed_file(
            target_url,
            "v0.1.0",
            "vibe.toml",
            manifest_text("something-else", "feat", "0.1.0").into_bytes(),
        );
        let r = build_resolver(
            cache.path(),
            vec![registry_section("stub-org", "git@host:org-stub")],
            vec![],
            vec![],
            fake,
        );
        let p = PackageRef::parse("org.vibevm/internal").unwrap();
        let err = r.resolve(&p).expect_err("identity mismatch must reject");
        assert!(
            err.to_string().contains("refusing to install"),
            "got: {err}"
        );
    }
}
