# P9 — verbatim answers

Each session's `result` text and `usage` counters, copied unedited from its `--output-format json` record (Claude Code 2.1.322, `claude-opus-5-5`). Verdicts are in `RESULTS.md`.

## C1 step 1 — live-1

turns 1 · cache_creation_input_tokens 13221 · cache_read_input_tokens 11275 · input_tokens 2

````text
The last real `vibe:lane-end` line in my context:

```
<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->
```

The CLAUDE.md boot block, which comes later in my context, has a line `vibe:lane-end org.live/sample <path> <id>`. That's only a template showing the marker format, so it isn't an actual lane-end marker.

**Canary words in my context:**
1. CANARY-LANE-HEAD-OBSIDIAN
2. CANARY-INDEX-CORE-BASALT
3. CANARY-INDEX-USER-GNEISS
4. CANARY-LANE-TAIL-PUMICE

**Files I've read with a tool this session:** none. I haven't called any tool. All of the content above was already in my context: the delivered boot lane and the CLAUDE.md shown in a system reminder.
````

## C2 step 2 — live-1 repeat

turns 1 · cache_creation_input_tokens 7418 · cache_read_input_tokens 15381 · input_tokens 2

````text
**Last `vibe:lane-end` line:** the last real lane marker in my context is:

```
<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->
```

A later line in CLAUDE.md, `    vibe:lane-end org.live/sample <path> <id>`, also matches. It is a placeholder template showing the format, not a delivered lane marker.

**Canary words in my context:**
1. `CANARY-LANE-HEAD-OBSIDIAN`
2. `CANARY-INDEX-CORE-BASALT`
3. `CANARY-INDEX-USER-GNEISS`
4. `CANARY-LANE-TAIL-PUMICE`

**Files I read with a tool this session:** none. I haven't made any tool calls. Everything above came from content that was already in my context: the system prompt lane blocks and the CLAUDE.md you provided.
````

## C3 small — live-1b

turns 1 · cache_creation_input_tokens 9576 · cache_read_input_tokens 13338 · input_tokens 2

````text
**Last `vibe:lane-end` line:** The last lane-end marker from an actual delivered file is:

```
<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->
```

A later line also contains `vibe:lane-end`, but it's only a placeholder example in the CLAUDE.md boot instructions:

```
    vibe:lane-end org.live/sample <path> <id>
```

**Canary words in my context:**
1. `CANARY-LANE-HEAD-OBSIDIAN`
2. `CANARY-INDEX-CORE-BASALT`
3. `CANARY-INDEX-USER-GNEISS`
4. `CANARY-LANE-TAIL-PUMICE`

**Files I've read with a tool this session:** None. I haven't called any tools. The lane contents and CLAUDE.md were already in my context when the session started. All three lane files (STATIC.md, 00-core.md, 90-user.md) arrived with matching begin and end lines, and STATIC.md has the expected id `df1cb2d5aadc`, so the boot rules didn't require reading anything from disk.
````

## C1 step 3 — live-3, 202 KB

turns 1 · cache_creation_input_tokens 73987 · cache_read_input_tokens 11275 · input_tokens 2

````text
**Last `vibe:lane-end` line in my context:**

```
<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->
```

