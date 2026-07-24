# HARVEST — <feature, reader-facing name> {#root}

One card = one promise worth telling a reader about. Written in the minute
the knowledge is hot (verification or fix just done); polished later, in the
documentation stage — never now.

- **Unit:** `spec://…#anchor`
- **Audience:** user | author (CSV allowed)
- **Promise (reader's register, one sentence):** what the reader can do,
  not what the system must. E.g. "You can pin a dependency to a local
  checkout with one manifest line."
- **Proven example:** command + real output, captured from the verification
  run (this is the doc fixture — it was actually executed):

  ```
  $ vibe <…>
  <real output>
  ```

- **Gotchas / edge cases:** what surprised us; what the reader will trip on.
- **Stability:** freeze-candidate | still-moving (`still-moving` cards wait;
  a chapter is written only from stable cards).
- **Source verdict ref:** <cache/baseline pointer>
