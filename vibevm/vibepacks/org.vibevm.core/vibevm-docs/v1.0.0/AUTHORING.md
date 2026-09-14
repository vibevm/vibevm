# How we write {#root}

<status stage="spec" state="plan" comment="style law for the VibeVM documentation; design draft 2026-09-10; in the campaign zone since 2026-09-11; moves into org.vibevm.core/vibevm-docs as AUTHORING.md when the package skeleton opens; cited from PROP-057 §16; written in English because English is the source language of everything it governs"/>

## 0. Why this document exists {#why}

Documentation written by language models fails smart readers in two ways, and the second is worse than the first.

The first is tics. There is a recognizable dialect of model prose: "delve", "robust", "seamlessly", "it is important to note", three of everything, a summary paragraph after every section. Readers spot it in one sentence and stop trusting the page. This part is easy: we keep a list and delete on sight (§3).

The second failure is misplaced density. A model writes a page as if the reader had already read every specification the page cites, then packs each paragraph with the specification's own terms. Senior engineers and people who train models for a living report that they cannot follow such pages. They are not the problem. The page assumed prior reading that never happened. Most of this document is about that (§2).

Everything here governs the English source text. Every other language, Russian included, is an adaptation of the English and follows §10.

## 1. The reader {#reader}

Write for one person. She is a senior engineer, or a researcher who has trained models, and she has never opened VibeVM. She has fifteen minutes. She arrived on this page from a search result and has read nothing else of ours. She wants to do one thing, or understand one thing, and get back to work. She notices bad prose the way a musician notices a wrong note, and she enjoys a good sentence for the same reason.

The test for every page: **would it work as the only page she ever reads?** If the honest answer starts with "but first read…", the page is not finished.

She is not a beginner in general. She is a beginner in VibeVM. Explain VibeVM. Do not explain what a lock file is for, unless ours does something unusual, in which case explain exactly that.

## 2. Where complexity is allowed to live {#complexity}

A documentation set has a fixed amount of difficulty in it. The question is not how to remove it but where to put it. Put it in containers the reader can see coming, and keep the corridors between the containers clear.

**Containers**, where density is welcome:

- `rule` blocks: the quoted normative text of a specification, with its address. Exact values, field names and edge conditions live here.
- reference tables and `derived` blocks: generated from code and schemas.
- `fence` and `example` blocks: real commands, real output.
- the glossary: one entry per term, and the term's only definition.

**Corridors**, where density is forbidden: every narrative paragraph. A corridor obeys six rules.

1. **Introduce before use.** A term appears on a page for the first time in one of two ways: as a link to its glossary entry, or with a half-sentence gloss in plain words ("the lock file, the record of which exact package versions this project uses"). Never as a bare noun the reader is expected to know already. This check is mechanical (§11).
2. **Two terms per sentence.** A narrative sentence carries at most two glossary terms. Three means the sentence is a container in disguise. Split it, or move it into a table.
3. **One new idea per paragraph**, named in the paragraph's first sentence.
4. **The page explains; the specification confirms.** "See the specification for details" is banned as a substitute for explaining. The explanation is complete on the page, in plain words. The `rule` block that follows adds the exact wording and the address. A citation is an addition, never a deferral.
5. **The ladder.** Order the concepts so that each rung uses only what the rungs below it established. If a page needs a concept that another page owns, state it here anyway, in one sentence. Repetition across pages is cheap. A reader sent away is expensive.
6. **Show, then name.** Where you can, present the thing before its term: show the file, then say "this is the lock file". Quanta Magazine does this with mathematics. It works for package managers.

## 3. Tics: delete on sight {#tics}

If a sentence collapses when you remove the tic, the sentence had nothing in it.

**Words.** delve, leverage, robust, seamless, seamlessly, comprehensive, crucial, powerful, flexible, intuitive, elegant, streamline, empower, unlock, harness, elevate, journey, landscape, tapestry, ecosystem as filler, paradigm, holistic, synergy, nuanced, granular, utilize, facilitate, functionality, capabilities as filler, cutting-edge, state-of-the-art, game-changer, various, numerous, a number of, multiple where the number is known, certain, specific and appropriate as filler, respective and respectively, essentially, ultimately, simply, just as a minimizer, easily.

**Phrases.** it is important to note; it is worth noting; note that; at its core; in essence; a testament to; navigate the complexities; in today's world; whether you are X or Y; not only … but also; this means that; this ensures that; is designed to; aims to; serves as; acts as; allows you to; enables you to; provides a way to; can be used to; let's dive in; in this section we will; in summary; in conclusion; key takeaways.

