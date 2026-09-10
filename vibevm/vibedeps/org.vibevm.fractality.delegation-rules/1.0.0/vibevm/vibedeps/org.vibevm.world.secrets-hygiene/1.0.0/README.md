# `flow:secrets-hygiene` — no secret value on any surface {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-A-DEFENSIVE-SECRETS-POSTURE A vibevm `flow` package that installs a **defensive secrets posture**
for a repository worked by a coding agent. @status:impl/done

@fact:agent-sessions-remove-the-traditional-safety-margin Agents read a lot and their
sessions may be recorded or logged, which removes the safety margin a
traditional secrets policy relies on: a value spoken once persists
wherever the session persists, and one echo into chat, a diff, or a
log is a full disclosure. @status:spec/done

@fact:PACKAGE-MAKES-THE-RULES-EXPLICIT-MECHANICAL-AND-ALWAYS-LOADED This package makes the handling rules
explicit, mechanical, and always-loaded so no code path and no session
puts a credential value on a surface. @status:impl/done

@fact:THE-CORE-IDEA-IS-THE-SURFACE-SECRET The core idea is the **surface secret**: a credential whose *value*
never appears on any surface the tooling produces, while its *source*
(an env-var name, a file path) may be printed freely. @status:impl/done

@fact:FOUR-LAWS-PLUS-SCOPE-DISCIPLINE-AND-A-CONSENT-GATE Four laws follow
from it — never printed, never persisted outside one sanctioned
location, sanctioned process boundaries only, redaction tested rather
than promised — plus scope discipline for integrations and a consent
gate for install-time code. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-FULL-PROTOCOL `spec/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL.xml` — the full
  protocol: the surface-secret definition, the four laws, the
  agent-era additions (recorded sessions, one-echo-is-a-leak, the
  accidental-read drill), the blast-radius rationale, and the
  suspected-leak drill (rotate first, investigate second). @status:impl/done
- @fact:CONTENT-THE-SCOPE-DISCIPLINE `spec/flows/secrets-hygiene/scope-discipline.xml` — the
  never-escalate law for integrations: explicit prefix checks,
  escalation as an error rather than a warning, and trust ordering so
  a low-trust source cannot override a trusted answer. @status:impl/done
- @fact:CONTENT-THE-THIRD-PARTY-CODE-CONSENT `spec/flows/secrets-hygiene/third-party-code-consent.xml` — the
  consent gate for install/build hooks: allow-list plus first-run
  consent, CI aborts rather than runs unseen, hooks as versioned files
  not inline strings, and secrets kept out of hook environments. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/57-flow-secrets-hygiene.xml` — boot snippet loaded at
  session start: the standing rules and the never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:secrets-hygiene
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:secrets-hygiene
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-ATTRIBUTION-POLICY `flow:git-attribution-policy` (`55-…`) is the sibling policy package:
  both are one-place policies, mechanised where mechanisation reaches
  and reviewed everywhere else — the sibling scans two of its eight
  surfaces before push, this one backs Law 4's redaction with a unit
  test. One keeps authorship marks off every surface, this one keeps
  secret values off every surface. @status:impl/done
- @fact:COMPOSES-MANUAL-TESTS `flow:manual-tests`: its clean-slate rule is the test-side of Law 2
  — tests never touch real per-user state, including real credential
  files, so a test run can never read or persist a live secret. @status:impl/done
- @fact:COMPOSES-HEALTH-AUDIT `flow:health-audit`: add a periodic audit line that scans for new
  output paths (logs, JSON fields, error messages) that could echo a
  secret value, catching drift as the tool grows surfaces. @status:impl/done
- @fact:COMPOSES-CONFLICT-PROTOCOL `flow:conflict-protocol`: when it is genuinely uncertain whether a
  value is secret, the conservative default governs — treat it as
  secret and ask, rather than guessing it is safe to print. @status:impl/done

## Philosophical background {#background}

@fact:crystallized-from-the-origin-projects-token-secrecy-law Crystallized from the origin project's token-secrecy law — a global
invariant that publish tokens, registry credentials, and provider keys
are surface secrets, audited on every code path because a single leak
is the whole organization and a single scope escalation is the whole
account. @status:spec/done

@fact:generalized-here-to-any-product-credential-and-agent Generalized here to any product, credential, and coding
agent. @status:spec/done

@fact:collections-spirit-is-the-redbook The collection's spirit is the book *AI-native development*,
which ships in Russian inside `flow:redbook` at `spec/book/ru/`. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See `LICENSE.md`. @status:impl/done

