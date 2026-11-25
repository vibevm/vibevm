# Extension: working rules with multi-user planning

This page applies only when the project also has the package
`org.vibevm.world/multi-user-planning` installed. It changes one thing: where a
lead's completion list lives. Everything else in the working rules stays as
written, and the rest of `lead-bounded.md` still applies to a bounded lead.

## The list is the plan

The completion list is not a separate file. It is the open nodes under the goal
node of the plan in the session's steward context: the `plan.toml` of that
context, with the goal node named by `goal_node` in the `settings.toml` beside
it. The "Completion list:" line of "Current mode" in the local binding points to
this page; the context is the one the planning package resolves for the session.
No second list is kept beside the plan.

Each leaf node under the goal node is one entry. Every new entry carries the
fields `order`, `kind`, `depends_on` and `mandates` shown below, because the
planning package refuses a node without them:

```
[[node]]
id = "EXPORT-REFUSE-EMPTY"
parent = "<goal node>"
order = 10
kind = "atom"
title = "the `export` command refuses an empty input with a typed error"
state = "planned"
depends_on = []
mandates = []
acceptance = ["cargo test --test export refuse_empty"]
write = ["src/export/**", "tests/export/**"]
evidence = []
note = "packets 0/6, failed checks 0/2"
```

| Entry line | Plan node field |
|---|---|
| `Outcome` | `title` |
| `Check` | the first line of `acceptance` |
| `Write` | `write` |
| `State` | `state`, with the counters of the entry in `note` |
| `Note` | `note`, after the counters |

States map as follows. `planned`, `ready`, `active` and `candidate` are open.
`accepted`, `dropped`, `deferred` and `superseded` are closed. `blocked` means
the entry waits for the owner and the lead has moved on.

## Reading and editing

- Use the generated goal file (`GOAL.md` beside the plan) to see which entries
  are open and in what order. For the entry you work on, read and edit its own
  node in the plan. Do not read or touch nodes outside the goal node.
- When an entry is closed as `accepted`, its `evidence` holds one commit hash and
  nothing else.
- After every edit of the plan, refresh the goal file with the planning
  package's `steward-goal` skill and the tree view with its `steward-tree` skill,
  as each skill's own page describes, so that the goal file and the owner's tree
  view show the same thing as the plan.
- Only the owner decides changes to the list; the session makes the edit in the
  plan on the owner's word, exactly as `lead-bounded.md` rule 3 says for a file.
