# Secrets Hygiene Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *what* a surface secret
is, the *four laws* that govern one, *how* the agent era changes the
threat model, *why* the blast radius makes these rules global rather
than local, and the *drill* to run when a secret may have leaked. @impl/done

##sibling-document-pointers
Scope discipline for integrations has its own document,
[`scope-discipline.md`](scope-discipline.md); consent for install-time
code has [`third-party-code-consent.md`](third-party-code-consent.md). @impl/done

## The surface secret {#surface-secret}

##A-SURFACE-SECRET-IS-A-VALUE-THAT-MUST-NEVER-APPEAR A **surface secret** is a credential value that must never appear on
any surface the tooling or the working session produces. @impl/done

##WHAT-COUNTS-AS-A-SURFACE-SECRET Publish and
deploy tokens, registry-API tokens, provider API keys, database
passwords, signing keys, SSH passphrases — all surface secrets. @impl/done

##value-versus-source-lead The distinction that makes the concept usable is **value versus
source**: @impl/done

- ##THE-SOURCE-IS-SAFE-TO-PRINT The **source** of a secret — an environment-variable name, a file
  path, the words "passed explicitly on the command line" — is safe
  to print, log, and discuss. It tells a reader *where* the secret
  comes from without disclosing it. @impl/done
- ##THE-VALUE-IS-THE-SECRET The **value** — the token string itself — is the secret. It never
  appears on any surface, full stop. @impl/done

##everything-below-is-that-discipline Everything below is the discipline that keeps the value off every
surface while letting the source be as visible as it needs to be. @impl/done

## The four laws {#laws}

| Law | The rule | The failure it prevents |
|-----|----------|-------------------------|
| ##ROW-LAW-NEVER-PRINTED Never printed @spec/done | No value to stdout, stderr, logs, JSON/event streams, error messages, panic traces, telemetry, or lockfiles @spec/done | A value scraped from a log or a captured error @spec/done |
| ##ROW-LAW-NEVER-PERSISTED Never persisted @spec/done | Only one sanctioned at-rest location: a per-user, permission-protected file (or an env var for CI) @spec/done | A value committed, cached, or written into the project tree @spec/done |
| ##ROW-LAW-SANCTIONED-BOUNDARIES Sanctioned boundaries @spec/done | A value crosses a process boundary only by an audited path @spec/done | A value handed to a channel that records or forwards it @spec/done |
| ##ROW-LAW-REDACTION-IS-TESTED Redaction is tested @spec/done | A wrapper redacts on display; a unit test proves it @spec/done | A redaction that was assumed but never verified @spec/done |

### Law 1 — never printed {#law-printed}

##the-value-goes-to-no-surface-the-tool-emits-lead The value goes to no surface the tool emits: @impl/done

- ##NOT-PRINTED-STDOUT not stdout, @impl/done
- ##NOT-PRINTED-STDERR not stderr, @impl/done
- ##NOT-PRINTED-THE-LOG
  not the log, @impl/done
- ##NOT-PRINTED-A-JSON-OR-EVENT-STREAM not a `--json` or event stream, @impl/done
- ##NOT-PRINTED-AN-ERROR-MESSAGE not an error message, @impl/done
- ##NOT-PRINTED-A-PANIC-OR-STACK-TRACE
  not a panic or stack trace, @impl/done
- ##NOT-PRINTED-TELEMETRY not telemetry, @impl/done
- ##NOT-PRINTED-THE-LOCKFILE not the lockfile. @impl/done

##WHEN-THE-TOOL-MENTIONS-A-CREDENTIAL-IT-NAMES-THE-SOURCE Where
the tool must *mention* a credential — "using the token from
`$DEPLOY_TOKEN`", "reading `<config-dir>/host.token`" — it names the
**source**. @impl/done

##THE-VALUE-IS-NEVER-INTERPOLATED The value is never interpolated into a message, an
error, or a structured record. @impl/done

### Law 2 — never persisted outside one place {#law-persisted}

##EXACTLY-ONE-SANCTIONED-AT-REST-LOCATION There is exactly one sanctioned at-rest location for a secret: a
per-user, permission-protected file in the tool's own config
directory, readable only by the owner. @impl/done

