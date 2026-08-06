# The Omnichannel Protocol {#root}

<status stage="impl" state="done"/>

@fact:self-uri `spec://org.vibevm.world/omnichannel/flows/omnichannel/OMNICHANNEL-PROTOCOL` @status:spec/done

@fact:PROTOCOL-STATEMENT A capability lives in a library. Every way a user or an agent reaches
it — command line, TUI, MCP server, HTTP API — is a **surface**: thin,
replaceable, and never the place the capability is defined. @status:impl/done

## Why this is a discipline and not a preference {#why}

@fact:THE-SECOND-SURFACE-IS-WHERE-IT-HURTS The cost never appears when the first surface is written. It appears
when the second one is, because the logic is already inside the first —
so the second either reimplements it or reaches into a place that was
never meant to be an interface. @status:spec/done

@fact:TWO-SURFACES-THAT-DIVERGE-ARE-INDISTINGUISHABLE-FROM-A-BUG Two surfaces of one capability that answer differently are
indistinguishable from a bug, and nobody can say which one is wrong
without a third place to appeal to. That third place is the library. @status:spec/done

@fact:THE-DIVERGENCE-IS-SMALL-AND-SILENT The divergence is usually small enough to survive review — a path
separator, a rounding, a default — which is exactly why it needs a
structural answer rather than care. @status:spec/done

## Where this belongs, and where it does not {#placement}

@fact:THIS-IS-ABOUT-DELIVERY-NOT-ABOUT-CODE-STYLE This flow is about **how a capability reaches its user**. It is not a
code-style rule and it is not part of any language discipline: a project
writing no AI-native code at all still has surfaces and still pays for
letting them diverge. @status:impl/done

@fact:IT-IS-INSTALLED-BY-CHOICE It is installed by a project that wants it, like every other
cross-cutting practice in this group — git practices, addressable specs,
campaign plans, source mirrors. @status:impl/done

## The vocabulary {#vocabulary}

@fact:ONE-LIST-SO-EVERY-PROJECT-MEANS-THE-SAME-THING One list, so that two projects mean the same thing by the same word: @status:impl/done

| class | surfaces | what the class shares |
|---|---|---|
| @fact:ROW-CLASS-LIBRARY **library** @status:impl/done | — @status:impl/done | not a surface; what the others sit on. Mandatory once there is more than one surface @status:impl/done |
| @fact:ROW-CLASS-LOCAL **local, synchronous** @status:impl/done | CLI · TUI · GUI @status:impl/done | rendering one dataset into different views for a human present at the machine @status:impl/done |
| @fact:ROW-CLASS-AGENT **agent-facing** @status:impl/done | MCP · LSP · IDE extension @status:impl/done | tool description, schema, and what an agent can discover without being told @status:impl/done |
| @fact:ROW-CLASS-NETWORKED **networked** @status:impl/done | REST · GraphQL · Queue @status:impl/done | contract versioning and compatibility with clients you do not deploy @status:impl/done |

@fact:THE-CLASS-IS-THE-USEFUL-UNIT The class, not the individual surface, is the useful unit: it decides
what repeats and therefore what belongs in the library versus what each
surface legitimately does for itself. @status:impl/done

## What "thin" means, per class {#thinness}

@fact:THIN-IS-DEFINED-PER-CLASS-NOT-GLOBALLY "Thin" cannot be one rule, because the classes fail differently: @status:impl/done

- @fact:THIN-LOCAL **local** — the surface may own presentation entirely: column
  widths, colour, paging, progress. It may not own a decision. If removing
  the surface would lose a rule about *what is true*, the rule is in the
  wrong place. @status:impl/done
- @fact:THIN-AGENT **agent-facing** — the surface owns its description and schema, and
  those are part of the surface, not of the library: an MCP tool's prose is
  written for a reader the library never sees. It owns nothing else. @status:impl/done
- @fact:THIN-NETWORKED **networked** — the surface owns its wire contract and its version
  policy. It may translate; it may not compute. @status:impl/done

@fact:THE-TEST-IS-DELETION-OF-THE-SURFACE The test that settles arguments: **delete the surface on paper.** If
anything but presentation and transport is lost, the split is wrong. @status:impl/done

## Declaring a floor {#declaration}

@fact:A-PROJECT-DECLARES-WHICH-SURFACES-IT-OWES A project declares which surfaces it owes. The declaration is the
whole point: without it, "should this have an MCP tool?" is answered per
capability, by whoever is writing it, differently each time. @status:impl/done

@fact:AN-UNDECLARED-SURFACE-IS-NOT-A-DEBT **An undeclared surface is not a debt.** A project with no LSP has
not failed to build one; it has not chosen to. This is what keeps the
vocabulary from reading as a checklist of things everyone owes. @status:impl/done

@fact:A-NEW-CAPABILITY-IS-BORN-WITH-THE-DECLARED-SET A new capability ships with every declared surface, or with a
recorded reason why one sufficed — recorded where the capability is
specified, not in a plan. @status:impl/done

## The coverage table is derived {#derived}

@fact:A-HAND-KEPT-COVERAGE-TABLE-ROTS A table listing which capability has which surface must not be
hand-maintained. It is a restated fact, and a restated fact diverges from
what it restates — reliably, and without announcing it. @status:impl/done

@fact:THE-VOCABULARY-IS-MACHINE-READABLE-SO-THE-TABLE-CAN-BE-A-QUERY Therefore the vocabulary is machine-readable, and the table is a
**query** over what the project already knows: its command surface, its
registered agent tools, its routes. Deriving it is a separate build; being
able to derive it is a property of this vocabulary. @status:spec/plan

## Never {#never}

- @fact:NEVER-DEFINE-A-CAPABILITY-INSIDE-A-SURFACE Never define a capability inside a surface. @status:impl/done
- @fact:NEVER-TREAT-ONE-SURFACE-AS-THE-REFERENCE Never treat one surface as the reference the others are ported
  from — the library is the reference, or there is none. @status:impl/done
- @fact:NEVER-LET-TWO-SURFACES-ANSWER-DIFFERENTLY Never let two surfaces answer one question differently; when they
  do, the fix belongs in the library, not in whichever surface was
  reported. @status:impl/done
- @fact:NEVER-HAND-MAINTAIN-THE-TABLE Never hand-maintain the coverage table. @status:impl/done
- @fact:NEVER-READ-THE-VOCABULARY-AS-A-CHECKLIST Never read the vocabulary as a list of surfaces every project owes.
  It is a naming scheme; the floor is the project's own declaration. @status:impl/done
