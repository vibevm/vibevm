//! The workspace crate's one refusal family, out of line per the
//! file-length budget — the same split `extension_world/errors.rs` and
//! `compile_trace/errors.rs` already make, for the same reason.
//!
//! Every display string ends with the Class-F machine tail —
//! `(violates spec://…; fix: …)` — so a failing run is navigable back to the
//! requirement without source access.

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;

use crate::extension_world::ExtensionWorldError;

/// Errors raised while discovering or loading a workspace.
///
/// Messages carry the offending path or pattern, so the operator knows
/// which manifest to repair, and every display string ends with the
/// Class-F machine tail — `(violates spec://…; fix: …)` — so a failing
/// run is navigable back to the requirement without source access:
///
/// ```
/// use vibe_workspace::WorkspaceError;
///
/// let err = WorkspaceError::NestingCycle {
///     path: "members/a".to_string(),
/// };
/// assert_eq!(
///     err.to_string(),
///     "workspace nesting cycle: `members/a` is reached more than once \
///      (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting; \
///      fix: remove the members entry that re-lists an ancestor workspace)",
/// );
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting")]
pub enum WorkspaceError {
    /// No `vibe.toml` exists at or above the starting directory.
    #[error(
        "no `vibe.toml` found at or above `{}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting; \
         fix: run inside a vibevm project or create a `vibe.toml` at the \
         project root)",
        .start.display()
    )]
    NoManifest { start: PathBuf },

    /// A node's `vibe.toml` failed to read or validate. The inner error is
    /// boxed — `vibe_core::Error` is large, and an unboxed copy would bloat
    /// every `Result` in this crate (`clippy::result_large_err`).
    #[error(
        "manifest at `{}` is invalid \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest; \
         fix: repair that vibe.toml — the underlying error names the defect)",
        .path.display()
    )]
    Manifest {
        path: PathBuf,
        #[source]
        source: Box<vibe_core::Error>,
    },

    /// A `[workspace].members` entry — an explicit (non-glob) path —
    /// names a directory that does not exist or carries no `vibe.toml`.
    #[error(
        "workspace member `{pattern}` declared in `{declared_in}` does not exist \
         or carries no vibe.toml \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#workspace-section; \
         fix: create the member directory with a vibe.toml or drop the entry \
         from [workspace].members)"
    )]
    MemberNotFound {
        pattern: String,
        declared_in: String,
    },

    /// A member resolved to a directory outside the absolute root. Every
    /// member must live under the root so its `rel_path` is portable.
    #[error(
        "workspace member `{path}` lies outside the workspace root `{root}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting; \
         fix: move the member under the workspace root or drop it from \
         [workspace].members)"
    )]
    MemberOutsideRoot { path: String, root: String },

    /// A `[workspace]` transitively lists itself — the member graph is not
    /// a tree.
    #[error(
        "workspace nesting cycle: `{path}` is reached more than once \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting; \
         fix: remove the members entry that re-lists an ancestor workspace)"
    )]
    NestingCycle { path: String },

    /// A `members` glob pattern is syntactically invalid.
    #[error(
        "invalid member glob pattern `{pattern}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#workspace-section; \
         fix: correct the glob in [workspace].members)"
    )]
    BadGlob { pattern: String, reason: String },

    /// A filesystem operation failed.
    #[error(
        "I/O error on `{}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting; \
         fix: check that the path exists and is readable, then retry)",
        .path.display()
    )]
    Io { path: PathBuf, reason: String },

    /// Discovery succeeded, but the exact selected path is not the root of a
    /// workspace node. Node ownership is never inferred from containment.
    #[error(
        "selected path `{}` is not a node of workspace `{}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#workspace-section; \
         fix: select the workspace root or the exact root of a listed member)",
        .selected.display(),
        .workspace_root.display()
    )]
    SelectedPathNotNode {
        selected: PathBuf,
        workspace_root: PathBuf,
    },

    /// A transformed spec-document slot could not be produced without
    /// violating the deterministic derived-tree contract.
    #[error(
        "cannot materialise spec documents at `{}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-045#materialisation; \
         fix: repair the colliding path or unreadable source, then re-run install)",
        .path.display()
    )]
    SpecMaterialization { path: PathBuf, reason: String },

    /// A `version.var` placeholder names no entry in any enclosing
    /// `[workspace.versions]` table.
    #[error(
        "version placeholder `{var}` referenced in `{declared_in}` is defined in no \
         enclosing [workspace.versions] \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#versions; \
         fix: define `{var}` in a [workspace.versions] table of an enclosing \
         workspace)"
    )]
    UnknownVersionVar { var: String, declared_in: String },

    /// A `[workspace.versions]` entry holds an unparseable version constraint.
    #[error(
        "[workspace.versions] placeholder `{var}` has an invalid constraint \
         `{constraint}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#versions; \
         fix: give the placeholder a parseable constraint such as `0.0.1` or `^0.3`)"
    )]
    BadVersionVar { var: String, constraint: String },

    /// A `version.var` dependency entry fails `PackageRef` validation when
    /// its placeholder resolves (PROP-007 §2.6) — the `group/name` pair is
    /// not a valid package reference.
    #[error(
        "var-dep for placeholder `{var}` in `{declared_in}` is not a valid \
         package reference: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#versions; \
         fix: use a kebab-case group/name in the [requires] var-dep entry)"
    )]
    BadVarDepRef {
        var: String,
        declared_in: String,
        reason: String,
    },

    /// The generated boot INDEX TOML manifest failed to
    /// serialise. Structurally unreachable with today's fixed manifest
    /// shape; routed as an error so a future shape change degrades to a
    /// diagnosis instead of a panic.
    #[error(
        "rendering {index} failed: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#artifacts; \
         fix: the IndexManifest shape no longer serialises as TOML — restore \
         a serialisable shape)",
        index = crate::layout_paths::boot(vibe_core::layout::INDEX_MD)
    )]
    IndexRender { reason: String },

    /// Compiling the inline boot lane (PROP-035 §8) failed — an `#embed`
    /// directive in an inline contribution could not be resolved. The
    /// contribution is malformed; `reason` names the offending address.
    #[error(
        "compiling {inline} failed: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#embed)",
        inline = crate::layout_paths::boot("INLINE.md")
    )]
    InlineCompile { reason: String },

    /// A boot contribution's TYPED provenance could not be built: a
    /// coordinate component the install model still carries as a bare string
    /// does not spell the grammar its typed identity requires. Refused, never
    /// downgraded — an untyped provider would silently put the contribution
    /// out of every `applies_to.packages` dimension's reach.
    #[error(
        "boot contribution `{origin}` names {component} `{spelling}`, which is not a valid \
         coordinate component: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR; \
         fix: correct that coordinate in the manifest that declares it, then run `vibe install`)"
    )]
    UntypedBootProvenance {
        origin: String,
        component: &'static str,
        spelling: String,
        reason: String,
    },

    /// The durable extension world of this install could not be observed, or
    /// the one kernel collector refused an owner-scoped view of it. Boxed:
    /// [`ExtensionWorldError`] carries a boxed manifest error and two paths,
    /// and an unboxed copy would bloat every `Result` in this crate
    /// (`clippy::result_large_err`).
    #[error(
        "the installed extension world could not be observed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: the underlying error names the defect and its repair)"
    )]
    ExtensionWorld {
        #[source]
        source: Box<ExtensionWorldError>,
    },

    /// One lane owner's effective `compile:*` rows do not form a transform
    /// plan. The declaration is malformed or names a capability this compiler
    /// does not yet own; `source` states which.
    #[error(
        "the compile-point extensions of `{owner}` do not form a transform plan: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: correct the declaration the underlying error names, then run `vibe install`)"
    )]
    TransformPlan {
        owner: String,
        #[source]
        source: vibe_spec::TransformLoweringError,
    },

    /// Runtime lowering named no node in the supplied workspace.
    #[error(
        "owner-runtime {role} node `{rel}` is not a node of this workspace \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM; \
         fix: pass `.` or one exact workspace-relative member path)"
    )]
    UnknownRuntimeNode { rel: String, role: &'static str },

    /// A well-typed package coordinate is absent from the lowered unit map.
    #[error(
        "owner-runtime unit `{provider}` is absent from this lowered world \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION; \
         fix: request one installed package coordinate retained by the runtime epoch)"
    )]
    UnknownRuntimeUnit { provider: String },

    /// An opaque registry index failed to project through the registry that
    /// the same owner runtime retained.
    #[error(
        "owner-runtime `{owner}` cannot project its retained {family} row index \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: keep opaque row indices co-owned with their immutable originating registry)"
    )]
    OwnerRuntimeIndex { owner: String, family: &'static str },

    #[error(
        "compiler-native binding for owner `{owner}` is unavailable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: supply one binding from this exact retained owner runtime)"
    )]
    NativeCompileProvider { owner: String, reason: String },

    #[error(
        "the explicit boot resolution differs from the ordered resolution retained by the \
         owner-runtime epoch (violates \
         spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: compose with the exact resolution that was lowered into this epoch)"
    )]
    OwnerRuntimeResolutionMismatch,

    #[error(
        "compiler-native execution for owner `{owner}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: repair the compiler-native refusal named by the underlying error)"
    )]
    NativeCompile {
        owner: String,
        #[source]
        source: Box<vibe_spec::ArtifactCompileError>,
    },

    #[error(
        "compiler-native fact join for owner `{owner}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: keep one terminal fact recorder on the exact compiler binding)"
    )]
    NativeCompileFacts {
        owner: String,
        #[source]
        source: crate::extension_world::CompilerNativeFactError,
    },

    #[error(
        "pending compiler-native evidence for owner `{owner}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: join the exact owner, format and build facts from this compile)"
    )]
    NativePendingEvidence {
        owner: String,
        #[source]
        source: crate::extension_world::PendingEvidenceError,
    },

    #[error(
        "pending compiler-native finalization for owner `{owner}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: finalize the untouched provisional artifact with its exact plan and fingerprint)"
    )]
    NativePendingFinalize {
        owner: String,
        #[source]
        source: vibe_spec::CompilerPendingFinalizeError,
    },

    /// A publish operation referenced a node `rel_path` that names no
    /// node of this workspace — the selection and the loaded workspace
    /// fell out of sync.
    #[error(
        "publish references `{rel_path}`, which is not a node of this workspace \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#selective-publish; \
         fix: pass a rel_path that names the root `.` or a listed member)"
    )]
    UnknownPublishNode { rel_path: String },

    /// The dependency boot graph handed to the computed-view engine
    /// contains a cycle — a package transitively requires itself.
    #[error(
        "boot dependency cycle among: {packages} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#effective-boot; \
         fix: break the [requires] cycle among the listed packages)"
    )]
    BootDependencyCycle { packages: String },

    /// A `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` carries a malformed
    /// vibevm managed block — not exactly one well-formed `<vibevm>` …
    /// `</vibevm>` pair (PROP-012 §2.3). vibevm never guesses which block
    /// is canonical; the operator repairs the file by hand.
    #[error(
        "malformed <vibevm> block in `{}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#markers; \
         fix: keep the block you want, delete the other marker(s), then re-run — \
         zero markers or exactly one <vibevm>/</vibevm> pair)",
        .path.display()
    )]
    MalformedRedirectBlock { path: PathBuf, reason: String },

    /// A package's install hook (PROP-020) failed: no usable interpreter, a
    /// spawn error, an untrusted run, or a `pre-install` non-zero exit. The
    /// wrapped hook error already carries its own Class-F `(violates …;
    /// fix: …)` tail, so this delegates its display transparently. For a
    /// `pre-install` failure the materialised slot is rolled back before
    /// this surfaces (PROP-020 §2.5).
    #[error(transparent)]
    Hook(#[from] crate::hooks::HookError),

    #[error(
        "slot lifecycle {phase} callback for `{package}` failed: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR; \
         fix: repair the lifecycle contribution and retry the phase)"
    )]
    SlotLifecycle {
        phase: &'static str,
        package: String,
        reason: String,
    },
}