##ci-substitutes-a-secret-environment-variable (CI substitutes a secret
environment variable, injected by the CI platform's secret store.) @impl/done

##THE-VALUE-IS-NEVER-COMMITTED-CACHED-OR-DROPPED-IN-TREE
The value is never committed to the repository, never written into
the lockfile, never embedded in a cache file, never dropped into the
project's working tree. @impl/done

##A-CREDENTIALED-URL-IS-BUILT-IN-MEMORY-AND-DISCARDED When the tool needs a credentialed URL, it
reads the secret from the environment or the sanctioned file, builds
the URL **in memory**, hands it to the child process, and discards
it — the value never touches disk by the tool's hand, and the
lockfile records only the canonical, credential-free URL. @impl/done

### Law 3 — sanctioned process boundaries only {#law-boundaries}

##audited-paths-lead A secret may leave the process only by a small set of audited paths: @impl/done

- ##PATH-TLS-AUTHORIZATION-HEADER A **TLS `Authorization: Bearer …` header** to the host API. @impl/done
- ##PATH-SINGLE-CHILD-PROCESS-INVOCATION A **single child-process invocation** with the credential embedded
  in a URL (`https://x-access-token:<TOKEN>@host/…`), relying on the
  child tool's own redaction of URL passwords in its output. (Modern
  version-control clients redact URL passwords in their own logs.) @impl/done

##NO-OTHER-PATH-IS-ALLOWED No other path is allowed. @impl/done

##NEVER-IN-A-THIRD-PARTY-SCRIPTS-ENVIRONMENT In particular the value is **never placed
in the environment of a spawned third-party script** — install and
build hooks run unseen third-party code, and a secret in their
environment is a secret handed to code no one reviewed
([`third-party-code-consent.md`](third-party-code-consent.md)). @impl/done

### Law 4 — redaction is tested, not promised {#law-tested}

##THE-WRAPPER-REDACTS-ON-DISPLAY-AND-DEBUG The in-process wrapper that carries a secret redacts the value on its
display and debug representations, so an accidental `print(token)`
emits `***` rather than the value. @impl/done

##THE-REDACTION-IS-BACKED-BY-A-UNIT-TEST That redaction is **backed by a
unit test** that constructs the wrapper around a known value and
asserts the value does not appear in either representation. @impl/done

##A-PROMISE-IS-NOT-A-PASSING-TEST A comment
that says "this is redacted" is a promise; a passing test is a fact. @impl/done

##EVERY-NEW-CODE-PATH-IS-REVIEWED-AGAINST-THE-FOUR-LAWS Every new code path that touches a secret is reviewed against these
four laws before it merges. @impl/done

## The agent era changes the threat model {#agent-era}

##a-traditional-policy-assumes-the-value-is-seen-once A traditional secrets policy assumes the operator sees the value once
and moves on. @spec/done

##three-ways-each-tightening-the-rule An agent-driven repository breaks that assumption in
three ways, and each one tightens the rule. @spec/done

- ##AGENT-ERA-SESSIONS-MAY-BE-RECORDED **Sessions may be recorded or logged.** Screen capture, transcript
  logging, shared session archives — the session is not ephemeral.
  A value spoken once persists wherever the session persists. @spec/done
- ##AGENT-ERA-ONE-ECHO-IS-A-LEAK **One echo is a leak.** There is no "just this once" for a secret.
  A single reflection of the value into chat, a diff, or output is a
  disclosure into a medium that may be recorded, replayed, or shared.
  The cost of one echo equals the cost of full disclosure. @spec/done
- ##AGENT-ERA-NEVER-LOAD-A-SECRET-INTO-THE-CONVERSATION **Never load a secret into the conversation.** Secret files are
  edited in an editor directly. They are **never** `cat`'d, never
  read with a file-reading tool, never `echo`'d — because doing so
  pulls the value into the conversation context, which is exactly the
  recorded, replayable medium the policy keeps secrets out of. @impl/done

### The accidental-read drill {#accidental-read}

##accidental-read-drill-lead If a secret value lands in context despite the rule — a mis-aimed
read, a tool that dumped a file — the response is mechanical: @impl/done

