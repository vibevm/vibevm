# Common configuration: upgrading clients without losing choices

Scope: accepted local NST deliveries through `cbab09865`, with current integration receipts through `cb3209e82` (2026-11-20). These are source-pinned development capabilities, not a promise that an older published binary with the same version string supports them. Check the executable you actually use. NST, all-platform support and real-agent USER-PROJECT acceptance are not complete.

## Before upgrading or changing a package

Preserve together:

- consumer `vibe.toml` files, `vibe.lock`, and any external answer document you own;
- the exact source versions/content pins and complete store payloads needed for replay;
- application installation/work/continuation records when using the separate app lifecycle.

**Do not delete a rich lock, lower `schema_version`, strip an unfamiliar field, or replace an omitted answer with its effective default to make an older client accept it.** The lock can contain submitted originals or source/member witnesses that cannot be recovered from a materialized slot. A cache sidecar or matching manifest is not a complete payload proof. Keep original evidence and use a compatible client; do not clean an unknown/live/recovery-dependent resource as an upgrade step.

Reads do not automatically migrate locks. The writer chooses the lowest epoch that actually represents the history being published; upgrading the reader does not by itself force every project to the latest epoch. A refused old client is a safe outcome, **not** semantic backward compatibility.

## Conditional project-lock epochs

Current project reader admits epochs7–18, subject to full structural/semantic validation. This is not the application index version, a wire protocol version or a software release number. Ordinary legacy-only state retains7; common explicit configuration typically needs8. Epochs9–11 belong to earlier remap/template history and are not redefined here.

| Minimum epoch | History that requires it | Preserve this meaning |
|---|---|---|
| 12 | Scalar exactly-one automatic choice basis | Actual consumer/group/member and selected-definition fingerprint; authored omission remains omitted |
| 13 | Configured lazy-pull `source_witness` | Exact package pin, declaration/member paths, byte lengths and hashes; selected member is not automatically injected into boot |
| 14 | Candidate-owned `forwarded_inputs` | Exact active parent/source feature/declaration edge/content pin and target feature; not a synthetic authored consumer |
| 15 | `consumer_choice_sets` | Lossless automatic member lists for admitted non-scalar groups; declared empty set is not an absent default |
| 16 | Operation-submitted ordinary originals | Submitted origin remains distinct from authored consumers; project/source witness is drift binding, not a signature or consent |
| 17 | Nonempty `consumer_dormant_choices` map | Inactive guarded default reliance is a reason-only group ID, not a selected member |
| 18 | Nonempty `consumer_feature_defaults` map | Canonical declared default seeds **before closure**, including a present consumer→[] empty-reliance row |

Active guarded singleton default selection still uses scalar12. An explicit no-reliance answer is not automatically promoted to17/18. Optional public bool without a default uses existing8: absent key is not false; present false/true remain booleans. In the admitted multi-consumer bool slice, absence does not vote, and conflicting present values refuse before effects.

### What old executables actually did

These are retained exact executable subjects, not compatibility claims for every older release:

| New subject | Tested prior executable SHA-256 prefix | Actual reader/writer result |
|---|---|---|
| Member-set basis15 | `12b4818f` | show/install refused the new field unchanged; historical scalar12 control round-tripped without loss |
| Submitted originals16 | `5c46aec5` | old show/install refused retained submitted history without state loss |
| Dormant17 / active guarded12 | `2acac864` | dormant field parse refusal / active guarded semantic refusal; independent unguarded reader/writer controls succeeded unchanged |
| Feature seeds18, including empty row | `858f4635` | old show/install refused unchanged; explicit false/no-reliance controls retained their prior meaning |
| Optional bool8 | `65e53d97` | old show/install refused optional records unchanged, even though no new epoch/field was necessary |

Lazy witness13 and forwarding14 also have actual pre-extension reader/writer refusal receipts. Their exact test/evidence navigation is in the accepted reports and [format ledger](../campaigns/next-semantic-transfer-2026-11/FORMAT-CHANGES.md), FMT081–082/088. Compatibility is checked against actual data and executable behavior, not inferred from epoch or deserialization alone.

## Read-only results versus installation or consent

### Ordinary configuration

`vibe show configuration-preview PACKAGE --path PROJECT --json` is source-bound inspection. The admitted first slice uses one exact local Flow, exactly-one answers and a public bool. It does not install, approve a plan, or authorize an MCP writer. Interactive preview is also read-only.

