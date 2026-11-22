# ADV-SDD-CHANGE-WRITE-6

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe change answer <id> q-1 "<text>"` writes refinements/q-1.md, appends a normative input refinement row to change.toml so the change's intent identity changes, and `vibe change questions` then lists q-1 as `answered`; an unknown question id is refused.

**Write.** `crates/vibe-cli/src/cli/change.rs`, `crates/vibe-cli/src/cli/change/write.rs`, `crates/vibe-cli/src/commands/change/mod.rs`, `crates/vibe-cli/src/commands/change/answer.rs`, `crates/vibe-cli/tests/cli_change_questions.rs`, `vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml`

**Check.** `cargo test -p vibe-cli --test cli_change_questions answer_ 2>&1 | grep -F "test result: ok. 2 passed"`

**Test.** crates/vibe-cli/tests/cli_change_questions.rs: `answer_declares_refinement_and_changes_intent`, `answer_refuses_unknown_question`

**Read first.**
- crates/vibe-cli/src/commands/change/ask.rs and crates/vibe-cli/src/commands/change/questions.rs (QuestionRow, question_rows, answer_path)
- crates/vibe-cli/src/commands/change/home.rs (change_home) and crates/vibe-cli/src/commands/change/new.rs (Project::dir + publish_new_in pattern)
- crates/vibe-requirements/src/change/identity.rs (SavedChange::identity_analysis, ChangeIdentityAnalysis.intent)
- crates/vibe-safefs/src/file/create_new.rs (Project::publish_new_in)
- crates/vibe-cli/tests/cli_change_questions.rs and crates/vibe-cli/tests/change_support/mod.rs

**Steps.**
1. cli/change/write.rs: add `#[derive(Debug, clap::Args)] pub struct ChangeAnswerArgs { pub id: String, pub question: String, pub text: String, #[arg(long, default_value = ".")] pub path: PathBuf }` (field docs: `The change id.`, `The question id, for example q-1.`, `The answer text.`, `Project root; defaults to the current directory.`). In cli/change.rs add to ChangeSubcommand `Answer(ChangeAnswerArgs)` with doc `/// Answer a question as a normative refinement document.` In commands/change/mod.rs add `mod answer;` and the arm `ChangeSubcommand::Answer(args) => answer::run(ctx, args),`.
2. Create answer.rs (line 1 `//! \`vibe change answer\`: an answer is a normative refinement document.`, line 2 `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-QUESTIONS");`). `#[derive(serde::Serialize)] struct AnswerReport { change_id: String, question_id: String, path: String }`. `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-QUESTIONS")] pub(super) fn run(ctx: &crate::output::Context, args: crate::cli::ChangeAnswerArgs) -> anyhow::Result<()>`: change_home, read_saved_change(&home.selected_root, ".", None, &args.id)?, question_rows. Let qid = args.question. If no row has `id == qid`, bail with `violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-QUESTIONS: unknown question {qid}; fix surface: an id listed by vibe change questions`. If `saved.manifest.documents()` has a row with `path == answer_path(&qid)`, bail with `violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-QUESTIONS: question {qid} is already answered; fix surface: edit the refinement document`. Both checks run before any write.
3. Write the refinement first: `let project = vibe_safefs::Project::open(&home.selected_root)?;` `let dir_text = vibe_core::machine_json_path(saved.manifest.directory());` `let mut parts: Vec<&str> = dir_text.split('/').collect(); parts.push("refinements");` `let dir = project.dir(&parts, true)?;` body = `format!("# Answer to {qid}\n\nQuestion: {}\n\n{}\n", row.text, args.text)`; `project.publish_new_in(&dir, &format!("{qid}.md"), body.as_bytes()).map_err(vibe_safefs::PublishError::into_report)?;`
4. Then append the declaration: old = String::from_utf8 of saved.manifest_bytes with a final '\n' ensured; new = old + `\n[[documents]]\nrole = "refinement"\npath = "refinements/<qid>.md"\nnormative = true\ninput = true\n`; validate with `ChangeManifest::parse_str(&new, &home.context, saved.manifest.directory())?`; write with `project.write_atomic(&vibe_core::machine_json_path(&saved.manifest.directory().join("change.toml")), new.as_bytes()).map_err(vibe_safefs::PublishError::into_report)?`. Output: JSON `AnswerReport { change_id, question_id, path: answer_path(&qid) }`; text `answered <qid> in refinements/<qid>.md`.
5. Add to tests/cli_change_questions.rs: `answer_declares_refinement_and_changes_intent`: installed(), new_change(), `w.vibe(&["change", "ask", ID, "Should retries apply to POST?"])` succeeds; `let before = vibe_requirements::change::read_saved_change(w.project.path(), ".", None, ID).unwrap().identity_analysis().unwrap().intent;` and save the change.toml bytes; run `w.vibe(&["change", "answer", ID, "q-1", "No, idempotent verbs only"])` and assert success; assert `refinements/q-1.md` in `w.change_dir(ID)` contains `No, idempotent verbs only`; change.toml starts with the saved bytes; the re-read intent != before; and `--json change questions` gives `questions[0].state == "answered"`. `answer_refuses_unknown_question`: installed(), new_change(), `let before = w.snapshot(ID);` `w.vibe(&["change", "answer", ID, "q-9", "x"])` fails, its stderr contains `unknown question q-9`, and `w.snapshot(ID) == before`.
6. Apply the spec_edit to PROP-071, then run the check.

**Decisions already made.**
- This entry runs after ADV-SDD-CHANGE-WRITE-5, one after the other: it shares cli/change.rs, cli/change/write.rs, commands/change/mod.rs, tests/cli_change_questions.rs and PROP-071 with it.
- An answer is a refinement document declared with normative = true and input = true (the grammar requires normative rows to be input rows). No answer field is written into the question table; the answered state is derived from the documents row.
- The refinement file is written before change.toml. A crash in between leaves an undeclared file, never a declaration of a missing file. The file is created exclusively and is never overwritten.
- The answer text is a positional argument.
- answer.rs is a new file; commands/change/mod.rs and cli/change.rs grow by 2 lines each. Run `wc -l` on each; none may exceed 600.
- Clippy -D warnings applies. Do not add #[allow].

**Not in this entry.** Not in this entry: --file input, editing or replacing an existing answer, removing questions, reordering change.toml, and an MCP tool.

**Depends on.** ADV-SDD-CHANGE-WRITE-5

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml replace
```
<CHANGE-QUESTIONS fact="true" status="impl/work">
```
with
```
<CHANGE-QUESTIONS fact="true" status="impl/done">
```

Entries that write the same file run one after another, never in parallel.
