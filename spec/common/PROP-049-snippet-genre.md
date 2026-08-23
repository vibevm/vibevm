# PROP-049 — the snippet genre: no presupposed disciplines {#root}

<status stage="spec" state="work" comment="owner-approved wave 2026-08-23 («давай попробуем — всегда можно откатить»): the genre rule for boot snippets, enforced by construction (the installed: predicate + conditional fragments) and by gate (the concepts dictionary check); includes the redbook/wal unbundling"/>

## 1. The defect this repairs {#defect}

@fact:PRESUPPOSITION-DEFECT **Agents creating fresh VibeVM projects created
`spec/WAL.md` unasked (owner observation, 2026-08-23).** The machinery is
clean — `vibe init` scaffolds no WAL, and both WAL checks return silently
when the file is absent («a project convention, not part of the package
manager's contract»). The cause is the PACKAGE chain: *(a)* redbook hard-pulls
`flow:wal`, whose snippet speaks in unconditional imperatives («canonical…
read before anything else»); *(b)* four UNRELATED snippets presuppose the WAL
in passing — sync-from-code's «record in the WAL» imperative, health-audit's
and git-atomic-commits' «the WAL» asides — so even without redbook the
compiled STATIC of a small discipline set tells an agent a WAL exists. An
obedient agent reads its static lane and mints the file. Not every external
project wants a centralised WAL (a multi-developer team may run many WALs, or
none); the presupposition takes that choice away. @status:spec/done

## 2. The genre rule {#genre-rule}

@fact:SNIPPET-GENRE-RULE **A boot snippet never presupposes another
discipline.** A snippet speaks unconditionally only about its OWN flow;
any mention of another flow's artifacts or duties is CONDITIONAL — and the
only lawful conditional form is structural, not verbal: the mention lives in
a snippet fragment guarded by `when = "installed:<group>/<name>"`, so the
text physically enters a project's lanes only when that discipline is
actually installed. Prose hedges («if you keep a WAL», «or equivalent») are
NOT the lawful form — they are unverifiable by machine and still teach the
concept unasked. This is ##THE-LAYER-LAW's sibling: a presupposition must
never travel ahead of its own discipline. @status:spec/work

## 3. The mechanism — the `installed:` predicate {#installed-predicate}

@fact:INSTALLED-PREDICATE **`when` grows a second predicate class:
`installed:<group>/<name>`, resolved at INSTALL time.** Unlike `os:*`
(resolved by the reading session), installedness is known when the boot
artifacts are generated: a fragment whose condition holds participates
normally (static compilation included); one whose condition fails is OMITTED
ENTIRELY — no STATIC bytes, no INDEX entry. Physical absence, not a softened
sentence: the tokenomics-preferred form (PROP-048 — the inapplicable is never
loaded). @status:spec/work

@fact:SNIPPET-FRAGMENTS **The manifest grows optional snippet fragments.**
`[boot_snippet]` stays the single main declaration (unchanged manifests keep
parsing); an optional `[[boot_snippet.fragment]]` array adds `{source, when}`
entries — each fragment a separate authored file, each with its own `when`
(`os:*` or `installed:*`). Cross-discipline mentions move from the main
snippet into fragments. @status:spec/work

## 4. The gate — the concepts dictionary check {#gate}

@fact:CONCEPTS-DECLARATION **A flow declares its concept tokens.** The
manifest's optional `concepts = […]` (under `[boot_snippet]`) names the
tokens that mean this discipline — for `flow:wal`: `"WAL"`,
`"spec/WAL.md"`. The dictionary is assembled from the world (authored
packages and installed manifests), never hard-coded into the checker.
@status:spec/work

@fact:SNIPPET-PRESUPPOSITION-CHECK **The check: a foreign concept in an
unconditional snippet is an error.** A vibe-check cell scans authored
snippet sources: a token owned by package D, found word-bounded in the main
snippet (or any fragment NOT guarded by `installed:D`) of package P≠D, is an
ERROR naming the token, the owner and the repair («move the mention into a
fragment guarded by installed:<D>, or drop it»). **The dependency
exemption:** a declared `[requires]` on D guarantees co-installation, so P's
bare mention of D's concepts is lawful (wal-specspaces presupposes the wal it
hard-requires by construction). Code spans and fences are opaque to the scan,
as to every markup scanner. Proven by a red fixture; a future publish wave
runs the same cell as a C6 gate. Enforcement is three-point: the compiler
(fragments are the only lawful conditional form), the panel/check (this
cell), the publish gate. @status:spec/work

## 5. The repairs shipped with the wave {#repairs}

@fact:FOUR-SNIPPETS-REPAIRED **The four presupposing snippets move their WAL
mentions into `installed:org.vibevm.world/wal` fragments** — sync-from-code
(the flow line and the «record in the WAL» imperative), health-audit,
git-atomic-commits, conflict-protocol (its «WAL or equivalent» hedge becomes
a fragment too: the hedge form is exactly what §2 outlaws). The main
snippets keep their own-discipline text, rephrased where the WAL was
load-bearing. @status:spec/work

@fact:REDBOOK-WAL-UNBUNDLED **redbook stops hard-pulling `flow:wal`.** The
book still TEACHES the WAL method (its chapters are untouched); adopting the
discipline is an explicit `flow:org.vibevm.world/wal` install — installation
returns to meaning consent. The host adds its own direct
`flow:org.vibevm.world/wal` entry (static-transitive, like its
wal-specspaces neighbour), so the host's lanes keep the flow it genuinely
runs. **The catalog consequence:** once unbundled, the book snippet's member
lines about the wal family are presuppositions like any other — they move
into `installed:`-guarded fragments, so the edition's catalog shows the
members a project actually installed. @status:spec/work

@fact:WAL-SNIPPET-SCOPE-CLAUSE **The wal snippet names its own genre.** One
added clause: this flow is the single-developer, central-WAL convention;
a multi-developer project chooses its own session-durability scheme — many
WALs (see `flow:wal-specspaces` for the registered-subprojects form), or
none — by not installing this flow or by superseding it. @status:spec/work

## 6. Rollback {#rollback}

@fact:ROLLBACK-SHAPE **The owner's escape hatch, named.** Every change is
package-local or additive: fragments fold back into main snippets, the
redbook dependency line returns, the predicate and the check cell are
removable without touching any consumer contract. Nothing here moves an
anchor or a public wire format. @status:spec/done

## 7. Companions {#companions}

@fact:companions-list [PROP-035](../modules/vibe-workspace/PROP-035-spec-compiler.md)
(the link model whose `dynamic + when` this predicate extends),
[PROP-024](PROP-024-code-bearing-packages.md) (`[boot_snippet]` manifest
surface), [PROP-048](PROP-048-tokenomics.md) (##THE-LAYER-LAW,
##MECH-DYNAMIC-CONDITIONS), the wal and wal-specspaces flows. @status:spec/done
