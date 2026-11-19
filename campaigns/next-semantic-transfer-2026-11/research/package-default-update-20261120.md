# Governed ordinary whole-workspace update — candidate

Baseline accepted fresh/replay basis `c830cd4b5`; update carrier contract integrated `6d34f6db2`. First RED remains retained in `automatic-cli-tests-r1.log`: configured update refused before semantic comparison. This candidate does not treat that refusal as success or plain install as update.

Only Roots::All with authored common inputs enters the governed configuration path. Existing named/scoped guards remain; mixed Git/path/var declarations, nonlocal retained sources and in-place packages refuse. Actual update root constraints, locked-source policy and re-resolution remain unchanged, so a reachable newer version can move. No separate solver/publication engine or new user acknowledgment exists. Final visible graph coordinate membership must match the prior admitted lock on this bounded configured route; an add/remove requires explicit future reconciliation and refuses before fetch/effects. This prevents retained scoped rows from becoming unreachable leftovers while allowing version movement and independent roots.

The shared ordinary RootRequirements collector provides original consumer inputs. Existing PlanningSource owns one operation-local ConfigurationSnapshot with admitted prior lock records, projects the selected graph, verifies actual fetched manifest equality and requires ordinary local-registry copy. The same selected PackageConfiguration travels as a typed carrier through fetch, staging, ResolvedDep and planned lock replacement. Full-world preflight and apply see the same records. Before publication, actual workspace inputs are rediscovered and compared to the captured originals. Existing record admission, cancellation checkpoints, prune accumulator and rollback remain owners. Legacy pending in-place tuple rows carry None and retain their original unsupported configuration boundary.

## Public discriminators

`automatic-update-tests-r2.log` records two focused PASS functions. The changed-default function exercises three actual public cases against reachable local version1.1.0:

- Omitted previously relied-on disabled default, changed to enabled: update --all refuses with retained-configuration semantic mismatch, not broad unsupported-input preflight; old lock bytes, boot, hook output and absent optional slot remain unchanged.
- Explicit disabled selection against the same changed-default version: update succeeds, lock actually names1.1.0 and extra stays absent.
- Omitted disabled default with unchanged default in the new version: update succeeds, lock actually names1.1.0 and extra stays absent. Source-version movement alone is not semantic drift.

Independent review then found a genuine r2 defect: unchanged explicit input with a valid new definition dropping a mandatory dependency left the old unreachable lock row/slot. The final membership guard refuses that unsupported graph change rather than adding generic orphan deletion. A new explicit-input discriminator asserts old version, lock/boot/hooks and slots remain intact. An intermediate r3 compiler failure comparing PackageName with String was corrected by normalizing the coordinate key, without changing version semantics. Named-route and corrupt historical-basis refusals are separate negative cases.

The ignored marker from the first atom is removed only after these cases pass. On/off install and cold replay remain in the focused module. The architect independently reproduced the three actual outcomes; final source/gate acceptance is separate.

`automatic-update-rollback-r2.log`: five existing scoped regression cases PASS, covering measured prefix, absent prune, restored superseded slot and partial prune failure. No test oracle was weakened; tuple fixtures only add an absent configuration carrier. Full public fixture and strict lint are recorded in final gate logs/manifest before acceptance. All runtime tests use unshared filesystem/process/network namespaces with synthetic HOME/settings and exact public CLI binding.

No shown-review acceptance, stale response protocol, profile/parameter/feature-default/closure automatic reliance, configured named updates, mixed/in-place support, broad package-kind matrix or NST closure is claimed. Changed automatic meaning remains fail-closed. Remaining consumer-drift and malformed historical admission gates follow the existing owner boundaries rather than silent retry.
