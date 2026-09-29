# Pure environment roles in mutable 1.1.0

`vibe_core::environment` supplies the neutral value model for the next
environment programme. It adds no environment CLI command or new package kind.
Existing install/deploy consumers keep their current host interpretation.

| Value | Supplied facts and owning producer |
|---|---|
| `EnvironmentDefinition` | Authored package roots, features, choices, target constraints, runtime bindings and tool/service exports |
| `EnvironmentLock` | Resolver-selected exact packages, content/capture/configuration references, dependency roles and external closures |
| `ExecutionContext` | Admitting caller's executor platform, tool/provider/principal and descriptive capabilities |
| `TargetBinding` | Storage caller's instance, definition/lock/generation references, target mode/platform, roots, prefix and principal |
| `EnvironmentGeneration` | Producer's lock/recipe, payload metadata, complete selected closure, adapters, services and activation references |

The five ID types include a producer label and opaque reference. Their checked
constructors preserve supplied text and reject blank references. Equality and
cross-reference validation establish consistency of these claims; they do not
authenticate a producer, content, physical root or permission. Root descriptions
and capability names cannot authorize filesystem or provider effects.

Construct a definition first, then pass it to `EnvironmentLock::new` with the
selected closure. Each selected package uses the existing `PackageRef` grammar,
a fully qualified exact version, its selected `semver::Version` and existing
`ContentHash`. A desired root may have unspecified kind/group; the qualified
selection must satisfy the fields that were specified. This checks supplied
selections without running a resolver or inventing another hash recipe.

Pass the resulting lock to `EnvironmentGeneration::new` and to
`TargetBinding::new`. Constructors reject inconsistent definition, lock,
generation, package-root and closure references. Tool names are unique within
tool exports, and service names within service exports; the two collections
may use the same name. Exports must reference declared roots/runtime bindings.
Immutable generation descriptions exclude mutable application data and secrets.

Keep the machine roles separate with `ToolchainPlatforms::new`: build executor,
produced-tool runtime and optional compiler-output platform. `None` means the
compiler-output role is inapplicable; `Some` with unknown facts means unresolved.
A Windows executor may describe a Linux payload and a different compiler output.
`native_extension_platform` reads the executor facts;
`payload_launcher_family` describes the target family. Neither helper selects,
admits or runs an executable.

`PlatformFacts` distinguishes `Fact::Known`, `Unknown` and `NotApplicable`.
`PlatformConstraints` distinguishes `Requirement::Any`, `Exact` and
`NotApplicable`. `compare` reports missing and unsatisfied fields separately.
OS equality does not prove architecture, ABI or libc compatibility.

For existing local source materialization, supply an absolute local root,
execution ID and instance ID to `LocalWorkspaceMapping::new`, then call
`workspace_source_projection_compatibility`. Wrong references/root or required
platform mismatches refuse. Unknown ABI/libc is allowed when unrequired, so
source-only installation does not acquire a new native-ABI gate. The result
`SupportedLocalWorkspaceSourceProjection` grants no write or native-payload
admission. `legacy_host_applicability` preserves the old executor-OS guard;
it does not retarget that guard using new payload facts.

User-profile, offline Linux rootfs and live Linux-system bindings can be
described, but this foundation returns `UnsupportedTargetMode` for their
operations. Target layout, physical alias/authority checks, payload realization,
HOME/rootfs/OCI delivery and service execution belong to subsequent campaign
tasks. Pure constructors perform no HOME/PATH lookup, filesystem probing,
Docker or provider execution.

The selected GNU/Linux proof comprises eight distinct cases and one exact
repeat after the export-namespace correction. Injected Windows/Linux facts
prove role interpretation, not native deployment on either platform. Complete
SDK/model/documentation coverage and final release proof remain separate.
Product and owned packages remain mutable 1.1.0; public publication is held.

The governing contract is
[`PROP-076`](../../../vibevm/vibespecs/common/PROP-076-environment-model.xml).
