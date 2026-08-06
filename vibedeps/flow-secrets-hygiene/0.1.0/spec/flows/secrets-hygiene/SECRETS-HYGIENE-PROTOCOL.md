# Secrets Hygiene Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines *what* a surface secret
is, the *four laws* that govern one, *how* the agent era changes the
threat model, *why* the blast radius makes these rules global rather
than local, and the *drill* to run when a secret may have leaked. @status:impl/done

@fact:sibling-document-pointers
Scope discipline for integrations has its own document,
[`scope-discipline.md`](scope-discipline.md); consent for install-time
code has [`third-party-code-consent.md`](third-party-code-consent.md). @status:impl/done

## The surface secret {#surface-secret}

@fact:A-SURFACE-SECRET-IS-A-VALUE-THAT-MUST-NEVER-APPEAR A **surface secret** is a credential value that must never appear on
any surface the tooling or the working session produces. @status:impl/done

@fact:WHAT-COUNTS-AS-A-SURFACE-SECRET Publish and
deploy tokens, registry-API tokens, provider API keys, database
passwords, signing keys, SSH passphrases — all surface secrets. @status:impl/done

@fact:value-versus-source-lead The distinction that makes the concept usable is **value versus
source**: @status:impl/done

- @fact:THE-SOURCE-IS-SAFE-TO-PRINT The **source** of a secret — an environment-variable name, a file
  path, the words "passed explicitly on the command line" — is safe
  to print, log, and discuss. It tells a reader *where* the secret
  comes from without disclosing it. @status:impl/done
- @fact:THE-VALUE-IS-THE-SECRET The **value** — the token string itself — is the secret. It never
  appears on any surface, full stop. @status:impl/done

@fact:everything-below-is-that-discipline Everything below is the discipline that keeps the value off every
surface while letting the source be as visible as it needs to be. @status:impl/done

## The four laws {#laws}

| Law | The rule | The failure it prevents |
|-----|----------|-------------------------|
| @fact:ROW-LAW-NEVER-PRINTED Never printed @status:spec/done | No value to stdout, stderr, logs, JSON/event streams, error messages, panic traces, telemetry, or lockfiles @status:spec/done | A value scraped from a log or a captured error @status:spec/done |
| @fact:ROW-LAW-NEVER-PERSISTED Never persisted @status:spec/done | Only one sanctioned at-rest location: a per-user, permission-protected file (or an env var for CI) @status:spec/done | A value committed, cached, or written into the project tree @status:spec/done |
| @fact:ROW-LAW-SANCTIONED-BOUNDARIES Sanctioned boundaries @status:spec/done | A value crosses a process boundary only by an audited path @status:spec/done | A value handed to a channel that records or forwards it @status:spec/done |
| @fact:ROW-LAW-REDACTION-IS-TESTED Redaction is tested @status:spec/done | A wrapper redacts on display; a unit test proves it @status:spec/done | A redaction that was assumed but never verified @status:spec/done |

### Law 1 — never printed {#law-printed}

@fact:the-value-goes-to-no-surface-the-tool-emits-lead The value goes to no surface the tool emits: @status:impl/done

- @fact:NOT-PRINTED-STDOUT not stdout, @status:impl/done
- @fact:NOT-PRINTED-STDERR not stderr, @status:impl/done
- @fact:NOT-PRINTED-THE-LOG
  not the log, @status:impl/done
- @fact:NOT-PRINTED-A-JSON-OR-EVENT-STREAM not a `--json` or event stream, @status:impl/done
- @fact:NOT-PRINTED-AN-ERROR-MESSAGE not an error message, @status:impl/done
- @fact:NOT-PRINTED-A-PANIC-OR-STACK-TRACE
  not a panic or stack trace, @status:impl/done
- @fact:NOT-PRINTED-TELEMETRY not telemetry, @status:impl/done
- @fact:NOT-PRINTED-THE-LOCKFILE not the lockfile. @status:impl/done

@fact:WHEN-THE-TOOL-MENTIONS-A-CREDENTIAL-IT-NAMES-THE-SOURCE Where
the tool must *mention* a credential — "using the token from
`$DEPLOY_TOKEN`", "reading `<config-dir>/host.token`" — it names the
**source**. @status:impl/done

@fact:THE-VALUE-IS-NEVER-INTERPOLATED The value is never interpolated into a message, an
error, or a structured record. @status:impl/done

### Law 2 — never persisted outside one place {#law-persisted}

@fact:EXACTLY-ONE-SANCTIONED-AT-REST-LOCATION There is exactly one sanctioned at-rest location for a secret: a
per-user, permission-protected file in the tool's own config
directory, readable only by the owner. @status:impl/done

