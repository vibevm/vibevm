# Spec Genres Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *why* project
documents are sorted into genres, *which* genres exist and what each
one is for, *who wins* when two of them disagree, and *how* the
two-way links keep the non-binding genres reachable from a cold
start. @impl/done

##this-file-is-the-taxonomy It is the taxonomy; the sibling documents are the routing
table and the contract/lore split in practice. @impl/done

## Why genres exist {#why}

##three-failures-lead Left alone, a spec tree becomes one undifferentiated pile of
markdown, and three failures follow, every time: @spec/done

- ##FAILURE-CONTRACTS-BLOAT-WITH-NARRATIVE **Contracts bloat with narrative.** A discussion log — every fork,
  every analogy, every rejected branch — gets poured into the
  document that an implementer must read as a contract. Now the
  contract is unreadable as a contract. @spec/done
- ##FAILURE-NARRATIVE-GETS-TREATED-AS-BINDING **Narrative gets treated as binding.** A paragraph of "here is how
  we were thinking about it" sits next to a requirement, in the same
  font, and a later reader implements the musing. @spec/done
- ##FAILURE-NOBODY-KNOWS-WHAT-WINS **Nobody knows what wins.** Two documents say different things and
  there is no rule for which one is authoritative, so the conflict is
  resolved by whoever edited last, not by design. @spec/done

##GENRE-TYPING-REMOVES-ALL-THREE Genre typing removes all three. @impl/done

##THE-KIND-FIXES-CHARTER-MUTABILITY-AUDIENCE-AND-AUTHORITY Each document declares what kind of
thing it is; the kind fixes its charter, its mutability, its
audience, and — decisively — its authority when it collides with
another document. @impl/done

## The genre table {#genres}

##EACH-GENRE-HAS-CHARTER-MUTABILITY-READER-AND-AUTHORITY Each genre has a **charter** (what it is for), a **mutability** (how
it is allowed to change), a **reader**, and an **authority** (what
wins on conflict). @impl/done

| Genre | Charter | Mutability | Reader | Authority |
|-------|---------|-----------|--------|-----------|
| ##ROW-GENRE-BOOT-FILES **Boot files** @impl/done | Standing instructions loaded at session start; small and stable @impl/done | Rare, deliberate edits @impl/done | Every session, first @impl/done | Binding @impl/done |
| ##ROW-GENRE-FOUNDATIONAL-DECISIONS **Foundational decisions** @impl/done | Choices that cross every module (identity, versioning, licensing) @impl/done | Amended by decision record @impl/done | Everyone @impl/done | Binding @impl/done |
| ##ROW-GENRE-MODULE-CONTRACTS **Module contracts** @impl/done | What one module does — its behaviour, constraints, invariants (this convention: PROP / FEAT) @impl/done | Edit + changelog line @impl/done | Implementers @impl/done | Binding @impl/done |
| ##ROW-GENRE-DESIGN-DOCS **Design docs** @impl/done | Why we chose what we chose — the narrative lore behind a contract @impl/done | Rewritten freely; corrected on contract conflict @impl/done | Anyone tracing intent @impl/done | Non-binding @impl/done |
| ##ROW-GENRE-RESEARCH-DOCS **Research docs** @impl/done | What *other* projects/systems did — external studies, prior art @impl/done | Superseded by newer study @impl/done | Anyone surveying the field @impl/done | Non-binding @impl/done |
| ##ROW-GENRE-CAMPAIGN-PLANS **Campaign plans** @impl/done | The phases and gates of one multi-session change @impl/done | Ticked as executed; archived when done @impl/done | The crew running it @impl/done | Non-binding (execution, not truth) @impl/done |
| ##ROW-GENRE-THE-CHECKPOINT **The checkpoint** @impl/done | Where work stands *right now* — branch, blocker, next step @impl/done | Overwritten every session @impl/done | The next session, first @impl/done | State, not truth @impl/done |

##two-axes-separate-the-genres Two axes separate the binding genres from the rest. @impl/done