**Structures.** A summary sentence closing every section. A section called Overview, Summary or Conclusion. "First… Second… Finally" scaffolding where a list would do, and a list where prose would do. Three of everything. Headings that are full sentences or questions. A rhetorical question as an opener. Meta-commentary ("this document describes…"). Boxes called "Why this matters" or "Best practices", filled with platitudes. Hedged claims ("may potentially"). Half a sentence in bold. Exclamation marks. Emoji. Chains of em dashes: at most one per paragraph, and a full stop is usually better. Parenthetical asides that hold the real point.

**Density tics** are the ones the word list cannot catch. They are covered in §2 and caught by the checks in §11.

## 4. The register: an essay, not a manual {#register}

The writers to imitate are not other documentation sites. They are the people who explain difficult mechanisms to intelligent readers for a living.

- **Matt Levine, "Money Stuff".** Explains a financial mechanism in one plain paragraph before saying anything clever about it, never assumes you read the prospectus, and lets the joke arrive from the facts rather than from adjectives. This is the closest model to what we want: mechanism first, in plain words, and wit by understatement.
- **The Economist.** Short words over long ones; the first sentence carries the news; the active voice; no jargon the reader did not bring; humour by understatement. Its style guide starts from Orwell's rules, and so do we: prefer the short word, cut what can be cut, prefer the active voice, prefer the everyday word to the technical one, and break any of these rules before writing something ugly.
- **Quanta Magazine.** Frontier mathematics for smart non-specialists, without dumbing it down: one concept per rung, an analogy with its limits stated, the term introduced after the thing is shown.
- **John McPhee.** Structure is decided before the first sentence; concrete detail over abstraction; the reader is led, never lectured.
- **Increment and ACM Queue.** Practitioners writing for practitioners: honest about trade-offs, specific about what actually happens, unafraid to say "this part is slow".
- **Feynman and Bryson.** Delight is allowed. A well-explained mechanism is a pleasure, and the prose may show it.

What we take from all of them: a page has a spine, one claim or one task; the first paragraph says what the page is for in words a stranger understands; detail is concrete; sentences vary in length; and the author is a person, not a template.

## 5. Simplified Technical English for the technical passages {#ste}

Procedures, warnings, reference descriptions and explanations of errors follow ASD-STE100, adapted to our vocabulary.

- **One word, one meaning.** A technical noun has one name, fixed in the glossary, and no synonyms: a package is a package, never a bundle, a module or an artifact. A verb has one meaning: `install` puts a package into a project, and nothing else is "installed".
- **One instruction per sentence**, in the imperative, present tense, active voice. "Run `vibe install`." Not "The package can then be installed by running…".
- **Sentence length.** Procedural sentences: at most 20 words. Descriptive sentences: at most 25. Paragraphs: at most six sentences.
- **Warnings before the step**, never after. Conditions before the instruction: "If the lock file is missing, run…".
- **Sequences as numbered lists**, alternatives as bulleted lists, never a sequence hidden in a paragraph.
- **No noun clusters longer than three words.** "registry index feed poll interval" is five. Rewrite it.
- **Keep the articles.** "Open the lock file", not "open lock file".
- **No gerunds as verbs** in instructions. "Install the package", not "Installing the package will…".
- **Numbers as numerals**, units spelled out once, no vague quantities where a number exists.

STE governs the technical passages, not the whole page. The corridors of §2 may be essay prose. The two registers sit side by side on every page, and the reader should feel the change: explanation, then exact instruction.

## 6. Humour {#humour}

Humour is rare, dry and placed.

- **At most one joke per page.** Most pages have none.
- **Never** inside a procedure, a warning, a reference table, an explanation of an error, a `rule` block or a title.
- **Placement:** the end of a section, a caption, a footnote, the last sentence of an introduction. Resting points, where the reader is not holding an instruction in mind.
- **Informative.** A joke that does not also explain something is cut. The allowed forms are understatement, the accurate absurd detail, and the plain statement of an awkward truth. Puns on product names, references to the author being a model, and self-deprecation about the software are not allowed.
- **Educated, not insular.** The reference may come from science, history, literature or ordinary life. It must not require an IT background, and it must not require the reader to be from any one country.
- **Survives adaptation.** If the joke depends on English wordplay, the adaptation replaces it with an equivalent or drops it (§10). It is never translated.

