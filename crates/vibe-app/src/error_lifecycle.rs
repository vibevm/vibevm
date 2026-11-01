//! Errors of the installation lifecycle beyond one install: requirements
//! between application packages, tree removal, environment steps and
//! rollback (`PROP-065#app-dependencies`, `#uninstall-tree`, `#env-step`,
//! `#update-and-rollback`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#uninstall-tree");

use specmark::spec;
use thiserror::Error;

/// Requirement resolution, removal of a tree, env steps and rollback.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#uninstall-tree")]
pub enum LifecycleError {
    #[error(
        "requirement `{requirement}` of `{key}` cannot be resolved: {reason}; nothing was downloaded or run \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies; fix: check the requirement, \
         the --registry path or the configured registries)"
    )]
    RequirementUnresolved {
        key: String,
        requirement: String,
        reason: String,
    },
    #[error(
        "the requirements form a cycle: {chain} (violates spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies; \
         fix: break the cycle in the packages' requires lists)"
    )]
    RequirementCycle { chain: String },
    #[error(
        "`{key}` is required at runtime scope by {dependents}; nothing was removed \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#uninstall-tree; fix: uninstall the dependents first, \
         or rerun with --cascade to remove them too after reviewing --plan-only)"
    )]
    Dependents { key: String, dependents: String },
    #[error(
        "`{key}` has no kept previous installation to roll back to (violates spec://org.vibevm.core/vibevm/common/PROP-065#update-and-rollback; \
         fix: a previous version is kept only after `vibe app update` and until the next update, --purge-previous or autoremove)"
    )]
    NoPrevious { key: String },
    #[error(
        "env step `{step}` of `{key}` failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-065#env-step; \
         fix: check that the startup files and the settings root are writable)"
    )]
    Environment {
        key: String,
        step: String,
        reason: String,
    },
    #[error(
        "env step `{step}` of `{key}` has no user environment to change in this context \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#env-step; fix: run the installation through the vibe CLI)"
    )]
    NoEnvironment { key: String, step: String },
    #[error(
        "`{key}` is already installed with {installed}, and the imported set asks for {wanted}; it was left as it is \
         (violates spec://org.vibevm.core/vibevm/common/PROP-059#portable-set; fix: change it with `vibe app modify {key}`, \
         or import with --skip-installed to keep the installed choice)"
    )]
    ChoiceDiffers {
        key: String,
        installed: String,
        wanted: String,
    },
    #[error(
        "the application set `{file}` cannot be read: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#portable-set; \
         fix: give a file `vibe app export` wrote, protocol `vibe-apps/1`)"
    )]
    PortableInvalid { file: String, reason: String },
}
