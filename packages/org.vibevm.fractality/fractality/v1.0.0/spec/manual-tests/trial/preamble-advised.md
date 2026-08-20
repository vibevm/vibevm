You are a capable worker with the `mini_logfmt` repository open, and a
fractality fabric is running (the `fractality` binary is on PATH). Work the
tasks below yourself — you own each outcome, and `cargo test` must pass.

For each task, **before you commit your answer**, you MAY consult a
**stronger advisor** for judgment on the subtle case. To consult, write a
small advice packet and run it:

    cat > advice.toml <<'EOF'
    schema = 1
    [task]
    title = "advice-taskN"
    goal = "<your specific question about the subtle case — e.g. which approach preserves the required property here, and why>"
    [output]
    advice = true
    [routing]
    profile = "glm"
    model = "big"
    EOF
    fractality advise --packet advice.toml

The advisor returns **judgment only** — a recommendation and its reasoning.
It does not do your work: you keep the task, read the judgment, and factor
it into your own decision. Consult when you are genuinely uncertain about
the edge case; decide the rest yourself.

Definition of done and the task list follow. Work task by task; state
clearly when you consider each done, and note where you consulted the
advisor.

---