Allowed: "The lock file records what you actually got, which is not always what you asked for." Not allowed: "Let's unlock the power of lock files!"

## 7. The page skeleton {#skeleton}

Two kinds of page, two skeletons. Both end without a conclusion, a summary or a "next steps" box; the navigation does that.

**A concept page** ("The lock file"):

1. **Title.** A noun phrase. Not "Understanding…", "Introduction to…", "A guide to…".
2. **First paragraph.** What this is and when you need it, in plain words, with no glossary terms at all. Two to four sentences. This paragraph is also the page's line in `llms.txt`, so it has to stand alone.
3. **Show it.** An `example` with its `expect`: the shortest real command and its real output.
4. **How it works.** The ladder of §2. Corridors and containers alternate: a paragraph, a `rule`, a paragraph, a table.
5. **Edge cases and rules.** What happens when things are missing, stale or in conflict. Each normative value is quoted through `rule`, never paraphrased.
6. **Questions.** Only if real questions exist, and written as questions.

**A task page** ("Publish a package"). In VibeVM any action can be done by an agent or by hand, and the agent is the main road. This is the difference from documentation of the past, where everything was done by hand, so the page starts with the prompt:

1. **Title.** An imperative.
2. **First paragraph.** What you get and when you want it, no terms.
3. **Ask your agent.** A `prompt` block: the plain request a person types to their agent to get the result. One prompt, one outcome. Written in the user's voice ("Create a package named `org.example/notes` in this project and publish it to the default registry"), self-contained (the coordinate, the path and the registry are in the prompt, never implied), neutral to the agent (it works for any agent that has the `vibevm` skill), free of secrets. Under it, `needs`: what the agent must have (the skill, an MCP server, network or none), and `outcome`: what the person sees when it worked. Every task prompt carries at least one `assert`: a command that must succeed afterwards, because a prompt cannot be checked by its output the way a shell example can.
4. **What happens.** Three to six sentences: the commands the agent runs, the files that appear or change, how to check. This is the "how does it work" for the reader who stops here, so it is written as a corridor, not a container.
5. **By hand.** Numbered STE steps, only when the manual road is worth walking. Sometimes it is not; then the section is absent, not padded.
6. **Edge cases and rules**, then **Questions**, as above.

**Prompt craft.** Imperative, addressed to the agent. Name the thing and state the result. At most sixty words. No adjectives, no "please", no explanation inside the prompt — the explanation is the paragraph after it. If the task has a step that cannot be undone (publish, delete), the prompt says what the agent must confirm first. The English prompt is the source; other languages adapt it (§10), they do not translate it.

A page covers one concept or one task and fits one sitting. If it needs a second concept, that is a second page, and this page keeps one sentence about it.

## 8. Sentences {#sentences}

- Vary the length. After a long sentence, a short one.
- Concrete over generic: "three files", "the file `vibe.lock`", the real output. Not "several files", "the relevant configuration", "output similar to the following".
- Verbs over nominalizations: "the resolver checks", not "the resolver performs a check".
- "You" for procedures. "We" only for design rationale ("we chose content hashes because…"). Never "the user", never "one".
- No hedging where the behaviour is defined. No certainty where it is not.
- Colons introduce lists. Semicolons are rare. One em dash per paragraph at most. Italics mark a term at the moment it is defined and nothing else. Bold never appears in prose; identifiers go in code font.
- The last sentence of a paragraph is a real sentence, not a bridge to the next one.

## 9. Before and after {#examples}

The examples are illustrative; the commands in them are not normative.

**The density trap.**

Before: *The resolver materialises the closure declared by `[requires]` into `vibedeps/` once the index feed is reconciled with the lock pins; unison versioning applies across the family to the `-lang` and `-mcp` satellites, whereas `-docs` remains outside unison per PROP-028.*

After: *When you run `vibe install`, vibe reads the packages your project asks for, finds the exact versions named in your lock file, and copies them into the `vibedeps/` folder. Packages that belong to one family move together: update the language package, and its MCP companion updates to the same version. Documentation packages are the exception. They keep their own version, so a fix in the manual never forces a new release of the tool.* Then a `rule` block quoting PROP-028 on family versioning.

**Tics.**

Before: *VibeVM is a powerful, flexible package manager that seamlessly streamlines how you leverage specifications. It's important to note that at its core, it empowers you to navigate the complexities of context engineering.*

After: *VibeVM installs specifications the way npm installs libraries. You name a package, and the text your agent reads at the start of a session is assembled from it.*

**Deferral.**