@fact:ci-substitutes-a-secret-environment-variable (CI substitutes a secret
environment variable, injected by the CI platform's secret store.) @status:impl/done

@fact:THE-VALUE-IS-NEVER-COMMITTED-CACHED-OR-DROPPED-IN-TREE
The value is never committed to the repository, never written into
the lockfile, never embedded in a cache file, never dropped into the
project's working tree. @status:impl/done

@fact:A-CREDENTIALED-URL-IS-BUILT-IN-MEMORY-AND-DISCARDED When the tool needs a credentialed URL, it
reads the secret from the environment or the sanctioned file, builds
the URL **in memory**, hands it to the child process, and discards
it — the value never touches disk by the tool's hand, and the
lockfile records only the canonical, credential-free URL. @status:impl/done

### Law 3 — sanctioned process boundaries only {#law-boundaries}

@fact:audited-paths-lead A secret may leave the process only by a small set of audited paths: @status:impl/done

- @fact:PATH-TLS-AUTHORIZATION-HEADER A **TLS `Authorization: Bearer …` header** to the host API. @status:impl/done
- @fact:PATH-SINGLE-CHILD-PROCESS-INVOCATION A **single child-process invocation** with the credential embedded
  in a URL (`https://x-access-token:<TOKEN>@host/…`), relying on the
  child tool's own redaction of URL passwords in its output. (Modern
  version-control clients redact URL passwords in their own logs.) @status:impl/done

@fact:NO-OTHER-PATH-IS-ALLOWED No other path is allowed. @status:impl/done

@fact:NEVER-IN-A-THIRD-PARTY-SCRIPTS-ENVIRONMENT In particular the value is **never placed
in the environment of a spawned third-party script** — install and
build hooks run unseen third-party code, and a secret in their
environment is a secret handed to code no one reviewed
([`third-party-code-consent.md`](third-party-code-consent.md)). @status:impl/done

### Law 4 — redaction is tested, not promised {#law-tested}

@fact:THE-WRAPPER-REDACTS-ON-DISPLAY-AND-DEBUG The in-process wrapper that carries a secret redacts the value on its
display and debug representations, so an accidental `print(token)`
emits `***` rather than the value. @status:impl/done

@fact:THE-REDACTION-IS-BACKED-BY-A-UNIT-TEST That redaction is **backed by a
unit test** that constructs the wrapper around a known value and
asserts the value does not appear in either representation. @status:impl/done

@fact:A-PROMISE-IS-NOT-A-PASSING-TEST A comment
that says "this is redacted" is a promise; a passing test is a fact. @status:impl/done

@fact:EVERY-NEW-CODE-PATH-IS-REVIEWED-AGAINST-THE-FOUR-LAWS Every new code path that touches a secret is reviewed against these
four laws before it merges. @status:impl/done

## The agent era changes the threat model {#agent-era}

@fact:a-traditional-policy-assumes-the-value-is-seen-once A traditional secrets policy assumes the operator sees the value once
and moves on. @status:spec/done

@fact:three-ways-each-tightening-the-rule An agent-driven repository breaks that assumption in
three ways, and each one tightens the rule. @status:spec/done

- @fact:AGENT-ERA-SESSIONS-MAY-BE-RECORDED **Sessions may be recorded or logged.** Screen capture, transcript
  logging, shared session archives — the session is not ephemeral.
  A value spoken once persists wherever the session persists. @status:spec/done
- @fact:AGENT-ERA-ONE-ECHO-IS-A-LEAK **One echo is a leak.** There is no "just this once" for a secret.
  A single reflection of the value into chat, a diff, or output is a
  disclosure into a medium that may be recorded, replayed, or shared.
  The cost of one echo equals the cost of full disclosure. @status:spec/done
- @fact:AGENT-ERA-NEVER-LOAD-A-SECRET-INTO-THE-CONVERSATION **Never load a secret into the conversation.** Secret files are
  edited in an editor directly. They are **never** `cat`'d, never
  read with a file-reading tool, never `echo`'d — because doing so
  pulls the value into the conversation context, which is exactly the
  recorded, replayable medium the policy keeps secrets out of. @status:impl/done

### The accidental-read drill {#accidental-read}

@fact:accidental-read-drill-lead If a secret value lands in context despite the rule — a mis-aimed
read, a tool that dumped a file — the response is mechanical: @status:impl/done

1. @fact:ACCIDENTAL-STOP **Stop.** Do not continue the action that surfaced it. @status:impl/done
2. @fact:ACCIDENTAL-DO-NOT-PROPAGATE **Do not propagate.** Do not quote the value back, do not echo it
   into a commit message, do not include it in a diff, do not
   summarize "the token is …". Every one of those is a second
   surface. @status:impl/done
