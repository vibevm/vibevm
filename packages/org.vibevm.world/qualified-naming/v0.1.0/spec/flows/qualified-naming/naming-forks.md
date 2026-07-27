# Naming forks {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The design forks a namespace author faces,
each with its options, the choice this practice recommends, and the
reasoning — so a future designer need not re-litigate settled ground. @impl/done

##four-forks Four forks: flat versus grouped, enforce versus recommend, where short
names live, and rename as alias versus new identity. @impl/done

##protocol-document-pointer The laws these forks produce are in [`QUALIFIED-NAMING-PROTOCOL.md`](QUALIFIED-NAMING-PROTOCOL.md). @impl/done

## Fork 1 — flat versus grouped {#flat-vs-grouped}

##fork-1-question The root fork: is an artifact addressed by a bare `name` unique across
the whole registry, or by a `group`-qualified coordinate? @spec/done

##both-models-shipped-at-scale Both models shipped at scale, so the trade is empirical, not
theoretical: @spec/done

| Aspect | Flat (Cargo, npm-unscoped) | Grouped (Maven `groupId:artifactId`) |
|---|---|---|
| ##ROW-ASPECT-SHORT-NAME-ERGONOMICS Short-name ergonomics @spec/done | best — `serde`, `express` @spec/done | worse — `org.example:widget` @spec/done |
| ##ROW-ASPECT-SQUATTING-PRESSURE Squatting pressure @spec/done | high — names are a finite commons @spec/done | low — squatting is local to a group @spec/done |
| ##ROW-ASPECT-TRUST-DELEGATION Trust delegation @spec/done | per-artifact — the name names no owner @spec/done | per-group — a group maps to an owner @spec/done |
| ##ROW-ASPECT-COMPOSITION Composition @spec/done | transitive name clashes possible @spec/done | clash-free — the group disambiguates @spec/done |
| ##ROW-ASPECT-ONBOARDING-COST Onboarding cost @spec/done | trivial @spec/done | a group must be chosen up front @spec/done |

##flat-systems-bought-ergonomics-and-paid-with-squatting **What each bought and paid.** Flat systems bought day-one ergonomics
and paid with a squatting arms race and a trust vacuum — `serde` reads
clean, but nothing in the coordinate tells you *who* stands behind it,
and two deep dependencies wanting the same bare name cannot both be
satisfied. @spec/done

##grouped-systems-paid-verbosity-and-bought-it-back-twice Grouped systems paid verbosity up front — nobody enjoys
typing `org.apache.commons` — and bought it back twice: a group maps to
an owner (so trust is delegated wholesale, not re-earned per artifact),
and group-qualified coordinates compose without collision at any depth. @spec/done

##CHOSEN-GROUPED **Chosen: grouped.** @impl/done

##the-verbosity-is-real-but-bounded The verbosity is real but bounded, and Fork 3
buys most of it back with a short name at the human boundary. @spec/done

##SQUATTING-AND-TRUST-PROPERTIES-ARE-STRUCTURAL The
squatting and trust properties are structural — they cannot be patched
onto a flat namespace after the fact. @spec/done

##npm-conceded-the-point-with-scope npm itself conceded the point by
bolting on `@scope/` once flat names ran out; starting grouped avoids
the migration. @spec/done

## Fork 2 — enforce style versus recommend it {#enforce-vs-recommend}

##fork-2-question Given a group grammar, how hard does the core enforce the reverse-FQDN
*convention*? @spec/done

- ##OPTION-A-ENFORCE **Option A — enforce.** Require the group to be a domain the
  publisher provably controls (DNS check, TXT record, the works). @spec/done
- ##OPTION-B-RECOMMEND **Option B — recommend.** Enforce only the *grammar* (lowercase,
  dot-separated segments) and the *uniqueness* of the group; leave
  reverse-FQDN as a convention for humans and linters. @spec/done

##CHOSEN-RECOMMEND **Chosen: recommend.** @impl/done

##reverse-fqdn-piggybacks-on-dns-uniqueness Reverse-FQDN is worth recommending because it
piggybacks on DNS's existing global uniqueness — the trick Sun adopted
for Java packages in 1995 — so two independent authors almost never
collide by accident. @spec/done

##enforcing-domain-ownership-buys-little-and-costs-a-lot But enforcing domain ownership buys little and
costs a lot: it couples publishing to DNS administration, breaks for
internal registries with no public domain, and still does not stop a
determined bad actor who *does* own a domain. @spec/done

##THE-RESOLVERS-JOB-IS-NARROW The resolver's job is
narrow — check grammar, check uniqueness — and taste is left to
linters. @impl/done

##maven-made-the-same-call Maven made the same call: it recommends reverse-FQDN
groupIds and enforces none of it. @spec/done

##THE-GRAMMAR-IS-THE-CONTRACT-THE-STYLE-IS-GUIDANCE The grammar is the contract; the style is guidance. @impl/done

## Fork 3 — where do short names live {#short-names}

##fork-3-question If a short name is a convenience, *how much* of the system may see it? @spec/done