##BINDING-GENRES-SAY-WHAT-THE-SYSTEM-MUST-DO **Boot,
foundational decisions, and module contracts are binding** — they say
what the system *is* and *must do*. @impl/done

##NON-BINDING-GENRES-EXPLAIN-THEY-DO-NOT-REQUIRE **Design and research are
non-binding** — they explain, they do not require. @impl/done

##VOLATILE-GENRES-TRACK-EXECUTION-AND-STATE **Campaign plans
and the checkpoint are volatile** — they track execution and state,
which change faster than truth and must never be mistaken for it. @impl/done

### Contract versus lore, precisely {#contract-vs-lore}

##the-line-that-does-the-most-work The line that does the most work is between a **module contract** and
a **design doc**. @impl/done

##LOAD-BEARING-RATIONALE-STAYS-INSIDE-THE-CONTRACT The load-bearing rationale — the decision itself,
its one-paragraph why, the constraints, the invariants — stays
*inside* the contract. @impl/done

##NARRATIVE-RATIONALE-MOVES-OUT-INTO-A-DESIGN-DOC The narrative rationale — the full discussion,
the forks weighed at length, the precedents studied, the ideas parked
for later — moves *out* into a design doc. @impl/done

##what-the-split-buys-both-sides The contract stays
readable as a contract; the lore stays available to anyone who wants
the whole story. @spec/done

##RESEARCH-AND-DESIGN-POINT-IN-OPPOSITE-DIRECTIONS Design and research both point away from the contract, but in
opposite directions: **research studies what other projects did;
design records why we chose what we chose.** @impl/done

##a-competitor-backgrounder-is-research A backgrounder on a
competitor is research. @impl/done

##our-own-modules-reasoning-is-design The reasoning behind our own module is
design. @impl/done

## The precedence law {#precedence}

> ##CONTRACT-WINS-OVER-LORE When a design document and the contract it explains disagree, the
> contract wins and the design document is corrected. @impl/done

##not-a-tie-breaker-of-convenience This is not a tie-breaker of convenience; it is the rule that keeps
lore honest. @spec/done

##LORE-IS-ALLOWED-TO-BE-WRONG Lore is allowed to be wrong — it records what was true
when it was written, and the world moves. @impl/done

##LORE-MAY-NOT-SILENTLY-DIVERGE-AND-BE-CITED-AS-CURRENT What lore is *not* allowed
to do is silently diverge and then get cited as if current. @impl/done

##THE-CORRECTION-ALWAYS-RUNS-ONE-WAY So the
correction always runs one way: @impl/done

- ##FIX-THE-DESIGN-DOC-WHEN-THE-CONTRACT-MOVED The contract changed; a design doc now contradicts it → **fix the
  design doc**, add a line noting what moved. @impl/done
- ##A-FORBIDDEN-PROPOSAL-IS-A-PARKED-IDEA A design doc proposes something the contract forbids → the design
  doc is a **parked idea**, marked as such, never mistaken for the
  rule. @impl/done
- ##NEVER-EDIT-THE-CONTRACT-TO-MATCH-THE-LORE Never edit the contract to match the lore. That inverts the
  authority and lets a stale musing rewrite the system. @impl/done

##VOLATILE-NEVER-OVERRIDES-BINDING The same ordering holds for the volatile genres against the binding
ones: a campaign plan or a checkpoint entry never overrides a
contract. @impl/done

##A-DISAGREEING-CHECKPOINT-IS-STALE If the checkpoint says one thing and the contract another,
the checkpoint is stale — state posing as truth — and gets corrected. @impl/done

## The two-way linking law {#linking}

##a-non-binding-document-must-be-findable A non-binding document is only useful if a cold reader can *find* it. @spec/done

##THE-LINK-MAKES-LORE-SURVIVE-A-COLD-START The link is the mechanism that makes the lore survive a cold start. @impl/done

> ##TWO-WAY-LINKING-LAW A contract section that has lore links to it. The lore links back.
> A cold reader entering from either side finds the other. @impl/done

##concretely-lead Concretely: @impl/done