Actual external/console installation uses separate flags:

```sh
vibe install --path PROJECT --ordinary-answers ANSWERS.json --offline --assume-yes --json
vibe install --path PROJECT --ordinary-interactive --offline
```

The first requires a strict `vibe-ordinary-install-answers/1` document, built from the supported preview's exact project/source witness and original answer. Do not transplant a document to another root or edit the manifest/source and reuse its old witness. `--assume-yes` is explicit apply consent for the file route; JSON/unattended alone is not. Interactive collection and separate default-No apply confirmation are distinct. This is not selected-app `--configuration-input`, and the bounded file/console route does not imply a general MCP install tool.

`show configuration-data` separates selected effect `values` from public `parameters`; false is not absence. Source package definitions are not rewritten to store answers. Submitted originals are published only through the verified existing install transaction; generic lock write/render is deliberately not authority to manufacture them.

### Doc and Remap

Registered `read_doc` accepts the dedicated closed request with optional exact selections and returns separate `selected_values`. The bounded answered route uses the captured in-tree document source; it does not edit page text, persist choices, install a package or grant execution. Answered `lang+input` is unsupported in this slice, not silently ignored.

```sh
vibe remap preview org.example/policy --input ANSWERS.json --offline --json
```

The strict selections-only request evaluates one verified **local-owned offer**. Output `vibe-remap-preview/1` reports its local content pin, not original supplier provenance. Preview/list/explain do not accept or activate a policy; only the existing explicit accept/reject/enable/disable operation owns that decision. Unknown fields/nulls/wrong shapes refuse rather than being ignored.

## Drift, update and safe recovery

| Situation | Safe action / boundary |
|---|---|
| Older reader refuses a new field or unsupported meaning | Keep all originals and lock unchanged; select a compatible executable. Do not down-convert by deleting fields |
| Source/member witness missing or forged | Restore the retained valid lock/source/payload evidence or diagnose the exact owner. Current cache bytes cannot manufacture historical authority |
| Store payload missing/corrupt | Restore/reacquire exact pinned bytes through an admitted route. A complete supported cold replay may work without registry; source-present inspection can still require the source |
| Submitted project/root/source changed | Collect a fresh supported preview/answer document and explicitly approve a new installation; old project witness is stale |
| Selected definition changed | Preserve original input and use the supported update/reconfiguration path. Invalid/new required meaning refuses before effects; do not guess missing answers |
| Same-version live source changed but original input did not | Distinguish pinned replay from new source selection. A successful held replay does not prove a changed live definition was accepted |
| Guarded gate on↔off rebind | Only the admitted unchanged exact-root/source/version/content/definition/input scope is supported; live source bytes are checked against the old pin. Simultaneous gate+default/source changes refuse |
| Automatic member-set default renewal | The separate v2 review described below admits only a graph-preserving declared-set replacement. Scalar v1 acknowledgement does not authorize it; dependency membership/edge changes still refuse |
| Feature/guarded default renewal | These wider renewals remain unsupported by either scalar v1 or member-set v2 acknowledgement. `--assume-yes` is not a replacement review token |
| Ordinary lexical Path | `must_exist=false` remains lexical, with no target-readiness claim. The bounded install-only `must_exist=true` route below checks relative file/directory metadata at the declaring root; it is not universal Path support |
| Secret answer in ordinary authored/serialized input | Do not persist or hash it into public history. Tested diagnostics refuse/redact; this is not a general ordinary ephemeral-secret delivery pipeline |

### Install-only host Path checks

Accepted source `4c61f9906` adds one `must_exist=true` slice: a compulsory exact named-local-registry copy Flow, one standalone declaring-root consumer and one required public authored relative Path, without profiles/defaults, automatic choice, submitted input or forwarding. The base is the directory of that actual consumer's `vibe.toml` (the selected project root in this slice), **not** process cwd, registry or HOME. Nested consumers, absolute/traversal paths, symlinks/reparse points, special entries and unsupported platforms refuse; no fallback changes the answer or follows a link.

Install checks only regular-file/directory presence and type through existing no-follow metadata/directory capabilities, without reading file contents or enumerating directory contents. It retains the original lexical string at epoch8, not an absolute host realization or perpetual readiness receipt. Checks repeat before effects and on cold/Fresh install before boot writes; a removed target or rebound parent refuses while preserving existing package state. Read-only `show configuration-data` may still report the historical lexical value after disappearance: **show does not assert current host readiness**. This recheck is not same-inode authentication for regular→regular replacement or atomic protection against hostile changes after observation.

