# Pure environment layout and prefix policy in mutable 1.1.0

`vibe_core::environment` provides injected layout and resource values. Constructors create no directories, discover no HOME/PATH, and run no providers. Existing project/package `vibevm/` roots and install/deploy paths remain unchanged.

Construct target paths before roots. `TargetComponent::new(policy, bytes)` checks one `Vec<u8>` component; `TargetRelativePath::new(policy, components)` checks a sequence. `AbsoluteTargetPath::new(policy, anchor, components)` adds an explicit `TargetAnchor::PosixRoot`, `WindowsDrive(letter)` or `WindowsUnc { server, share }`. `appended` and `appended_component` preserve the declared policy.

`TargetPathPolicy::LinuxBytes` preserves non-UTF8 bytes, colon, backslash and device-like names even under Windows control. Empty, dot, dotdot, NUL and embedded slash components refuse. `WindowsPortableWin32Utf8` requires UTF8 and the existing portable-Win32 component subset, including reserved-name/character and trailing-dot/space refusals. Anchors must match the target policy. Neither host `PathBuf` parsing nor the source-portable path grammar defines Linux payload validity.

Select `CaseComparison` explicitly. `ByteSensitive` compares literal bytes. `AsciiInsensitive` folds only ASCII; nonASCII comparison is unresolved. `Unknown` is unresolved. `AbsoluteTargetPath::lexical_relation` reports `Equal`, `Ancestor`, `Descendant`, `Separate` or `Unresolved`. These are lexical results, without native collation or physical-alias guarantees.

`DeclaredRootBinding::new(instance, role, references, principal, path)` describes a root. `RootBindingReferences` carries supplied `binding`, `producer` and optional `physical_evidence`; `evidence_verification()` always returns `ClaimVerification::Unverified`. Opaque IDs/references are preserved as data, never encoded or joined into directory names.

Use `EnvironmentLayout::settings_anchored(SettingsLayoutInputs)` to explicitly select the settings table:

| RootRole | Selected settings-table rule |
|---|---|
| Configuration | Required injected settings path S |
| ImmutableStore | Required S/store, with separately supplied store references |
| MutableState | Required S/state, with separately supplied state references |
| Profile | Inapplicable for WorkspaceProjection; otherwise an explicit profile binding is required |
| DisposableCache | `CacheSelection::Unused` is inapplicable; `Requested(None)` is missing-required; `Requested(Some(RootSpec))` binds the explicit cache |

Missing `settings` leaves configuration/store/state `RootSlot::MissingRequired`, even when `home` is present. Home and `config_file_override` are independent optional target paths; neither supplies settings. Supplying a profile to the workspace settings table refuses. The inputs also name instance, principal, policy and mode. `RootSpec` supplies a checked path and references.

For another explicitly chosen layout, `EnvironmentLayout::explicit` takes five `ExplicitRootChoice` entries in `RootRole::ALL` order: configuration, immutable store, profile, mutable state, disposable cache. `Required(None)` remains missing-required; `Inapplicable` states that the role is unneeded. Bound entries must match role, instance, principal and target policy. Inspect `slot(role)` and `missing_required()` before any separately admitted operation.

`ResourceIdentity::new(instance, root, principal, relative_path, comparison)` requires a nonempty relative path and consistent root joins. `lexical_compare` distinguishes declared identities using instance, root role, binding reference, producer, principal, policy, comparison and component keys. Shared immutable content/root references across instances do not transfer mutable resource identities or receipts. This consistency check authenticates no physical root, producer or permission.

`protected_configuration_paths_cache_relation` checks only `config.toml`, `settings.toml`, `registry.toml` and the supplied config override. Its lexical overlap/separation/unresolved result does not protect all configuration or credentials, prove physical cache isolation, or authorize deletion.

`PrefixCompatibility::PrefixBound { exact_target_prefix }` refuses changed prefixes. `DeclaredRelocatable { producer, evidence, conditions }` needs supplied evidence and all declared condition references; `assess` reports missing evidence/conditions, mismatch, unresolved comparison or unsupported status. `DeclarativeMatch` does not verify relocation evidence or grant effects. A HOME destination cannot make an unknown or /usr-bound payload relocatable. `evidence_verification()` remains Unverified.

`ReservedEnvironmentResource` describes account/service/mount/boot names; `operation_support()` returns `UnsupportedOperation`.

Existing settings accessors retain VIBE_SETTINGS/home precedence. Pure `*_from(&Path)` helpers retain config/settings/registry filenames and registries/search-cache suffixes. Binary store/bin and deployment state/deployments retain their paths; S/state is an environment description, without migration. No native execution, complete SDK coverage or physical admission follows from these data APIs.

The [retained proof receipt](evidence/environment-layout-proof-retention.json) records eight distinct data cases, one exact repeat after a name-only correction and two distinct legacy checks. Injected platform facts are not native Windows/Linux execution proof. Product and owned packages remain mutable 1.1.0; public publication is held.

Governing contract: [PROP-076](../../../vibevm/vibespecs/common/PROP-076-environment-model.xml#ENVIRONMENT-LAYOUT).
