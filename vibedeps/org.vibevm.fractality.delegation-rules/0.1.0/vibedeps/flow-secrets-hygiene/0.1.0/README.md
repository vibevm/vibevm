# `flow:secrets-hygiene` — no secret value on any surface

A vibevm `flow` package that installs a **defensive secrets posture**
for a repository worked by a coding agent. Agents read a lot and their
sessions may be recorded or logged, which removes the safety margin a
traditional secrets policy relies on: a value spoken once persists
wherever the session persists, and one echo into chat, a diff, or a
log is a full disclosure. This package makes the handling rules
explicit, mechanical, and always-loaded so no code path and no session
puts a credential value on a surface.

The core idea is the **surface secret**: a credential whose *value*
never appears on any surface the tooling produces, while its *source*
(an env-var name, a file path) may be printed freely. Four laws follow
from it — never printed, never persisted outside one sanctioned
location, sanctioned process boundaries only, redaction tested rather
than promised — plus scope discipline for integrations and a consent
gate for install-time code.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL.md` — the full
  protocol: the surface-secret definition, the four laws, the
  agent-era additions (recorded sessions, one-echo-is-a-leak, the
  accidental-read drill), the blast-radius rationale, and the
  suspected-leak drill (rotate first, investigate second).
- `spec/flows/secrets-hygiene/scope-discipline.md` — the
  never-escalate law for integrations: explicit prefix checks,
  escalation as an error rather than a warning, and trust ordering so
  a low-trust source cannot override a trusted answer.
- `spec/flows/secrets-hygiene/third-party-code-consent.md` — the
  consent gate for install/build hooks: allow-list plus first-run
  consent, CI aborts rather than runs unseen, hooks as versioned files
  not inline strings, and secrets kept out of hook environments.
- `spec/boot/57-flow-secrets-hygiene.md` — boot snippet loaded at
  session start: the standing rules and the never-do list.

## Install

```bash
vibe install flow:secrets-hygiene
```

## Uninstall

```bash
vibe uninstall flow:secrets-hygiene
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Composition

- `flow:attribution-policy` (`55-…`) is the sibling policy package:
  both are one-place policies enforced by mechanical scans — one keeps
  authorship marks off every surface, this one keeps secret values off
  every surface.
- `flow:manual-tests`: its clean-slate rule is the test-side of Law 2
  — tests never touch real per-user state, including real credential
  files, so a test run can never read or persist a live secret.
- `flow:health-audit`: add a periodic audit line that scans for new
  output paths (logs, JSON fields, error messages) that could echo a
  secret value, catching drift as the tool grows surfaces.
- `flow:conflict-protocol`: when it is genuinely uncertain whether a
  value is secret, the conservative default governs — treat it as
  secret and ask, rather than guessing it is safe to print.

## Philosophical background

Crystallized from the origin project's token-secrecy law — a global
invariant that publish tokens, registry credentials, and provider keys
are surface secrets, audited on every code path because a single leak
is the whole organization and a single scope escalation is the whole
account. Generalized here to any product, credential, and coding
agent. The collection's spirit is the book *AI-native development*,
which ships in Russian inside `flow:redbook` at `spec/book/ru/`.

## License

UPL-1.0. See `LICENSE.md`.