1. ##ACCIDENTAL-STOP **Stop.** Do not continue the action that surfaced it. @impl/done
2. ##ACCIDENTAL-DO-NOT-PROPAGATE **Do not propagate.** Do not quote the value back, do not echo it
   into a commit message, do not include it in a diff, do not
   summarize "the token is …". Every one of those is a second
   surface. @impl/done
3. ##ACCIDENTAL-ROTATE **Rotate.** Tell the human the value is compromised and must be
   rotated. Once a value may have been captured, it is dead — see the
   leak drill below. @impl/done

## Blast radius — why the rules are global {#blast-radius}

##global-invariants-lead These rules are global invariants, not module-local conventions,
because the failure modes are catastrophic in a way local discipline
cannot bound: @impl/done

- ##BLAST-A-LEAKED-TOKEN-IS-THE-ORGANIZATION A leaked **publish or deploy token** is the whole organization:
  cross-repository writes, branch deletions, CI-secret reads. @spec/done
- ##BLAST-AN-ESCALATED-INTEGRATION-IS-THE-ACCOUNT An **escalated integration** is the whole host account: every
  resource the credential can reach, not just the intended one. @spec/done

##THE-ONLY-SAFE-POSTURE-IS-GLOBAL-RULES-AND-AUDIT When one leak costs the org and one escalation costs the account, the
only safe posture is to make the rules apply everywhere and to audit
**every** code path that touches a secret or acts under one — not to
trust that each module reinvented the discipline correctly. @impl/done

##scope-discipline-follows-the-same-logic Scope
discipline follows the same logic in
[`scope-discipline.md`](scope-discipline.md). @impl/done

## Suspected-leak drill — rotate first {#leak-drill}

##leak-drill-lead When a secret **may** have been exposed — printed, committed, read
into a recorded session, pasted anywhere — the order is fixed: @impl/done

1. ##LEAK-ROTATE-FIRST **Rotate first.** Revoke and reissue the credential at its issuer
   immediately, before investigating anything. The leaked value is
   dead the moment it *may* have been seen; a revoked credential
   cannot be abused no matter who captured it. @impl/done
2. ##LEAK-INVESTIGATE-SECOND **Investigate second.** Only after the live credential is dead,
   trace how it leaked and close the path — the log sink, the code
   line, the tool call that surfaced it. @impl/done
3. ##LEAK-PURGE-BUT-ASSUME-CAPTURE **Purge where feasible, but assume capture.** Scrub the value from
   logs and history where you can, but treat the value as already
   captured. Purging reduces exposure; rotation is what actually ends
   it. Never let "we can just delete the log" substitute for
   rotation. @impl/done

##investigating-first-is-the-classic-mistake Investigating first is the classic mistake: every minute spent
diagnosing while the live credential sits exposed is a minute an
attacker can spend using it. @spec/done

##KILL-IT-THEN-DIAGNOSE Kill it, then diagnose. @impl/done

## Re-derive for your project {#re-derive}

##COPY-THE-PROMPT-TASK-NOT-THE-PROMPT-IMPLEMENTATION Copy the prompt-task, not the prompt-implementation. @impl/done

##re-derive-lead Paste this to
your agent in a fresh session: @impl/done

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

- ##SUM-VALUE-NEVER-APPEARS-SOURCE-MAY A surface secret's **value** never appears on any surface; its
  **source** may be printed freely. @impl/done
- ##SUM-THE-FOUR-LAWS Four laws: never printed, never persisted outside the one
  sanctioned location, sanctioned process boundaries only, redaction
  tested rather than promised. @impl/done
- ##SUM-THE-AGENT-ERA-TIGHTENS-THE-RULE The agent era tightens the rule: sessions may be recorded, one echo
  is a leak, never read a secret into the conversation, and the
  accidental-read drill is stop / do-not-propagate / rotate. @impl/done
- ##SUM-BLAST-RADIUS-MAKES-THE-RULES-GLOBAL Blast radius makes the rules global — one token leak is the org,
  one escalation is the account. @spec/done
- ##SUM-ROTATE-FIRST-INVESTIGATE-SECOND On any suspected leak: rotate first, investigate second. The value
  is dead the moment it may have been seen. @impl/done
