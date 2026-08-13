# `flow:dev-runtime-docs` — setup docs that never drift {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-SAME-COMMIT-DOC-DISCIPLINE A vibevm `flow` package that installs a small, load-bearing discipline: a project's
**setup and runtime documentation is updated in the same commit** as any change to the
toolchain, prerequisites, environment, paths, or bootstrap steps. @status:impl/done

@fact:setup-docs-are-the-file-someone-opens-when-things-break Setup docs are the file someone opens when the build breaks or the environment is wrong. @status:spec/done

@fact:THE-UPDATE-IS-NEVER-SEPARABLE-FROM-THE-CHANGE A deferred doc update is exactly the drift these files exist to prevent — so the update is
never separable from the change that necessitates it. @status:impl/done

@fact:package-contents-lead This package ships: @status:impl/done

- @fact:CONTENT-THE-FULL-PROTOCOL `spec/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.md` — the obligation, why it is
  pinned in the project's foundational conventions, the contributor-vs-runtime audience
  split, and how to keep the docs honest. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/58-flow-dev-runtime-docs.md` — the boot snippet loaded at session start: the
  rule and the never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:dev-runtime-docs
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:dev-runtime-docs
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot snippet. @status:impl/done

## Philosophical background {#background}

@fact:extracted-from-vibevms-own-foundational-conventions Extracted from vibevm's own foundational conventions (PROP-000 §19): the obligation that a
change touching prerequisites/toolchain/env/paths ships with the matching guide update in the
same commit, pinned centrally so it is met during the boot-sequence read-order. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0 — The Universal Permissive License, Version 1.0. See `LICENSE`. @status:impl/done
