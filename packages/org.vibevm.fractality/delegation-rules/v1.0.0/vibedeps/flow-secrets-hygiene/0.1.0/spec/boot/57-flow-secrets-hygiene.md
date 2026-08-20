# Flow: Secrets Hygiene {#root}

This repository is worked by a coding agent that reads a lot and
whose sessions may be **recorded or logged**. Under those conditions
a secret has no safe margin: one echo of its value into chat, a
diff, or a log is a leak. This flow is the standing rule for
handling credentials — tokens, keys, passwords — so that no code
path and no session ever puts a secret value on a surface.

## What counts as a secret {#surface-secret}

A **surface secret** is any credential value that must never appear
on a surface the tooling or the session produces. Publish and deploy
tokens, registry-API tokens, provider API keys, database passwords,
SSH passphrases — all surface secrets. The rule is about the
**value**: you may freely print the *source* of a secret (an env-var
name, a file path, "explicit flag"); you never print the value it
resolves to.

## The four laws {#laws}

1. **Never printed.** Not to stdout, stderr, logs, a `--json` or
   event stream, error messages, panic or stack traces, telemetry,
   or a lockfile. Print the source, never the value.
2. **Never persisted** outside the one sanctioned at-rest location:
   a per-user, permission-protected file in the tool's own config
   directory (or an environment variable, for CI). Never committed,
   never in the lockfile, never in a cache or the project tree.
3. **Sanctioned process boundaries only.** A secret crosses a
   process boundary only by a narrow, audited path — a TLS
   `Authorization` header; a single child-process call with the
   credential embedded in a URL, relying on that child tool's own
   redaction of URL passwords. No other path.
4. **Redaction is tested, not promised.** A wrapper type that
   redacts the value on display is backed by a unit test asserting
   the value never appears. A promise in a comment is not redaction.

## On accidental exposure {#accidental}

Secrets files are edited in an editor directly — never `cat`'d,
never read into the conversation with a file-reading tool. If a
secret value nonetheless lands in context: **stop, do not
propagate.** Do not quote it back, do not echo it into a commit
message, do not show it in a diff. Treat the value as compromised
and tell the human to rotate it — the value is dead the moment it
may have been seen.

## Never {#never}

- Never print, echo, quote, or paste a secret value — or the
  contents of a secret file — into chat, output, a log, a commit, or
  a diff. Print the source, never the value.
- Never read a secret file into the conversation with a file-reading
  or `cat`-style tool; edit it in an editor instead.
- Never commit or persist a secret anywhere but the one sanctioned
  per-user, permission-protected location.
- Never place a secret in the environment of a spawned third-party
  script (install/build hooks run unseen third-party code).
- Never let an integration act outside its declared scope; refuse
  scope escalation — it is an error, not a warning.
- On any suspected exposure: rotate first, investigate second.

Full protocol:
[`SECRETS-HYGIENE-PROTOCOL.md`](../flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL.md).
Scope rules: [`scope-discipline.md`](../flows/secrets-hygiene/scope-discipline.md).
Install-time code: [`third-party-code-consent.md`](../flows/secrets-hygiene/third-party-code-consent.md).
