# A2 source proposal, not implementation

Seven reqr1 anchors in the companion fragment; all spec/plan. Settings-anchored defaults are an explicitly selected pure table, never a host fallback for an offline target. Store/state choices reuse known spellings; profile/cache choices deliberately require explicit binding rather than inventing an old profile directory or treating settings as a disposable cache ancestor. Registry/search caches remain independent. Case/collation remains explicit lexical model; unknown native comparison/admission is unsupported/unmeasured, not inferred from OS. No opaque ID is joined into paths.

Task perimeter: P076 source read/write/source-first step is now resolved by root amendment a517bc4bd180a6aa79f08380b4665aad082d31d8dc7a726c78a51a42123c3110; P052/layout/settings reads and core/layout write already bound. Before execution request settings.rs write only for pure injected-root path helpers sharing existing suffix ownership; legacy ambient entrypoints retain behavior. Add exact declarant_path.rs read to inspect Windows-specific helpers without invoking generic source-path guards. Any collation reuse/dependency needs separate explicit owner binding; no new algorithm assumed.

EnvironmentLayout/ResourceIdentity/PrefixCompatibility leaves and environment/mod.rs export fit allowed environment/. Core/lib.rs already exports environment. No required consumer write: store.rs/model.rs remain read-only; aliasing old constants later needs separate amendment. No Cargo/dependency change justified now.

No repository edits, executable types, tests, Git or effects. Writer stopped for source/default-policy review.
