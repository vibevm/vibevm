# Flow: Omnichannel {#root}

<status stage="impl" state="done"/>

@fact:A-CAPABILITY-LIVES-IN-A-LIBRARY-NOT-IN-A-SURFACE In this project a capability lives in a **library**. The command
line, the TUI, the MCP server, the HTTP API are **surfaces** over it —
thin, interchangeable, and none of them the base. @status:impl/done

@fact:NAILING-LOGIC-TO-ONE-SURFACE-IS-THE-DEFECT Nailing the logic to
whichever surface was written first is the defect this flow exists to
prevent: the second surface then reimplements it, and the two answer the
same question differently. @status:impl/done

## The surface vocabulary {#vocabulary}

@fact:SURFACES-ARE-NAMED-FROM-ONE-LIST Surfaces are named from one list, so that every project means the
same thing by them: @status:impl/done

```
Library  ─── not a surface, but what every surface sits on
             (mandatory once there is more than one surface)
    │
    ├── local, synchronous:   CLI · TUI · GUI
    ├── agent-facing:         MCP · LSP · IDE extension
    └── networked:            REST · GraphQL · Queue
```

@fact:THE-CLASS-DECIDES-WHAT-REPEATS The grouping earns its place because the class decides **what
repeats**. Local surfaces share the problem of rendering one dataset into
different views; agent-facing ones share tool description and schema;
networked ones share contract versioning and compatibility. "Logic in a
library" is one law for all three — what counts as a *thin* surface differs
per class. @status:impl/done

## What a project owes {#obligations}

- @fact:A-PROJECT-DECLARES-ITS-OWN-SET **A project declares its own set** of surfaces. A surface that is
  not declared is not a debt: an undeclared LSP is a choice, not a gap. @status:impl/done
- @fact:A-NEW-CAPABILITY-IS-BORN-WITH-EVERY-DECLARED-SURFACE **A new capability is born with all declared surfaces**, or with a
  recorded reason why one sufficed. The reason is the artifact; "we will
  add it later" is not one. @status:impl/done
- @fact:THE-SURFACE-TABLE-IS-DERIVED-NEVER-HAND-MAINTAINED **The table of which capability has which surface is derived, never
  hand-maintained.** A hand-kept table rots exactly like every other
  restated fact — so the vocabulary is machine-readable, and the table is
  a query. @status:spec/plan

## Never {#never}

- @fact:NEVER-PUT-THE-LOGIC-IN-THE-SURFACE Never put a capability's logic in a surface. The surface parses
  input, calls the library, renders output. @status:impl/done
- @fact:NEVER-LET-ONE-SURFACE-BECOME-THE-REFERENCE-IMPLEMENTATION Never let one surface become the reference implementation that
  others are ported from — that is the same defect wearing a plan. @status:impl/done
- @fact:NEVER-HAND-MAINTAIN-THE-COVERAGE-TABLE Never hand-maintain the surface-coverage table; derive it. @status:impl/done

@fact:full-protocol-pointer Full protocol, including the declaration form and the thinness test
per class:
@spec://org.vibevm.world/omnichannel/flows/omnichannel/OMNICHANNEL-PROTOCOL#root. @status:impl/done
