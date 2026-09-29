# M14A2 bounded code review

No concrete blocker found in the named A2 implementation and adapter diffs. This is a bounded source verdict, not task acceptance or native execution proof.

TargetComponent retains arbitrary Linux bytes, including non-UTF8, colon, backslash and device-like spellings, while refusing empty/dot/dotdot/NUL/slash. WindowsPortableWin32Utf8 delegates the named Windows component owner, validates drive/UNC anchors and does not impose its subset on Linux. ByteSensitive retains arbitrary bytes; AsciiInsensitive only compares ASCII; Unknown and unsupported nonASCII comparison return Unresolved. Stored original bytes are not replaced by comparison keys. Target paths do not use executor std::path parsing.

Settings defaults keep configuration/store/state required even when S is absent and home is supplied. Profile/cache applicability and explicit-layout slot choices preserve missing-required versus inapplicable; bound roots must match role, instance, principal and target policy. Opaque root/producer/instance references are never derived into directory components. Optional physical evidence always reports Unverified. ResourceIdentity additionally checks root joins and retains instance, role, binding reference, producer, principal, policy and comparison. Shared immutable references/path/content do not merge resource identities across instances; equal mutable spellings likewise do not transfer identities.

The cache query explicitly names its limited subject: config.toml, settings.toml, registry.toml and the optional supplied override. It returns lexical overlap/separation/unresolved, not whole-configuration protection, physical disjointness or cleanup authority. Prefix checks preserve exact declared prefix comparison, missing evidence/conditions, unsupported and unresolved results. A DeclarativeMatch remains a supplied claim; evidence_verification is always Unverified. Account/service/mount/boot operations remain explicitly UnsupportedOperation.

The four adapter diffs against aa7a8cdede6cabe1f835976f79a8a120b8cd5c7c introduce shared suffix constants, pure injected-root helpers and constant aliases only. Legacy settings precedence and environment lookup bodies are unchanged; project vibevm roots, store/bin and state/deployments paths remain unchanged. No provider, placement or cleanup algorithm is replaced.

Eight meaningful retained cases passed: exit 0, 8 passed, 541 filtered. The tests cover required missing roots, explicit roots/home/override, raw bytes, traversal/Windows subset, distinct resource identities, limited lexical comparison, declarative prefixes and cache/reserved boundaries. They are data tests, not physical storage/native execution tests. No new runs were performed in this review.

Nine of ten final source SHA256 values match the original family receipt exactly. layout_tests.rs differs only by the function-name capital S to lowercase s correction: reversing that exact byte substitution reproduces the original receipt SHA. The original raw log hash also matches. Root is separately binding the renamed exact case and legacy checks; their uninspected results are not claimed here. Source metadata, consumer adoption and any future native/collation/physical evidence remain separate acceptance obligations.

Final reviewed SHA256:

- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/target_path.rs: aa455d0c8463ffb8c5b7de0710617f2f783928dbc2fe78bc4351796d51d79847
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/layout.rs: 40659ac92a30fa639bc82d2c8850c370f77732d035086541e87fec2f4180a811
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/resource.rs: 1bbc258583cce1c558107736c38f55a690ebc3d3b26a47c5628ab54855f61723
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/prefix.rs: 3b48d2bc32758c1eba21fe4faf3df050a36a9d79f7f5f850ae0055fd2f465131
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/layout_tests.rs: 9aed0c7596e931f653ff6be7943e8eeef9b8b579973ef47165b3dd51c26255e7
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/environment/mod.rs: cc9633f5a1ab9062ad1ecc20da1631f2f9858e1a921ceeaea5bd32b236c9c11b
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/layout.rs: 6e262590eb1e7aa8cf1773aa329e59536f93bad69ebb9aa2ef2adfd4d82e9151
- /home/olegchir/git/v/vibevm-next/crates/vibe-core/src/settings.rs: 1d83d15346ea1ac2ce4826acc057094880a1b8fd735d90b85046b1d339d56d6c
- /home/olegchir/git/v/vibevm-next/crates/vibe-lifecycle/src/mechanism/vibebin/store.rs: 6df696d32fc1f2a0079887b99a68d80c50eb2f800d2011661cf31689dd35aa98
- /home/olegchir/git/v/vibevm-next/crates/vibe-lifecycle/src/mechanism/deploy/model.rs: fa2616b5bb276643af2800d647406c62fc6248bd8f7b8a2bdaaecd355d3dda74
