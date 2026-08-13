# Flow: Spec Genres {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-DOCUMENTS-ARE-GENRE-TYPED This project's documents are **genre-typed**. @status:impl/done

@fact:UNDIFFERENTIATED-PILE-OF-MARKDOWN-ROTS One undifferentiated
pile of markdown rots: contracts bloat with narrative, narrative
gets treated as binding, and nobody knows what wins. @status:spec/done

@fact:EVERY-DOCUMENT-BELONGS-TO-EXACTLY-ONE-GENRE Every document
belongs to exactly one genre; the genre decides where it lives, how
it may change, and what authority it carries. @status:impl/done

## The genre map {#genre-map}

| Genre | Holds | Binding? |
|-------|-------|----------|
| @fact:ROW-GENRE-BOOT-FILES Boot files @status:impl/done | Standing instructions read at session start @status:impl/done | yes @status:impl/done |
| @fact:ROW-GENRE-FOUNDATIONAL-DECISIONS Foundational decisions @status:impl/done | Choices that cross every module @status:impl/done | yes @status:impl/done |
| @fact:ROW-GENRE-MODULE-CONTRACTS Module contracts @status:impl/done | What each module does (here: PROP / FEAT) @status:impl/done | yes @status:impl/done |
| @fact:ROW-GENRE-DESIGN-DOCS Design docs @status:impl/done | Why we chose what we chose — the lore @status:impl/done | no @status:impl/done |
| @fact:ROW-GENRE-RESEARCH-DOCS Research docs @status:impl/done | What *other* projects did @status:impl/done | no @status:impl/done |
| @fact:ROW-GENRE-CAMPAIGN-PLANS Campaign plans @status:impl/done | Phases and gates of one multi-session change @status:impl/done | no @status:impl/done |
| @fact:ROW-GENRE-THE-CHECKPOINT The checkpoint @status:impl/done | Where work stands right now @status:impl/done | state, not truth @status:impl/done |

@fact:full-charters-pointer Full charters, mutability rules, and conflict authority:
@spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#root. @status:impl/done

## Core rule {#core-rule}

@fact:NAME-THE-GENRE-BEFORE-WRITING-ANY-DOCUMENT **Before writing any project document, name its genre first.** Then: @status:impl/done

1. @fact:CONTRACT-WINS-OVER-LORE **Contract wins over lore.** When a design document and the
   contract it explains disagree, the contract wins and the design
   document is corrected — lore never silently diverges. @status:impl/done
2. @fact:KEEP-THE-TWO-WAY-LINKS **Keep the two-way links.** A contract section that has lore
   links to it; the lore names the section it explains. A cold
   reader entering from either side finds the other. @status:impl/done

@fact:routing-table-pointer Routing table for new material:
@spec://org.vibevm.world/spec-genres/flows/spec-genres/when-to-write-what#root. @status:impl/done

@fact:design-docs-pointer The contract/lore split in practice:
@spec://org.vibevm.world/spec-genres/flows/spec-genres/design-docs#root. @status:impl/done

## Why this matters in a human-AI team {#why}

@fact:AGENT-READS-THE-TREE-COLD-EVERY-SESSION The agent reads the tree cold every session. @status:spec/done

@fact:look-alike-prose-gets-implemented-or-softened If binding and
non-binding prose look alike, it will implement a parked idea out of
a design doc, or soften a contract because the narrative around it
sounded tentative. @status:spec/done

@fact:GENRE-TYPING-ASSIGNS-AUTHORITY-WITHOUT-ASKING Genre typing is what lets a cold reader assign
authority to a sentence without asking anyone. @status:spec/done

## Never {#never}

- @fact:NEVER-PUT-NORMATIVE-LANGUAGE-IN-A-DESIGN-DOC Never put normative language — "must", "shall", requirement
  lists — in a design doc. Extract it to the contract; link back. @status:impl/done
- @fact:NEVER-EDIT-THE-CONTRACT-TO-MATCH-THE-LORE Never resolve a contract-vs-lore conflict by editing the contract
  to match the lore. The correction runs the other way. @status:impl/done
- @fact:NEVER-CREATE-A-DOCUMENT-WITHOUT-DECIDING-ITS-GENRE Never create a document without deciding its genre. @status:impl/done
- @fact:NEVER-LET-LORE-GO-UNLINKED-FROM-ITS-CONTRACT Never let lore go unlinked from its contract — an unlinked design
  doc is invisible at the next cold start. @status:impl/done
