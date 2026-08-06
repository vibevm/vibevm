# HARVEST — <feature, reader-facing name> {#root}

<status stage="spec" state="done" comment="B2 2026-07-24: template in force since Phase A (campaign writes cards from it); fact grain from birth"/>

- @fact:CARD-ONE-PROMISE One card = one promise worth telling a reader about. @status:spec/done
- @fact:CARD-WRITTEN-HOT Written in the minute the knowledge is hot (verification or fix just
  done); polished later, in the documentation stage — never now. @status:spec/done

- @fact:FIELD-UNIT **Unit:** `spec://…#anchor` @status:spec/done
- @fact:FIELD-AUDIENCE **Audience:** user | author (CSV allowed) @status:spec/done
- @fact:FIELD-PROMISE **Promise (reader's register, one sentence):** what the reader can do,
  not what the system must. E.g. "You can pin a dependency to a local
  checkout with one manifest line." @status:spec/done
- @fact:FIELD-PROVEN-EXAMPLE @status:spec/done **Proven example:** command + real output, captured from the verification
  run (this is the doc fixture — it was actually executed):

  ```
  $ vibe <…>
  <real output>
  ```

- @fact:FIELD-GOTCHAS **Gotchas / edge cases:** what surprised us; what the reader will trip on. @status:spec/done
- @fact:FIELD-STABILITY **Stability:** freeze-candidate | still-moving (`still-moving` cards wait;
  a chapter is written only from stable cards). @status:spec/done
- @fact:FIELD-VERDICT-REF **Source verdict ref:** <cache/baseline pointer> @status:spec/done