3. @fact:ACCIDENTAL-ROTATE **Rotate.** Tell the human the value is compromised and must be
   rotated. Once a value may have been captured, it is dead — see the
   leak drill below. @status:impl/done

## Blast radius — why the rules are global {#blast-radius}

@fact:global-invariants-lead These rules are global invariants, not module-local conventions,
because the failure modes are catastrophic in a way local discipline
cannot bound: @status:impl/done

- @fact:BLAST-A-LEAKED-TOKEN-IS-THE-ORGANIZATION A leaked **publish or deploy token** is the whole organization:
  cross-repository writes, branch deletions, CI-secret reads. @status:spec/done
- @fact:BLAST-AN-ESCALATED-INTEGRATION-IS-THE-ACCOUNT An **escalated integration** is the whole host account: every
  resource the credential can reach, not just the intended one. @status:spec/done

@fact:THE-ONLY-SAFE-POSTURE-IS-GLOBAL-RULES-AND-AUDIT When one leak costs the org and one escalation costs the account, the
only safe posture is to make the rules apply everywhere and to audit
**every** code path that touches a secret or acts under one — not to
trust that each module reinvented the discipline correctly. @status:impl/done

@fact:scope-discipline-follows-the-same-logic Scope
discipline follows the same logic in
[`scope-discipline.md`](scope-discipline.md). @status:impl/done

## Suspected-leak drill — rotate first {#leak-drill}

@fact:leak-drill-lead When a secret **may** have been exposed — printed, committed, read
into a recorded session, pasted anywhere — the order is fixed: @status:impl/done

1. @fact:LEAK-ROTATE-FIRST **Rotate first.** Revoke and reissue the credential at its issuer
   immediately, before investigating anything. The leaked value is
   dead the moment it *may* have been seen; a revoked credential
   cannot be abused no matter who captured it. @status:impl/done
2. @fact:LEAK-INVESTIGATE-SECOND **Investigate second.** Only after the live credential is dead,
   trace how it leaked and close the path — the log sink, the code
   line, the tool call that surfaced it. @status:impl/done
3. @fact:LEAK-PURGE-BUT-ASSUME-CAPTURE **Purge where feasible, but assume capture.** Scrub the value from
   logs and history where you can, but treat the value as already
   captured. Purging reduces exposure; rotation is what actually ends
   it. Never let "we can just delete the log" substitute for
   rotation. @status:impl/done

@fact:investigating-first-is-the-classic-mistake Investigating first is the classic mistake: every minute spent
diagnosing while the live credential sits exposed is a minute an
attacker can spend using it. @status:spec/done

@fact:KILL-IT-THEN-DIAGNOSE Kill it, then diagnose. @status:impl/done

## Re-derive for your project {#re-derive}

@fact:COPY-THE-PROMPT-TASK-NOT-THE-PROMPT-IMPLEMENTATION Copy the prompt-task, not the prompt-implementation. @status:impl/done

@fact:re-derive-lead Paste this to
your agent in a fresh session: @status:impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-secrets-hygiene/<version>/spec/flows/secrets-hygiene/`, check `vibe.lock`) end to end. Then map it onto THIS
project: (1) enumerate every credential the tooling handles — tokens,
keys, passwords — and its source (env var / file / flag); (2) name
the ONE sanctioned at-rest location for each and confirm nothing else
persists it; (3) list every surface the tooling emits (stdout, logs,
JSON, errors, panics, telemetry, lockfiles) and confirm no value can
reach any of them; (4) identify each process boundary a secret
crosses and confirm it is a sanctioned path; (5) confirm a test
proves redaction. Report gaps as findings with file:line. Draft the
project's secrets-hygiene boot section: the four laws, the agent-era
rules, and the leak drill. Show me the draft; do not apply until I
approve.
```

## Summary {#summary}

- @fact:SUM-VALUE-NEVER-APPEARS-SOURCE-MAY A surface secret's **value** never appears on any surface; its
  **source** may be printed freely. @status:impl/done
- @fact:SUM-THE-FOUR-LAWS Four laws: never printed, never persisted outside the one
  sanctioned location, sanctioned process boundaries only, redaction
  tested rather than promised. @status:impl/done
- @fact:SUM-THE-AGENT-ERA-TIGHTENS-THE-RULE The agent era tightens the rule: sessions may be recorded, one echo
  is a leak, never read a secret into the conversation, and the
  accidental-read drill is stop / do-not-propagate / rotate. @status:impl/done
- @fact:SUM-BLAST-RADIUS-MAKES-THE-RULES-GLOBAL Blast radius makes the rules global — one token leak is the org,
  one escalation is the account. @status:spec/done
- @fact:SUM-ROTATE-FIRST-INVESTIGATE-SECOND On any suspected leak: rotate first, investigate second. The value
  is dead the moment it may have been seen. @status:impl/done