- ##OPTION-EVERYWHERE **Everywhere.** Short names are first-class: manifests, lockfiles,
  and dependency edges may all carry them. @spec/done
- ##OPTION-NOWHERE **Nowhere.** Ban short names entirely; humans type fully-qualified
  coordinates always. @spec/done
- ##OPTION-CLI-BOUNDARY-ONLY **CLI boundary only.** Short names are legal solely as human-typed CLI
  input, resolved once, and never persisted. @spec/done

##CHOSEN-BOUNDARY-ONLY **Chosen: boundary only.** @impl/done

##everywhere-re-imports-the-transitive-collision-problem "Everywhere" re-imports the flat namespace's
transitive-collision problem: a short name buried in a transitive
manifest is ambiguous at a point where no human is present to
disambiguate it. @spec/done

##nowhere-throws-away-the-ergonomic-win "Nowhere" is collision-safe but throws away the entire
ergonomic win of Fork 1's concession — nobody wants to type
`org.example.tools/widget` at a prompt. @spec/done

##BOUNDARY-ONLY-IS-THE-SWEET-SPOT "Boundary only" is the sweet spot, and its property is decisive:
short-name resolution happens *once*, for a human's argument, against an
index — then the qualified form is stored. @impl/done

##A-SHORT-NAME-NEVER-RECURSES-INTO-THE-GRAPH Because persisted state is
qualified-only, the dependency graph is built entirely from qualified
names, and **a short name never recurses into the graph**. @impl/done

##TRANSITIVE-COLLISIONS-BECOME-IMPOSSIBLE-BY-CONSTRUCTION Transitive collisions become impossible by construction rather than by a runtime
check. @impl/done

##this-is-the-cargo-npm-split-generalised This is exactly the cargo/npm split — `add serde` on the command
line, `serde = "1"` in the manifest — generalised into a law. @spec/done

## Fork 4 — rename: alias table versus new identity {#rename}

##an-author-wants-to-rename-a-published-package An author wants to rename a published package. @spec/done

##fork-4-question What does the system do? @spec/done

- ##OPTION-ALIAS-TABLE **Alias table.** Keep a mapping `old → new`; resolve the old
  coordinate to the new artifact so existing consumers keep working. @spec/done
- ##OPTION-NEW-IDENTITY **New identity.** Treat the renamed package as a genuinely new
  package: it starts a fresh version line, and the old coordinate is
  frozen (yanked or left as-is), never repointed. @spec/done

##CHOSEN-NEW-IDENTITY **Chosen: new identity.** @impl/done

##the-alias-table-is-seductive-but-re-introduces-ambiguity The alias table is seductive — it seems to
spare consumers a migration — but it re-introduces precisely the
ambiguity groups were built to remove. @spec/done

##under-an-alias-every-reader-must-consult-the-mapping Under an alias, two coordinates
now name one artifact, and every reader must consult the mapping to know
that `old/foo` and `new/foo` are the same bytes. @spec/done

##trust-stops-being-delegable-under-an-alias Trust stops being
delegable: the coordinate no longer tells the whole truth about
ownership, because the *real* owner is one hop away through a table the
reader must know exists. @spec/done

##NEW-IDENTITY-KEEPS-EVERY-COORDINATE-HONEST New identity keeps every coordinate honest. @impl/done

##A-CHANGED-TUPLE-IS-A-CHANGED-IDENTITY Identity is the tuple
`(group, name, version, content-hash)`; change the group or name and the
tuple changed, so the identity changed — this is a consequence of the
identity law, not an extra rule. @impl/done

##THE-OLD-COORDINATE-STAYS-WELDED-TO-ITS-BYTES The old `name@version` stays welded to
the bytes it always meant (no coordinate is *ever* reused for different
content), and the new name earns its own history from `0.1.0` forward. @impl/done

##A-CONSUMER-MIGRATES-DELIBERATELY A
consumer migrates deliberately, by editing a qualified name they can
see — not silently, through a redirect they cannot. @impl/done

## Summary {#summary}

- ##SUM-FLAT-VS-GROUPED **Flat vs grouped → grouped.** Verbosity is bounded and bought back at
  the boundary; squatting-resistance and delegated trust are structural
  and cannot be retrofitted onto flat names. @impl/done
- ##SUM-ENFORCE-VS-RECOMMEND **Enforce vs recommend → recommend.** Enforce grammar and uniqueness;
  leave reverse-FQDN as style for humans and linters. Enforcing domain
  ownership couples publishing to DNS for little gain. @impl/done
- ##SUM-WHERE-SHORT-NAMES-LIVE **Where short names live → CLI boundary only.** Resolved once against
  an index, then stored qualified — which is what makes transitive
  collisions impossible. @impl/done
- ##SUM-RENAME-IS-A-NEW-IDENTITY **Rename → new identity, not alias.** An alias re-introduces the
  ambiguity groups removed; a new identity keeps every coordinate
  honest and every version line attached to the bytes it named. @impl/done
