# Pi 1.6.0 native settings reader qualification

## Bounded result

The exact official Pi 1.6.0 Linux x64 standalone executable reads a literal API key from native `models.json`, without launcher credential environment injection. This is native-reader qualification, **not** implementation or acceptance of Vibe's native-settings apply operation.

Pinned archive SHA-256: `ef77dbe4379b3bdeb425a1e1b862fa3c915c255000dfe4965c98eee96a5765d1`. Executable identity is checked against the previously retained fresh native archive inventory. Tagged upstream `auth-storage.ts`, `resolve-config-value.ts`, `model-registry.ts`, `config.ts` and generated models configuration reference were acquired as data, without importing them on the host.

## Literal semantics

Pi's native configuration resolver interprets leading `!` as command execution and `$NAME`/`${NAME}` as environment interpolation. Its supported literal escapes are `$$` for `$` and `$!` for `!`. A native writer must encode a literal key character by character, rather than placing arbitrary key bytes directly into an expression-bearing field. JSON string escaping alone does not meet this requirement.

The fixture uses an intentionally nonfunctional, random synthetic marker containing these metacharacters. It writes the native escaped value in `models.json`, then directly launches the pinned native executable. A synthetic local OpenAI-compatible SSE receiver checks equality with the original marker and records only a boolean. No real provider or credentials are involved.

## Executed evidence

Private evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`.

- `qualify-pi160-native-literal-r1.py`, `pi160-native-literal-r1/`: actual native runtime exit 0; one intended local request; native reader resolved the original literal; no key in process environment and no marker in captured native output.
- `qualify-pi160-native-literal-r2.py`, `pi160-native-literal-r2/`: same actual positive with an adjacent native provider, foreign top-level field and an unrelated synthetic OAuth record in `auth.json`; SHA-256 checks establish that both native files remained unchanged after direct launch. Exit 0.

Both executions use an enforced new filesystem/PID/network namespace, synthetic HOME, selected read-only archive/script inputs and isolated loopback. No ordinary user settings, auth stores or private corporate kit are mounted. Marker values and their digests are not printed.

## Remaining

The r2 preservation witness concerns **native launch**, not a Vibe merge/write. The production adapter still needs the shared internal interface, protected confirmed-operation secret sink, native-directory protection, snapshot/revision rechecks, unknown-field preserving merge, atomic publication, cancellation/no-op/stale/unsafe-path negatives and independent acceptance. Windows requires actual native ACL/reparse admission, not Unix chmod. No broker, service, export command, plugin, fork or launcher credential environment is introduced by this qualification.
