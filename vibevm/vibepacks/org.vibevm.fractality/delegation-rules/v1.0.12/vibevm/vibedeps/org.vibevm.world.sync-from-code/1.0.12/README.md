# `flow:sync-from-code` — reconcile specs with code when code changes first {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-SYNC-FROM-CODE-PROTOCOL A vibevm `flow` package that installs the **Sync-from-Code** protocol into
a project. @status:impl/done

@fact:SYNC-FROM-CODE-IS-THE-EXCEPTIONAL-PATH-OFF-TOP-DOWN-FLOW The normal information flow in a spec-driven project is
top-down (head → WAL → spec → code); Sync-from-Code is the *exceptional*
path for closing spec drift when code moves before the spec. @status:impl/done

@fact:two-legitimate-situations-lead Two legitimate situations break top-down flow: @status:impl/done

- @fact:SITUATION-THE-USER-EDITS-CODE-DIRECTLY The user edits code directly in the editor because it's faster than
  articulating the intent to the agent first. @status:spec/done
- @fact:SITUATION-THE-USER-GIVES-AN-IMPERATIVE-CHAT-COMMAND The user gives an imperative command in chat ("change the timeout to
  600 s") and the agent edits code without touching the spec. @status:spec/done

@fact:in-both-cases-the-spec-is-now-wrong-and-gets-fixed-back In both cases the spec is now wrong; left unreconciled, the next session
reads the stale spec, concludes the code is in error, and "fixes" the
code back — correctly by the spec-wins rule, wrong in outcome. @status:spec/done

@fact:THE-FLOW-IS-THE-SANCTIONED-WAY-TO-CLOSE-THAT-GAP This
flow is the sanctioned way to close that gap. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-FULL-PROTOCOL `spec/flows/sync-from-code/SYNC-PROTOCOL.xml` — full protocol: what
  Sync-from-Code is, when to run it, what the draft spec diff must
  contain (value + reason + revisit trigger), and what it explicitly
  does not do. @status:impl/done
- @fact:CONTENT-THE-DECISION-TABLE `spec/flows/sync-from-code/when-to-apply.xml` — decision table:
  *should I run it right now?*, including the cases where you should
  **not** (temporary hacks, mechanical changes, unnamed reasons). @status:impl/done
- @fact:CONTENT-THE-REVIEW-CHECKLIST `spec/flows/sync-from-code/review-workflow.xml` — human-side checklist
  for the approval step: six checks that catch bad syncs before they
  land. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/20-flow-sync-from-code.xml` — boot snippet loaded at
  session start, pointing the agent at the protocol. @status:impl/done

## Install {#install}

```bash
vibe install flow:sync-from-code
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:sync-from-code
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files (`00-core.xml`, `90-user.xml`, `WAL.md`) are
never touched. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-WAL-AND-ATOMIC-COMMITS-BY-DISTINCT-PREFIXES Works with `flow:wal` (`10-…`) and `flow:git-atomic-commits` (`30-…`):
  numeric boot-snippet prefixes are distinct by design. @status:impl/done
- @fact:COMPOSES-WAL-FOR-THE-SESSION-END-UPDATE A successful sync *may* trigger a WAL update; that update goes
  through `flow:wal`'s session-end hook, not this flow. @status:impl/done
- @fact:COMPOSES-ATOMIC-COMMITS-FOR-THE-COMMIT-MESSAGE A sync ends in a `docs(spec)` commit; message formatting is pinned
  by `flow:git-atomic-commits`. @status:impl/done

## Philosophical background {#background}

@fact:extracted-from-the-books-third-chapter The protocol is extracted from *AI-native development*, chapter 3
(*"Архитектура памяти"*, subsection "Протокол Sync-from-Code"). @status:spec/done

@fact:short-version-spec-driven-projects-need-the-inverse-path Short
version: spec-driven projects need a named, rare, human-approved path
for the inverse flow; without one, drift accumulates silently and the
spec stops being authoritative. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0 — The Universal Permissive License, Version 1.0. See `LICENSE` and the surrounding registry for distribution terms. @status:impl/done

