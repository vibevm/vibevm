# `doc:vibevm-docs` — the VibeVM manual {#root}

<status stage="doc" state="work" audience="user"/>

@fact:PACKAGE-PURPOSE This package is the manual of VibeVM: what vibe is, how
to install it, what a project contains, how packages, registries and the lock
file work, how to work through a coding agent, how the lifecycle runs, and
how to write packages of every kind. It documents the host coordinate
`org.vibevm.core/vibevm` and is read, never installed. @status:doc/work

@fact:HOW-TO-READ Read it on the site at `https://vibevm.org/doc/`, or on your
own machine: `vibe cache add org.vibevm.core/vibevm-docs` warms the manual
into the machine store and `vibe doc serve` opens it in a browser without
any network. An agent reads the same pages as plain text through the
`vibevm-docs` skill this package declares. @status:doc/work

@fact:WHERE-THE-PAGES-ARE The pages live under `vibevm/vibespecs/`, one
concept or one task per file, in the XML dialect of the project with the
documentation vocabulary: `example` blocks that are run against the real
binary, `rule` blocks that quote a specification by its address, `derived`
blocks generated from the product, and `prompt` blocks that open every task
page with the request you give your agent. @status:doc/work

@fact:HOW-TO-WRITE-A-PAGE Writing rules live in [`AUTHORING.md`](AUTHORING.md)
beside this file: who the reader is, where complexity may live, the banned
words (`style/banned.en.txt`, `style/banned.ru.txt`), the two page skeletons
and the prompt-first order of a task page. The mechanical checks are
`vibe doc check --examples --citations --derived --translations --coverage
--media --style`; a page is committed only when they are green and the
author has read it as a stranger would. @status:doc/work

@fact:TRANSLATIONS Translations are separate packages named
`vibevm-docs-<lang>` in the same group, mirroring these pages file for file
and block for block; the first one, `vibevm-docs-ru`, mirrors this manual
and the site serves it beside the source. @status:doc/work