- ##EVERY-DESIGN-DOC-NAMES-ITS-CONTRACT-SECTIONS Every design doc names the contract section(s) it explains. @impl/done
- ##EVERY-CONTRACT-SECTION-WITH-LORE-LINKS-TO-IT Every contract section that has a design doc links to it — from a
  `Related` line, a footnote, an anchor reference — so a session
  reading the contract during boot discovers the rationale *without
  being told it exists*. @impl/done
- ##A-ONE-DIRECTIONAL-LINK-IS-A-LATENT-BREAK A one-directional link is a latent break: follow the tree from the
  unlinked side and the other half is invisible. Both directions or
  neither. @impl/done

##AN-UNLINKED-DESIGN-DOC-IS-A-DEFECT This is why an unlinked design doc counts as a defect, not merely
untidy. @impl/done

##why-an-unlinked-doc-costs It holds real reasoning that the next cold reader will never
reach, and so the project will re-derive — or re-litigate — a
question it already settled. @spec/done

## Placing a document {#placing}

##placing-routine-lead The routine is short, and it runs *before* the first sentence: @impl/done

1. ##PLACE-NAME-THE-GENRE **Name the genre.** Is this binding or explanatory? Does it
   describe our system (contract/design) or someone else's
   (research)? Is it truth (contract) or state (checkpoint/plan)? @impl/done
2. ##PLACE-PUT-IT-IN-THAT-GENRES-HOME **Put it in that genre's home** and give it that genre's shape —
   a contract reads as a contract, a design doc reads as a story. @impl/done
3. ##PLACE-WIRE-THE-LINKS **Wire the links** if it is lore: name its contract, and add the
   back-link from the contract. @impl/done
4. ##PLACE-DO-NOT-BLEND **Do not blend.** If the draft is half requirement and half
   story, it is two documents — split it along the binding line. @impl/done

##sibling-document-pointers The decision table for common situations lives in
[`when-to-write-what.md`](when-to-write-what.md); the contract/lore
split, with the fork-by-fork record shape, lives in
[`design-docs.md`](design-docs.md). @impl/done

## Re-derive for your project {#re-derive}

##DO-NOT-ADOPT-THIS-TABLE-VERBATIM Do not adopt this table verbatim — your project already has genres,
named or not. @impl/done

##re-derive-prompt-lead Have the agent surface them and map yours onto this
frame: @impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-spec-genres/<version>/spec/flows/spec-genres/`, check `vibe.lock`) in full, then adapt it to this project:
1. Inventory every kind of document this repo already holds (specs,
   ADRs, RFCs, design docs, wikis, READMEs, runbooks, planning docs,
   status files). Group them by the genre they actually play.
2. For each genre, name its charter, mutability, reader, and what
   wins when it conflicts with another — flag any genre with no
   conflict rule.
3. Find contract documents carrying paragraphs of narrative history,
   and design/planning documents written in binding language. List
   each as a misfiling to correct.
4. Check the binding<->lore links: list contract sections whose
   rationale lives elsewhere but is not linked, and lore documents
   that name no contract.
5. Show me the genre map and the misfilings as a plan. Change
   nothing until I approve.
```

## Summary {#summary}

- ##SUM-THE-PILE-ROTS-THREE-WAYS One undifferentiated pile of markdown rots three ways: contracts
  bloat, narrative goes binding, nobody knows what wins. @spec/done
- ##SUM-SEVEN-GENRES-THREE-AUTHORITIES Seven genres, three authorities: binding (boot, foundational,
  contracts), non-binding (design, research), volatile (plans, the
  checkpoint). @impl/done
- ##SUM-CONTRACT-WINS-OVER-LORE Contract wins over lore. Lore is corrected on conflict, never the
  other way, and never silently diverges. @impl/done
- ##SUM-TWO-WAY-LINKS-ARE-THE-MECHANISM Two-way links are the mechanism: a cold reader entering from either
  the contract or the lore finds the other. @impl/done
- ##SUM-NAME-THE-GENRE-FIRST Name the genre before writing the first sentence. @impl/done