Before: *The naming rules for groups are defined in PROP-003; see the specification for details.*

After: *A group name is a reversed domain, like `org.vibevm.core`: lower-case, dots between the parts, no part starting with a digit. The rule, with its edge cases, in the specification's own words:* followed by the `rule` block.

**A procedure in prose.**

Before: *Publishing involves making sure that you've built the package and that your token is configured, after which the publish command can be run, and you should then verify the result in the registry.*

After:

1. Build the package: `vibe package`.
2. Check that the token file exists: `ls ~/.vibe/github.publish.token`.
3. Publish: `vibe publish`.
4. Open the package's page in the registry and check the version number.

## 10. Adaptations {#adaptations}

English is the source text of all documentation; `org.vibevm.core/vibevm-docs` carries `lang = "en"`. Every other language, Russian included, is an **adaptation**, published as its own package (`VISION.md`, D-18). Block for block, the adaptation mirrors the source: the same anchors, the same number and kinds of blocks, `example ref` instead of examples of its own. Sentence for sentence, it mirrors nothing. The adapter writes the sentence a native editor would write.

- **Terms** follow the bilingual glossary of the adaptation package: one term, one word, in that language too. Where the language has no settled word, keep the English term in code font and gloss it once.
- **Jokes** are replaced with an equivalent in the target language or dropped. Never translated.
- **Register** follows the best popular-science editing in that language. For Russian, the standard is set by «Элементы», N+1 and «Кот Шрёдингера», with Nora Gal's «Слово живое и мёртвое» as the canon against канцелярит. From Ilyahov's infostyle take only the war on clutter; the rest is too dry for an essay.
- **Russian tics to delete on sight:** является, представляет собой, данный, осуществлять, реализовывать as a vague verb, в рамках, в данной статье, давайте разберёмся, стоит отметить, важно понимать, ключевой, мощный, гибкий, бесшовный, экосистема, функционал, решение for "product", позволяет as a crutch, а также as a connector; «Вы» with a capital letter; calques such as «вы можете» where the imperative or «можно» is natural. Sentences get shorter in Russian, not longer. The letter ё is used consistently.
- **Machine translation** may produce a draft. It never produces the text. The adaptation is written by the same tier of author as the source (§12).

Wherever `VISION.md` and `AGENT-PLAN.md` say "translation", read "adaptation".

## 11. Checks: what the tool catches, what the author catches, what the owner reads {#checks}

`vibe doc check --style` is mechanical and gates the build:

- banned words and phrases from §3, per language, with the Russian list from §10;
- sentence length by block kind (20 words procedural, 25 descriptive) and paragraph length (6 sentences): errors in procedures and warnings, warnings in corridors;
- a glossary term whose first occurrence on a page is neither a link to the glossary anchor nor followed by a gloss in the same sentence: error;
- more than two glossary terms in one narrative sentence: warning;
- "see the specification", "refer to the spec", "as described in" without an explanation on the same page: error;
- headings named Overview, Summary, Conclusion, Next steps, Key takeaways: error;
- exclamation marks, emoji, more than one em dash per paragraph, bold in prose: error;
- a readability score per page: reported, not gated.

The author (§12) reads the page as the reader of §1 and checks what no tool can: the ladder, a first paragraph free of terms, the placement of the one joke if there is one, the rhythm.

The owner reads three pages aloud at the phase gate. A page that cannot be read aloud without stumbling goes back.

## 12. Who writes {#who}

Prose is written by the strongest model available in the central session; the owner has named Fable, Astra and Sol. It is never written by a delegated worker. This covers pages, first paragraphs, the `title` and `abstract` of packages, the glossary, questions, the skill text, and adaptations into other languages. Workers, by the owner's directive Claude Opus 5 at high reasoning effort, do the work around the prose: fixtures, expected output, the block mirrors of adaptations, code, checks, the web shell. A worker's draft of prose is a draft. It is rewritten, not edited, before it ships.

The order of work for a page: outline the ladder, write, self-edit against §2–§8, run `vibe doc check --style`, and hand the page to the owner's spot-read at the gate.

## Two words that are not synonyms {#manual-and-documentation}

*Documentation* is the genre and the package kind: any package of kind `doc`, of any subject. *Manual* is this one, `org.vibevm.core/vibevm-docs`, and the pages say «this manual» when they mean the thing the reader is holding. Neither word is a glossary term, because both are ordinary English; the corpus audit of 2026-09-12 asked whether they were synonyms, and the answer is that they name two different things.
