# Flow: Spec Genres {#root}

<status stage="impl" state="done"/>

##PROJECT-DOCUMENTS-ARE-GENRE-TYPED This project's documents are **genre-typed**. @impl/done

##UNDIFFERENTIATED-PILE-OF-MARKDOWN-ROTS One undifferentiated
pile of markdown rots: contracts bloat with narrative, narrative
gets treated as binding, and nobody knows what wins. @spec/done

##EVERY-DOCUMENT-BELONGS-TO-EXACTLY-ONE-GENRE Every document
belongs to exactly one genre; the genre decides where it lives, how
it may change, and what authority it carries. @impl/done

## The genre map {#genre-map}

| Genre | Holds | Binding? |
|-------|-------|----------|
| ##ROW-GENRE-BOOT-FILES Boot files @impl/done | Standing instructions read at session start @impl/done | yes @impl/done |
| ##ROW-GENRE-FOUNDATIONAL-DECISIONS Foundational decisions @impl/done | Choices that cross every module @impl/done | yes @impl/done |
| ##ROW-GENRE-MODULE-CONTRACTS Module contracts @impl/done | What each module does (here: PROP / FEAT) @impl/done | yes @impl/done |
| ##ROW-GENRE-DESIGN-DOCS Design docs @impl/done | Why we chose what we chose — the lore @impl/done | no @impl/done |
| ##ROW-GENRE-RESEARCH-DOCS Research docs @impl/done | What *other* projects did @impl/done | no @impl/done |
| ##ROW-GENRE-CAMPAIGN-PLANS Campaign plans @impl/done | Phases and gates of one multi-session change @impl/done | no @impl/done |
| ##ROW-GENRE-THE-CHECKPOINT The checkpoint @impl/done | Where work stands right now @impl/done | state, not truth @impl/done |

##full-charters-pointer Full charters, mutability rules, and conflict authority:
@spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#root. @impl/done

## Core rule {#core-rule}

##NAME-THE-GENRE-BEFORE-WRITING-ANY-DOCUMENT **Before writing any project document, name its genre first.** Then: @impl/done

1. ##CONTRACT-WINS-OVER-LORE **Contract wins over lore.** When a design document and the
   contract it explains disagree, the contract wins and the design
   document is corrected — lore never silently diverges. @impl/done
2. ##KEEP-THE-TWO-WAY-LINKS **Keep the two-way links.** A contract section that has lore
   links to it; the lore names the section it explains. A cold
   reader entering from either side finds the other. @impl/done

##routing-table-pointer Routing table for new material:
@spec://org.vibevm.world/spec-genres/flows/spec-genres/when-to-write-what#root. @impl/done

##design-docs-pointer The contract/lore split in practice:
@spec://org.vibevm.world/spec-genres/flows/spec-genres/design-docs#root. @impl/done

## Why this matters in a human-AI team {#why}

##AGENT-READS-THE-TREE-COLD-EVERY-SESSION The agent reads the tree cold every session. @spec/done

##look-alike-prose-gets-implemented-or-softened If binding and
non-binding prose look alike, it will implement a parked idea out of
a design doc, or soften a contract because the narrative around it
sounded tentative. @spec/done

##GENRE-TYPING-ASSIGNS-AUTHORITY-WITHOUT-ASKING Genre typing is what lets a cold reader assign
authority to a sentence without asking anyone. @spec/done

## Never {#never}

- ##NEVER-PUT-NORMATIVE-LANGUAGE-IN-A-DESIGN-DOC Never put normative language — "must", "shall", requirement
  lists — in a design doc. Extract it to the contract; link back. @impl/done
- ##NEVER-EDIT-THE-CONTRACT-TO-MATCH-THE-LORE Never resolve a contract-vs-lore conflict by editing the contract
  to match the lore. The correction runs the other way. @impl/done
- ##NEVER-CREATE-A-DOCUMENT-WITHOUT-DECIDING-ITS-GENRE Never create a document without deciding its genre. @impl/done
- ##NEVER-LET-LORE-GO-UNLINKED-FROM-ITS-CONTRACT Never let lore go unlinked from its contract — an unlinked design
  doc is invisible at the next cold start. @impl/done
