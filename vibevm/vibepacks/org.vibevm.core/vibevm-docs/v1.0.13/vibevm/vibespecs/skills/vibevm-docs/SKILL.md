---
name: vibevm-docs
description: Use when the user asks how something in VibeVM works, what a vibe error means, or asks you to do a task the VibeVM manual describes (create a project, install or publish a package, work offline, set up a workspace, write a package of any kind). Find the page, quote the rule by its address, and take the page's prompt as your task.
---

<status stage="doc" state="work" audience="agent"/>

# The VibeVM manual, for an agent {#root}

@fact:DOCS-SKILL-PURPOSE This skill leads you to the VibeVM manual — the
package `org.vibevm.core/vibevm-docs` — and tells you how to read it with
the smallest number of tokens: one page per question, one rule per claim,
one prompt per task. The specifications stay the authority; the manual
explains them and cites their addresses. @status:doc/work

## Where the manual is {#where}

1. @fact:DOCS-SKILL-LOCAL Locally, when the store holds it: `vibe cache list`
   shows `org.vibevm.core/vibevm-docs`; then `vibe explain "spec://org.vibevm.core/vibevm-docs/<page>#<anchor>"`
   prints a page or a block, and `vibe doc serve` opens the reader on
   `127.0.0.1` for a person. If the store does not hold it and the user
   allows the network, `vibe cache add org.vibevm.core/vibevm-docs` warms
   it once. @status:doc/work
2. @fact:DOCS-SKILL-WEB On the web: `https://vibevm.org/doc/llms.txt` is the
   index with one line per page; `https://vibevm.org/doc/<page>.md` is any
   page as plain Markdown; `https://vibevm.org/doc/llms-full.txt` is the
   whole corpus, and `llms-small.txt` / `llms-medium.txt` fit smaller
   budgets. Prefer the index and one page over the full corpus. @status:doc/work

## How to use it {#how}

1. @fact:DOCS-SKILL-ON-ERROR When a vibe command fails, read the rule address
   the message names (`spec://…#…`) with `vibe explain`, then open the page
   that explains it — the manual's diagnostics page maps messages to pages.
   Do not guess a fix from the message alone. @status:doc/work
2. @fact:DOCS-SKILL-ON-QUESTION When the user asks how something works, answer
   from one page: the first paragraph is the plain answer, the `rule` blocks
   carry the exact wording and the address you quote. Prefer a page marked
   official; the site marks community documentation as such. @status:doc/work
3. @fact:DOCS-SKILL-ON-TASK When the user asks you to do what a task page
   describes, take the page's `prompt` block as your task: substitute the
   user's coordinates and paths, run what it says, then run the page's
   `assert` commands — the task is done only when every assert exits zero.
   Ask before any step the prompt marks as irreversible (publish, delete). @status:doc/work
4. @fact:DOCS-SKILL-ON-CHOICE When the user must choose a package or a guide,
   read the catalogue lines (title, publisher, language, abstract) and
   prefer official documentation; name the publisher of anything from the
   community. @status:doc/work

@fact:DOCS-SKILL-NEVER Never restate a normative value from memory when a
page quotes it; never load the full corpus when one page answers; never
send the user's private package content anywhere — the local reader and the
store keep it on the machine. @status:doc/work
