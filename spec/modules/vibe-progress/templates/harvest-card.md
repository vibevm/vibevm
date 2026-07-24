# HARVEST — <feature, reader-facing name> {#root}

<status stage="spec" state="done" comment="B2 2026-07-24: template in force since Phase A (campaign writes cards from it); fact grain from birth"/>

- ##CARD-ONE-PROMISE One card = one promise worth telling a reader about. @spec/done
- ##CARD-WRITTEN-HOT Written in the minute the knowledge is hot (verification or fix just
  done); polished later, in the documentation stage — never now. @spec/done

- ##FIELD-UNIT **Unit:** `spec://…#anchor` @spec/done
- ##FIELD-AUDIENCE **Audience:** user | author (CSV allowed) @spec/done
- ##FIELD-PROMISE **Promise (reader's register, one sentence):** what the reader can do,
  not what the system must. E.g. "You can pin a dependency to a local
  checkout with one manifest line." @spec/done
- ##FIELD-PROVEN-EXAMPLE @spec/done **Proven example:** command + real output, captured from the verification
  run (this is the doc fixture — it was actually executed):

  ```
  $ vibe <…>
  <real output>
  ```

- ##FIELD-GOTCHAS **Gotchas / edge cases:** what surprised us; what the reader will trip on. @spec/done
- ##FIELD-STABILITY **Stability:** freeze-candidate | still-moving (`still-moving` cards wait;
  a chapter is written only from stable cards). @spec/done
- ##FIELD-VERDICT-REF **Source verdict ref:** <cache/baseline pointer> @spec/done
