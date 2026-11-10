# Project frame — example

Copy this file to `vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md` and edit it.
Its presence is the whole opt-in: with it, `vibe install` and
`vibe prompt build` generate the prefix; without it, nothing changes.

Everything below is **literal text**. There is no template language in this
version — no shell, no `HOME` read, no `INCLUDE`, no conditionals, no slot
grammar and no placement directives. Curly braces like `{this}`, fenced
examples, Markdown links and the literal strings `SYSTEM.md` or `<vibevm>`
are ordinary characters with no effect. Where the generated segments go is
fixed: your text first, then the entrypoint export, then the compiled lane.

---

You are working inside a vibevm-managed repository. The project's own
instructions and its compiled boot lane follow this frame; treat them as the
project's standing rules.

Three standing expectations of this project, as an example of what belongs
here:

1. Prefer the smallest change that makes the acceptance criterion true, and
   say which criterion it is.
2. When a command's output contradicts a document, the output wins and the
   document is corrected in the same change.
3. Never mark any part of this repository as machine-authored.

What does **not** belong in this file: anything that changes between
sessions. A task, a date, a branch name, a plan's current step or a machine
fact makes the prefix change for a reason a provider's cache cannot absorb,
and the prefix exists to be byte-stable. Put per-session material in the
request itself.
