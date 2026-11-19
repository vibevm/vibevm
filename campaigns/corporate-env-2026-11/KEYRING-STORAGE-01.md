# Generic local keyring — first storage atom

2026-11-20. Candidate, not full terminal CRUD or AI connection support. Authority: PROP-000 TS-USER-KEYRING/TS-KEYRING-CONSUMER and AI-CONNECTIONS-DESIGN-01.md. The plaintext-at-rest policy is intentional: current-user filesystem protection is not encryption or protection against unrestricted same-user tools, malware, administrators or backups.

## Owner and format

`vibe-secrets` is independent of AI, UI preferences and application payloads. Its trusted caller injects the effective user settings directory; the sole selected root is `<settings>/secrets`. No HOME/cwd discovery, alternate store fallback or credential import occurs. Initial public operations are create, metadata-only list/details and expected-revision rotation. No raw-value read/export, Display, serialization or generic consumer grant exists in this atom.

Private schema 1 `index.toml` has `schema=1`, monotonic `store_revision`, and `[items.<UUID>]` tables with `current_revision` UUID, nonsecret `kind` and `label`. Both logical IDs and revision IDs are independently generated UUIDv4 using the existing OS-random owner; values/labels never determine IDs. Canonical lowercase UUID spelling and version/variant bits are validated. IDs are references, never bearer authorization. Fresh logical and revision paths are claimed exclusively; collisions refuse rather than reuse prior items. A deleted/unselected ID cannot be reassigned through path reuse.

Immutable value location: `items/<logical-id>/<revision-id>/value`. Bounded opaque values are at most 1 MiB; the index is at most 4 MiB. Proposed complete index serialization/schema/size is checked before publishing a new value. Mutation publishes a fresh private value revision first, then atomically replaces the selected index under one private store lock. An interrupted rotation before index publication preserves the old selection; unselected private revisions may remain and are not eagerly deleted. An error after index publication may mean publication completed and requires metadata reassessment, not blind replay. Old revisions remain available; secure erasure is not claimed.

List/details do not create directories, locks, timestamps or last-used files. They parse metadata only, while validating selected value presence/protection without loading value bytes. Private index bytes are read through the same held handle whose owner/mode was admitted, followed by stable-read/final-name and protection rechecks. Malformed schemas/IDs, unsafe parents/links/hardlinks and over-bound files fail without echoing data.

## Filesystem boundary

The existing `vibe-safefs` capability owner gains narrowly named private child/leaf/lock/read operations. Unix private directories require effective-user ownership and exact 0700; files/lock require effective-user ownership and exact 0600. The injected settings parent must be effective-user owned and not writable by group/other. Modes are set at creation, not by exposing value bytes then chmod. Existing insecure occupants refuse, not silently repair. Lock is directly below the protected root, not a hidden `.vibe` subtree. All writes remain held-parent no-follow operations; namespace/lock identity is rechecked.

Windows and other platforms without witnessed current-user ACL protection fail closed before writing. This is an explicit unsupported cell, not a claim that Unix mode=None enforces privacy. A reviewed Windows ACL owner remains a later requirement. No host permission/config changes are part of tests.

## Acceptance and remaining work

Synthetic tests cover stable UUID rotation, canonical parse, redacted metadata/Debug, side-effect-free reads, expected-revision conflict, two concurrent writers with one winner, interruption before index switch, exact index size boundary/oversize refusal with no new value revision, foreign neighbor retention and unsafe permission/symlink/hardlink refusal. Full safe-filesystem regression and strict affected lint remain required before source acceptance.

Remaining: reviewed Windows DACL, public CLI/UI CRUD/search/metadata editing and reference-aware deletion, admitted local consumer resolution, AI connection/default-model/bindings transaction, terminal/protected input, native authentication delivery and final distribution. No real secret is read or stored by these tests, and this atom does not complete CORP-AI-CONNECTIONS.
