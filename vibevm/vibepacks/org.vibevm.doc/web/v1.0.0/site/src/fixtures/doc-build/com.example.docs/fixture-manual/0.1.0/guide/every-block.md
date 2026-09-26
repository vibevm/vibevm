# Every block once {#root}

@status:impl/done @audience:user,dev

[p01] @fact:LEAD This page uses every block of the documentation genre **once**, so the island backend has a subject with no gaps. @status:impl/done

[p02] Inline conventions ride inside a unit: `code`, **strong**, *emphasis* and [a link](/doc/com.example.docs/fixture-manual/latest/guide/every-block/).

## Prose blocks {#prose}

- [p03] A plain item.
- @fact:ANCHORED-ITEM An item that carries a fact. @status:impl/done

1. [p04] First.
2. Second.

[p05]
| Verb | What it does |
| --- | --- |
| `list` | lists what is installed |

> [p06] A quotation from nowhere in particular.

[p07]
```rust
let answer = 42;
```

[p08]
```
a bare fence, no language claimed
```

> **Warning**
> [p09] A call-out the reader should not skip.

[p10] ![Two boxes and an arrow](media/diagram.svg)

The caption, which is a unit like any other.

## Blocks that are checked {#verifiable}

> [p11] A package **MUST** declare its `kind`, and the kind decides which shape a build reads it as.
>
> <spec://com.example/subject/common/PROP-001#A-RULE>

> [p12] <spec://com.example/subject/common/PROP-001#UNRESOLVED>

[p13]
```text
Usage: vibe list [OPTIONS]

```

[p14]
```sh
vibe --version
```

```output
vibe 1.0.0
```

[p15]
```sh
vibe nope
```

```output
```

```stderr
error: unrecognized subcommand `nope`
```

[p16]
```sh
vibe --version
```

```output
vibe 1.0.0
```

[p17]
```prompt
Install vibe on this machine and show me the version.
```

- needs: a terminal and a network connection

outcome: `vibe --version` prints a version number

- assert: `vibe --version`

[p18]
```prompt
Ask the agent for anything at all.
```

- assert: none

## Blocks that vary {#varying}

### os:windows

[p19] On Windows the path separator is a backslash.

### os:linux

[p20] On Linux it is a slash.

### For one agent (agent:codex) {#for-codex}

[p21] A whole section that only one agent sees.

## Footnotes {#footnotes}

Apparatus, not flow: these blocks take no numbers, because their count changes whenever a reference above them does.