**Canary words in my context:**
- `CANARY-LANE-HEAD-OBSIDIAN` (in `vibevm/vibespecs/boot/STATIC.md`)
- `CANARY-INDEX-CORE-BASALT` (in `vibevm/vibespecs/boot/00-core.md`)
- `CANARY-INDEX-USER-GNEISS` (in `vibevm/vibespecs/boot/90-user.md`)
- `CANARY-LANE-TAIL-PUMICE` (in `vibevm/vibespecs/boot/90-user.md`, the lane's tail canary)

**Files I have read with a tool this session:** none. I haven't made any tool calls. Everything above came from what was already in my context.
````

## C2 at size — live-3 repeat

turns 1 · cache_creation_input_tokens 0 · cache_read_input_tokens 85262 · input_tokens 2

````text
**Last `vibe:lane-end` line in my context:**

```
<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->
```

**Canary words in my context:**
- `CANARY-LANE-HEAD-OBSIDIAN` (in `STATIC.md`)
- `CANARY-INDEX-CORE-BASALT` (in `00-core.md`)
- `CANARY-INDEX-USER-GNEISS` (in `90-user.md`)
- `CANARY-LANE-TAIL-PUMICE` (in `90-user.md`, the lane's tail canary)

**Files I have read with a tool this session:** none. I haven't made any tool calls. The lane files and `CLAUDE.md` were already in my context when the session started, so I didn't open them.
````

## C3 at size — live-3b

turns 1 · cache_creation_input_tokens 9574 · cache_read_input_tokens 75803 · input_tokens 2

````text
**Last `vibe:lane-end` line in my context:**

```
<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/90-user.md 6494d071a30d -->
```

**Canary words in my context:**

1. `CANARY-LANE-HEAD-OBSIDIAN`, in `STATIC.md`
2. `CANARY-INDEX-CORE-BASALT`, in `00-core.md`
3. `CANARY-INDEX-USER-GNEISS`, in `90-user.md`
4. `CANARY-LANE-TAIL-PUMICE`, in `90-user.md`, the lane's tail canary

**Files I have read with a tool this session:** none. I haven't made a single tool call. Everything above came from what was already in my context.
````

## C1 this repository

turns 1 · cache_creation_input_tokens 180811 · cache_read_input_tokens 11275 · input_tokens 2

````text
**Last `vibe:lane-end` line.** The last real delivered lane-end in my context is the omnichannel one:

```
<!-- vibe:lane-end org.vibevm.core/vibevm vibevm/vibedeps/org.vibevm.world.omnichannel/1.0.0/vibevm/vibespecs/boot/68-flow-omnichannel.xml 49d8aae57569 -->
```

One line appears after it. The `CLAUDE.md` boot block includes a template line, `vibe:lane-end org.vibevm.core/vibevm <path> <id>`, that shows the marker format. It is not a delivered lane marker.

**`CANARY-…` words:** none. I can't find any word of that form anywhere in my context.

**Files I have read with a tool this session:** none. I have made no tool calls at all. Everything I know came already loaded into my context: the system prompt with its boot lanes, the `CLAUDE.md` contents and the auto-memory `MEMORY.md` index.

**Did I try to resolve any `@spec://` token as a file path?** No. I tried none.
````

## C2 this repository, repeat

turns 1 · cache_creation_input_tokens 8109 · cache_read_input_tokens 183981 · input_tokens 2

````text
**1. The last `vibe:lane-end` line in my context**

The last real lane-end marker is:

```
<!-- vibe:lane-end org.vibevm.core/vibevm vibevm/vibedeps/org.vibevm.world.omnichannel/1.0.0/vibevm/vibespecs/boot/68-flow-omnichannel.xml 49d8aae57569 -->
```

One later line also contains the string. It is the template example in the `CLAUDE.md` block, not a real marker:

```
    vibe:lane-end org.vibevm.core/vibevm <path> <id>
```

**2. `CANARY-…` words**

There are none in my context.

**3. Files I read with a tool this session**

None. I made no tool calls at all. Everything above comes from the system prompt and the system reminders: the boot lanes and the `CLAUDE.md` and `MEMORY.md` contents the harness injected.

**4. Did I try to open any `@spec://` token as a file path?**

No. I didn't try to resolve any of them.
````

## C4 classic, plain claude2

turns 5 · cache_creation_input_tokens 14146 · cache_read_input_tokens 58153 · input_tokens 6

````text
Boot is complete. None of the lane files were in my context beforehand, so I read each one from disk: `STATIC.md` first, then the entries `INDEX.md` lists, in order. No INDEX entry had a `when` condition. Since `STATIC.md` came in by a direct read and not as a delivered copy, there were no lane lines and no id to check against `df1cb2d5aadc`.

**Canary words in context:**
1. `CANARY-LANE-HEAD-OBSIDIAN` (from `STATIC.md`)
2. `CANARY-INDEX-CORE-BASALT` (from `00-core.md`)
3. `CANARY-INDEX-USER-GNEISS` (from `90-user.md`)
4. `CANARY-LANE-TAIL-PUMICE` (from `90-user.md`)

**Files read with a tool this session, in order:**
1. `vibevm/vibespecs/boot/STATIC.md` (once)
2. `vibevm/vibespecs/boot/INDEX.md` (once)
3. `vibevm/vibespecs/boot/00-core.md` (once)
4. `vibevm/vibespecs/boot/90-user.md` (once)

I didn't open `CLAUDE.md` with a tool; it was already in my context.
````

## C4 classic, vibe agent (after 6323e8a36)

turns 5 · cache_creation_input_tokens 3635 · cache_read_input_tokens 68782 · input_tokens 6

````text
I've read all four boot files. None of them had been delivered in context beforehand (no lane lines), so I read each from disk: STATIC.md first, then the files INDEX lists, in order. Neither INDEX entry has a `when` condition.

**Canary words in context:**
1. `CANARY-LANE-HEAD-OBSIDIAN` (STATIC.md)
2. `CANARY-INDEX-CORE-BASALT` (00-core.md)
3. `CANARY-INDEX-USER-GNEISS` (90-user.md)
4. `CANARY-LANE-TAIL-PUMICE` (90-user.md)

**Files read with a tool this session, in order:**
1. `vibevm/vibespecs/boot/STATIC.md`, 1 time
2. `vibevm/vibespecs/boot/INDEX.md`, 1 time
3. `vibevm/vibespecs/boot/00-core.md`, 1 time
4. `vibevm/vibespecs/boot/90-user.md`, 1 time

I didn't read `CLAUDE.md` with a tool; the system put it into context automatically.
````

## C5 subagent

turns 2 · cache_creation_input_tokens 11882 · cache_read_input_tokens 36151 · input_tokens 4

````text
Here is the subagent's answer, quoted word for word:

> I see no word of the form CANARY-… anywhere in my context.
>
> I also see no line beginning with `<!-- vibe:lane-begin`. The only `vibe:lane-begin` text is the indented format example inside the CLAUDE.md block (`vibe:lane-begin org.live/sample <path> <id>`). It has no `<!--` prefix and is not a real lane marker. The only `<!--` line in context is the "Generated by vibe — do not edit…" comment.
````

## N1 nested

turns 2 · cache_creation_input_tokens 12707 · cache_read_input_tokens 36108 · input_tokens 4

````text
I opened `nested/AGENTS.md` and nothing else, as you asked. Its boot block belongs to the nested project `org.live/nested`, not this session's root, so I ignored that block and read none of its files. When I opened that folder, Claude Code also loaded `nested/CLAUDE.md` into my context automatically, but I didn't read it with a tool. Its boot block gets ignored for the same reason.

**Canary words now in my context:**
1. CANARY-LANE-HEAD-OBSIDIAN (static lane, `org.live/carrier` block)
2. CANARY-INDEX-CORE-BASALT (`00-core.md`)
3. CANARY-INDEX-USER-GNEISS (`90-user.md`)
4. CANARY-LANE-TAIL-PUMICE (lane tail, `90-user.md`)
5. CANARY-NESTED-TUFF (`nested/AGENTS.md`)

The first four came with the boot lane, which arrived already in my context, so I didn't open those files.

**Files I read with a tool this session:**
- `/fast/tmp/live-1/nested/AGENTS.md`

That is the only one.
````

## C6 repeated flag (plain claude2)

turns 1 · cache_creation_input_tokens 9962 · cache_read_input_tokens 11275 · input_tokens 2

````text
There is one word of the form CANARY-… in my context:

- **CANARY-C6-SECOND-ONYX**: from the system prompt, in the line "Second appended file. Its canary is CANARY-C6-SECOND-ONYX."

The line calls it the "second" appended file, but no canary from a first file appears anywhere in my context. I read no files.
````

## C7 resume after a band-U edit

turns 1 · cache_creation_input_tokens 709 · cache_read_input_tokens 22799 · input_tokens 2

````text
**`vibe:lane-end` lines for 00-core.md:** there's one:

```
<!-- vibe:lane-end org.live/sample vibevm/vibespecs/boot/00-core.md cc8a3fa2cf16 -->
```

Its id is `cc8a3fa2cf16`, the same as its matching `vibe:lane-begin` line.

**Canary words in my context:**
1. `CANARY-LANE-HEAD-OBSIDIAN`
2. `CANARY-INDEX-CORE-BASALT`
3. `CANARY-INDEX-USER-GNEISS`
4. `CANARY-LANE-TAIL-PUMICE`

**Is the delivered lane out of date?** I see no sign that it is.
- CLAUDE.md says STATIC.md should have the id `df1cb2d5aadc`. Both of its lane lines carry that id, so it's current.
- STATIC.md, 00-core.md and 90-user.md each have a complete begin/end pair with matching ids, so none of them was cut short.
- CLAUDE.md only gives an expected id for STATIC.md. I can't check the ids of 00-core.md and 90-user.md against anything without reading files.
- I also can't see INDEX.md itself, so I can't confirm those two files are everything it lists. One small sign the delivery is complete: the tail canary `CANARY-LANE-TAIL-PUMICE` is present, and 90-user.md says it's the last file the lane delivers.

I still haven't read any files with a tool this session.
````
