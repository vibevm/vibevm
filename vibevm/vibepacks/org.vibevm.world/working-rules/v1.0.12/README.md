# `flow:working-rules` — sessions that share a project, and work that ends {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-WORKING-RULES A vibevm `flow` package that installs the working rules for
sessions that share one project: who reads which page, how an owner's short
request is read, how a frozen completion list lets a campaign end, how a lead
with bounded judgement is kept inside its list, and how executors carry out one
packet each without deciding anything. @status:impl/done

@fact:EACH-ROLE-READS-ONE-PAGE Each role reads one short page and nothing else: an open lead reads
the protocol, a bounded lead reads its own page, an executor reads the executor
page and its packet. Long rule files read literally make a weak reader worse, so
the pages are kept separate on purpose. @status:impl/done

@fact:package-contents-lead This package ships, under `rules/`: @status:impl/done

- @fact:CONTENT-PROTOCOL `protocol.md` — the portable protocol: roles and profiles, reading the
  owner's words, finishing and stopping, split-mode responsibilities, bounded
  workers, credential isolation, synchronization and emergency preservation. @status:impl/done
- @fact:CONTENT-LEAD-BOUNDED `lead-bounded.md` — the whole working rules of a bounded lead. @status:impl/done
- @fact:CONTENT-WORKER `worker.md` and `packet.template.md` — the whole instruction surface of
  an executor, and the packet a lead writes for it. @status:impl/done
- @fact:CONTENT-DECOMPOSING `decomposing.md` — how an open lead prepares a completion list that a bounded lead can execute without spiralling: the scout, decompose, skeptic and revise procedure, and the traps a skeptic finds. @status:impl/done
- @fact:CONTENT-RUNS `runs.md` — the optional contract for managed execution resources. @status:impl/done
- @fact:CONTENT-TEMPLATES `project.template.md` and `local.template.md` — inactive templates for
  the two project-owned files. @status:impl/done
- @fact:CONTENT-EXTENSION `with-multi-user-planning.md` — read only when that planning package is also installed. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/13-flow-working-rules.xml` — the routing table read at session
  start, plus a fragment that appears only together with the planning extension. @status:impl/done

@fact:PAGES-ARE-OUTSIDE-THE-SPEC-ROOT The pages live outside the package's spec root so that they reach
every consumer byte for byte, whatever spec format the consumer materialises,
and keep the installed path the routing table names. @status:impl/done

## Project-owned files {#project-files}

@fact:THE-PACKAGE-NEVER-WRITES-PROJECT-FILES Two files belong to the project, at its root:
`WORKING-RULES.project.md`, the committed project policy, and
`WORKING-RULES.local.md`, the untracked local binding. Installing, updating or
removing the package never writes, replaces or activates them. Without them a
session works in ordinary mode. @status:impl/done

## Install {#install}

```bash
vibe install flow:working-rules
```

## License {#license}

@fact:license-line UPL-1.0. See `LICENSE.md`. @status:impl/done
