# Generated subagent type: probe (2026-11-22)

Owner-approved probe for the subagent-type generation system. One fresh
`claude -p` process (Claude Code 2.1.330, model Haiku) in a temporary project
whose `.claude/agents/size-probe.md` held a 250,249-byte definition: frontmatter
plus a marker, the first 250,000 bytes of the composed boot lane, and a second
marker at the very end. The session started one worker of that type, waited,
then started four in parallel with the same prompt.

| Worker | cache_creation_input_tokens | cache_read_input_tokens |
|---|---|---|
| 1 | 147734 | 0 |
| 2 | 8361 | 141270 |
| 3 | 0 | 149631 |
| 4 | 0 | 149631 |
| 5 | 0 | 149631 |

Findings:

- A 250 KB definition is accepted whole: every worker reported both the first
  and the last marker.
- Same-type workers share one cache: after the first worker, the others read
  the definition from the cache (about 95-100%).
- Definitions are read when a session starts. A type written while a session
  runs is not available to it ("Agent type 'size-probe' not found" in a
  running session), so a generated type takes effect at the next launch.
- Claude Code's `--agents '<json>'` flag cannot carry such a body: Linux caps a
  single argument at 128 KB (MAX_ARG_STRLEN) and Windows caps the whole command
  line at about 32 K characters. Generated types must be files.
- A `SubagentStart` hook cannot carry it either: `additionalContext` is capped
  at 10,000 characters (Claude Code hooks reference).

## Second probe (2026-11-22): size and location

A temporary git repository with a type at the root (`.claude/agents/probe-root.md`)
and a 340,129-byte type in a member directory (`member/.claude/agents/probe-member.md`,
first and last markers). Fresh `claude -p` sessions (Haiku) were started in
`member/sub/` and in `member/`.

- The 340 KB definition is accepted whole: the worker reported both markers.
- From `member/sub/`, both the member's type and the root's type were found:
  Claude Code collects `.claude/agents` from the session's directory upwards
  to the repository root. A type written into a node's `.claude/agents` is
  therefore available when a session starts in that node or below it.
