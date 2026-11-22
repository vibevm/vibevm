# Worker prefix: measured cache reuse (SWP-1)

2026-11-22, Claude Code subagents (model Haiku), five same-type workers per
wave, one wave per case. Each worker's prompt is a 1500-line synthetic prefix
(about 76 thousand tokens with the harness context after the shared system
prompt) followed by a one-line task. Numbers are the first request's usage as
recorded in each worker's transcript.

Commands run (Claude Code workflow script, one worker per `agent()` call):

1. Shared: worker 1 alone with `PREFIX + TASK`, then workers 2-5 in parallel
   with the same bytes.
2. Varied: five workers in parallel with `"Worker k of wave B, assembled at
   step <k*7919>.\n" + PREFIX + TASK`, the same content with a small
   per-worker difference at the start, as a hand-assembled prompt would have.

| Case | Worker | cache_creation_input_tokens | cache_read_input_tokens |
|---|---|---|---|
| shared | 1 | 75807 | 19784 |
| shared | 2 | 0 | 95591 |
| shared | 3 | 0 | 95591 |
| shared | 4 | 0 | 95591 |
| shared | 5 | 0 | 95591 |
| varied | 1 | 75829 | 19784 |
| varied | 2 | 75829 | 19784 |
| varied | 3 | 75829 | 19784 |
| varied | 4 | 75829 | 19784 |
| varied | 5 | 75829 | 19784 |

Result: with byte-identical prefixes the wave writes the prefix to the cache
once and every later worker reads it (about 135 thousand input-token
equivalents for the wave at 1.25x for writes and 0.1x for reads), while any
per-worker difference before the prefix makes every worker write it again
(about 484 thousand), so one shared prefix per wave is about 3.6 times cheaper.

Method note: the harness's own subagents were used instead of launching
external `claude` processes, because the local rules forbid external launches
with host credentials. The prefix sat at the start of each worker's first
message rather than in its system prompt; caching is by prefix over the system
prompt and messages, so the mechanism measured is the one a launcher-appended
prefix relies on. A preceding run without the large prefix showed the same
pattern on the harness context alone (39602 tokens read by workers 2-5 versus
19840 rewritten per varied worker).
