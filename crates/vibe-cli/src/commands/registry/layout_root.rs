//! The tree of a directory-layout registry (`monorepo`, `localdir`) for a
//! command that reads its index (PROP-002 `#registry-index`): a localdir is
//! its directory; a monorepo is cloned through the operation's controlled
//! transport, source policy and remap Locator — the same clone resolution
//! makes.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-index");

use specmark::spec;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};
use vibe_core::manifest::{MirrorSection, RegistrySection, RegistryType};
use vibe_index::layout::LayoutIdentity;
use vibe_registry::RegistryError;
use vibe_registry::multi_registry_resolver::RegistrySource;

/// An opened layout registry; holds the clone alive while it is read.
pub(crate) struct LayoutRoot {
    root: PathBuf,
    identity: LayoutIdentity,
    _source: Option<Arc<RegistrySource>>,
}

impl LayoutRoot {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn identity(&self) -> &LayoutIdentity {
        &self.identity
    }
}

/// Whether `reg` is a `monorepo` or `localdir` registry.
pub(crate) fn is_layout(reg: &RegistrySection) -> bool {
    matches!(
        reg.checked_type(),
        Ok(RegistryType::Monorepo | RegistryType::Localdir)
    )
}

/// Open the tree of the layout registry `reg`.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-index")]
pub(crate) fn open(
    reg: &RegistrySection,
    mirrors: &[MirrorSection],
    operation: &vibe_package_source::SourceOperation,
    offline: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<LayoutRoot> {
    match reg.checked_type()? {
        RegistryType::Localdir => {
            let root = vibe_registry::localdir_path(&reg.url)?;
            if !root.is_dir() {
                bail!(
                    "registry `{}` names the directory `{}`, which does not exist \
                     (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type; \
                     fix: mount or create it, or correct the registry url)",
                    reg.name,
                    root.display()
                );
            }
            Ok(LayoutRoot {
                root,
                identity: LayoutIdentity {
                    registry: reg.name.clone(),
                    registry_url: reg.url.clone(),
                    branch: None,
                },
                _source: None,
            })
        }
        RegistryType::Monorepo => {
            let resolver = vibe_package_source::registry_resolver(
                operation,
                vec![reg.clone()],
                mirrors.to_vec(),
                offline,
                progress,
            )?;
            let source = resolver
                .selected_registry_source(&reg.name)
                .map_err(refusal_first)?;
            let root = match source.as_ref() {
                RegistrySource::Monorepo(mono) => {
                    mono.local().map_err(refusal_first)?.root().to_path_buf()
                }
                _ => bail!(
                    "internal: registry `{}` did not open as a monorepo",
                    reg.name
                ),
            };
            Ok(LayoutRoot {
                root,
                identity: LayoutIdentity {
                    registry: reg.name.clone(),
                    registry_url: reg.url.clone(),
                    branch: Some(reg.r#ref.clone()),
                },
                _source: Some(source),
            })
        }
        other => bail!(
            "registry `{}` is a `{}` registry, not a monorepo or localdir",
            reg.name,
            other.as_str()
        ),
    }
}

/// Open an alias table's registry, naming the errand only for git failures.
/// Policy and cancellation retain their original typed error identities.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
pub(crate) fn open_alias(
    reg: &RegistrySection,
    mirrors: &[MirrorSection],
    operation: &vibe_package_source::SourceOperation,
    offline: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<LayoutRoot> {
    struct Unrendered;
    impl vibe_core::progress::ProgressObserver for Unrendered {
        fn observe(&self, _: vibe_core::progress::ProgressEvent) {}
    }
    // A real observer prevents nested transfer scopes from inheriting the
    // visible parent. Control and accounting remain the invocation's own.
    let scoped = vibe_core::progress::Progress::new(Arc::new(Unrendered))
        .with_cancellation(progress.cancellation().clone())
        .with_transfers(progress.transfers().cloned());
    let parent = tracing::dispatcher::get_default(Clone::clone);
    let subscriber = AliasOpeningSubscriber { parent };
    let opened = tracing::subscriber::with_default(subscriber, || {
        vibe_registry::git_backend::transfers::observed(Some(&scoped), None, None, || {
            open(reg, mirrors, operation, offline, &scoped)
        })
    });
    opened.map_err(|error| {
        let git = match error.downcast_ref::<RegistryError>() {
            Some(RegistryError::Git(git)) => git,
            Some(RegistryError::GitOperation { detail }) => &detail.source,
            _ => return error,
        };
        use vibe_registry::git_backend::GitError;
        // Do not render argv, endpoint strings or arbitrary stderr. Even a
        // classified error may retain those fields for other consumers.
        let cause = match git {
            GitError::ContactRefused(_) => return error,
            GitError::CommandFailed { status, .. } => format!("git exited with status {status}"),
            GitError::NativeMetadata { status: Some(status), .. } => {
                format!("git metadata read failed with exit {status}")
            }
            GitError::NativeMetadata { status: None, .. } => "git metadata read failed".into(),
            GitError::RepoNotFound { .. } => "git repository not found or access not granted".into(),
            GitError::AuthFailed { .. } => "git authentication rejected".into(),
            GitError::SshHostKeyRejected => "git SSH host key rejected".into(),
            GitError::SshAuthenticationRejected => "git SSH authentication rejected".into(),
            GitError::HttpRedirected { status, .. } => format!("git received HTTP redirect {status}"),
            GitError::TlsTrust { .. } => "git TLS trust check failed".into(),
            GitError::NetworkUnreachable { .. } => "git network or DNS failure".into(),
            GitError::RefNotFound { .. } => "git branch or ref not found".into(),
            GitError::NotInstalled => "git executable is unavailable".into(),
            GitError::Io { source, .. } => format!("git could not start ({:?})", source.kind()),
            // Preparation, policy and other non-contact failures are not
            // relabelled as an unreachable registry.
            _ => return error,
        };
        anyhow::anyhow!(
            "registry_unreachable: could not open registry {} to read its alias table \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
             fix: `vibe registry test`, `vibe dl --test-proxy`): {cause}",
            reg.name,
        )
    })
}

/// Forward the invocation's tracing unchanged except the duplicate capture
/// warning while the alias opener owns its eventual refusal.
struct AliasOpeningSubscriber {
    parent: tracing::Dispatch,
}

impl tracing::Subscriber for AliasOpeningSubscriber {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        self.parent.enabled(metadata)
    }

    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        self.parent.new_span(span)
    }

    fn record(&self, span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        self.parent.record(span, values);
    }

    fn record_follows_from(&self, span: &tracing::span::Id, follows: &tracing::span::Id) {
        self.parent.record_follows_from(span, follows);
    }

    fn event(&self, event: &tracing::Event<'_>) {
        let metadata = event.metadata();
        let capture = metadata.target() == "vibe_registry"
            && metadata.module_path() == Some("vibe_registry::git_registry::native")
            && metadata.file().is_some_and(|file| {
                file.replace('\\', "/").ends_with("git_registry/native.rs")
            })
            && *metadata.level() == tracing::Level::WARN;
        struct Message(bool);
        impl tracing::field::Visit for Message {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{value:?}") == "registry snapshot candidate failed";
                }
            }
        }
        if capture {
            let mut message = Message(false);
            event.record(&mut message);
            if message.0 {
                return;
            }
        }
        self.parent.event(event);
    }

    fn enter(&self, span: &tracing::span::Id) {
        self.parent.enter(span);
    }

    fn exit(&self, span: &tracing::span::Id) {
        self.parent.exit(span);
    }

    fn clone_span(&self, span: &tracing::span::Id) -> tracing::span::Id {
        self.parent.clone_span(span)
    }

    fn try_close(&self, span: tracing::span::Id) -> bool {
        self.parent.try_close(span)
    }

    fn current_span(&self) -> tracing_core::span::Current {
        self.parent.current_span()
    }
}

/// A contact refusal travels as itself, so a caller can tell a policy
/// skip from a broken registry.
pub(crate) fn refusal_first(error: RegistryError) -> anyhow::Error {
    match error {
        RegistryError::ContactRefused(refusal) => refusal.into(),
        other => other.into(),
    }
}