Update/reinstall and other effectful routes lacking the same install-owned gate explicitly refuse these records. `must_exist=false` behavior and separate app path realization remain unchanged. Actual old65e53d97 reader/writer refuse the new host-required records without state loss; an unchanged epoch number alone does not establish old-client support. Evidence: [ordinary host Path](../campaigns/next-semantic-transfer-2026-11/research/ordinary-host-path-first-20261120.md), FMT-102.

### Separate scalar v1 and member-set v2 review

Accepted source `c65f2f414` adds one bounded automatic-set renewal. One standalone exact local Flow root, one actual omitted consumer/group and one unguarded `zero-or-more` choice may change only that group's declared default members. Package membership and dependency edges must remain unchanged; only the root version/content pin may move. Profiles, parameters, forwarding, subskills and feature-default reliance are outside this route. Typed config-data may change; adding or removing an optional dependency still refuses before package effects, even with a token.

Run ordinary `vibe update --all --offline --assume-yes --json --path PROJECT` to discover the proposal; it refuses without semantic acknowledgement and leaves the old history/effects intact. Inspect its exact old/new versions/content pins, real consumer/group and full sorted `from_members`/`to_members`. The separate closed `vibe-package-member-set-review/2` proposal supplies a `package-member-set-review/2:sha256:…` identifier. Explicitly pass that current identifier with `--package-configuration-review` to the same update operation only after reviewing it. Scalar `package-default-review/1:sha256:…` and member-set v2 identifiers are not interchangeable; `--assume-yes` alone never approves the set change.

The response rechecks actual selected sources and full payload bytes. Changed payload/definition/graph, prior lock, forged/cross-project or already-consumed tokens refuse; obtain and inspect a fresh proposal rather than reusing authority. Successful renewal preserves authored omission and records the accepted new basis. Unchanged registry-independent replay with the complete retained store needs no new review. A fully explicit answer equal to the former defaults stays explicit and requires no automatic-default renewal. This does not add general graph retirement or wider renewal support. Exact evidence: [graph-preserving member-set review](../campaigns/next-semantic-transfer-2026-11/research/multi-set-review-first-20261120.md), FMT-101.

A refusal before package effects can still create ordinary cache/coordination evidence. An injected submitted-lock post-publication failure has a specific owned-rollback receipt; it does not prove every OS crash or rollback of arbitrary package-authored external effects.

## Separate application and corporate boundaries

Selected application wizard/install/modify/update/rollback/repair retains its existing owner and original/dormant semantics; ordinary project history must not be substituted for app installation history. Current application journey/wizard/interrupt reruns are separate scenarios, not a combined universal cancellation proof.

Native installer `--configure-ai` is an explicit metadata-only post-bootstrap adapter. No flag keeps old bootstrap behavior; nonterminal input is `input_required`. Successful child exit is `completed_unconfirmed`, not evidence that a user applied metadata. Cancel/decline/optional failure do not undo completed Vibe installation. This is not full six-agent onboarding, configure-only recovery, credential delivery or provider permission.

## Evidence and implementation reference

- [Current obligation register](../campaigns/next-semantic-transfer-2026-11/research/nst-open-obligations-20261120.md): accepted boundaries and mandatory open work.
- [Format changes](../campaigns/next-semantic-transfer-2026-11/FORMAT-CHANGES.md): FMT076–100, including staged reader-versus-writer delivery.
- [Guarded defaults](../campaigns/next-semantic-transfer-2026-11/research/guarded-automatic-default-first-20261120.md), [feature defaults](../campaigns/next-semantic-transfer-2026-11/research/ordinary-feature-defaults-first-20261120.md), [optional bool](../campaigns/next-semantic-transfer-2026-11/research/optional-public-bool-first-20261120.md).
- Canonical [reader](../crates/vibe-core/src/manifest/lockfile/io.rs), [minimum writer/validation](../crates/vibe-core/src/manifest/lockfile/configuration.rs), and [original/evaluation owner](../crates/vibe-core/src/manifest/configuration/package.rs).

The old schema5 reference is historical. Its unconditional delete/regenerate advice is not a safe migration for rich configuration or submitted originals.
