# Study note — grishahq/recursive-llm (minimal RLM) {#root}

_T2 note (boss-authored) for INVENTORY S13 — repo pin `cc7a826`,
MIT, ~2.2k LOC total (~722 core). First-pass survey delegated to
GLM-5.2 over a sandboxed copy (live-observation law); the boss
spot-checked load-bearing claims against the source verbatim
(depth guards, unenforced timeout, two-model switch) — all held.
Decisions and facts only._

## What it is {#what}

The cleanest small implementation of the anchor's Algorithm 1: a
sync `complete(query, context)` wrapper over an async loop; the LM
sees only the *size* of the context (a test pins that the body
never enters the prompt); a restricted Python REPL exposes exactly
`context`, `query`, a bound `recursive_llm` function, and `re`;
the loop ends on `FINAL(...)`/`FINAL_VAR(...)`.

## Facts that matter to us {#facts}

- **Two-tier depth guard:** hard exception (`MaxDepthError`) if a
  call *enters* past `max_depth` (default 5), but a **soft
  string-return** ("Max recursion depth (N) reached") when the LM
  itself asks for one level too many — the parent gets an
  explanatory value instead of a crash and can adapt. Depth rides
  an internal counter; the LM's only view is a `Depth: N` prompt
  line.
- **Per-level model choice is first-class:** `model` at depth 0,
  `recursive_model` (default = model) below — the root/sub
  economics as a constructor argument.
- **Safety by allowlist:** RestrictedPython compile + explicit
  builtin/module allowlist (bare `import os` refused); stdout
  captured; output truncated at 2 000 chars/step (the paper's
  "truncated stdout" made concrete).
- **Termination triangle:** iteration cap (default 30 →
  `MaxIterationsError`), depth cap, REPL errors folded back as
  `"Error: ..."` user messages so the loop self-recovers.
- **Honest gaps (their own):** the REPL `timeout` parameter is
  recorded but **not enforced**; no wall-clock bound at all — one
  hung LM call hangs the run; sub-calls are blocking (a
  thread-pool bridge exists only because sync REPL meets async
  engine, not for parallelism); child failures surface as free
  text; the child's answer is an unstructured string the parent
  must interpret.

## Decisions we take {#decisions}

1. **The soft depth-limit return is a keeper:** at the boundary,
   answer the caller with a *stated refusal* it can reason about,
   not an exception it can't. Fractality analog: a spawn request
   past a quota returns a structured `denied(reason=depth_cap)`
   packet outcome — the boss can re-plan instead of dying.
2. **Depth visibility belongs in the surface:** one `Depth: N`
   line in the child's context is cheap and changes behavior; our
   worker env already carries run ids — add depth to the packet
   env/prompt surface (FRACTALITY_DEPTH).
3. **Negative proof as a test is house-style confirmation:** their
   `test_context_not_in_prompt` is exactly our I1-style invariant
   testing (assert what must NEVER appear); reuse the pattern for
   "transcript never enters parent context" in Stage B.
4. **The unenforced-timeout hole is the field's norm, not an
   outlier** — wall-clock enforcement lives OUTSIDE the loop in
   every surveyed impl or nowhere. MC owning wall/turn budgets
   (I-quotas, kill-trees) is a genuine fabric advantage; keep
   enforcement out of the agent loop.
5. **String-typed child results are the anti-pattern to avoid:**
   the parent-LM-interprets-free-text contract is where silent
   errors breed; our packet results stay structured (status +
   files), per the anchor note's decision 5.

**Non-adoptions:** RestrictedPython-style in-process sandboxing
(our isolation is process/env-level, I1); FINAL/FINAL_VAR string
sentinels.
