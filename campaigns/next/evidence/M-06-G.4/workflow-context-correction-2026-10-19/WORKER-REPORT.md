M-06-G.4 continuation: workflow context correction

Scope and observed workspace
Read complete campaigns/next/tasks/M-06-G.json including M-06-G.4. Initial git status --short was empty. Inspected the retained committed Windows workflow and current diff. Preserved existing materialization implementation and native fixtures. The Coordinator reports GitHub run 39210946345 rejected before jobs at job env runner.temp; that remote run was not independently inspected in this capture and supplies no native verification.

Change
Moved VIBE_G4_EVIDENCE_DIR=${{ runner.temp }}/g4-evidence from jobs.materialization-paths.env to the selected test step env. GitHub's official context availability table excludes runner from jobs.<job_id>.env and includes it in jobs.<job_id>.steps.env and steps.with:
https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#context-availability
The upload step retains its own matching runner.temp path. Existing branch/path filters, pinned actions, contents:read permission, selected Cargo invocation, output capture, actual exit propagation, and always-upload behavior are preserved. No independent specification law is replaced. This is a CI wiring correction; no additional production behavior is claimed.

Actual local validation in this capture
1. Python/PyYAML BaseLoader parsed the current workflow and committed HEAD workflow. Compared complete parsed documents after moving only the baseline evidence env key to the test step: equal. Asserted runner absent from job env, correct test env and upload paths, always() upload, exactly one selected Cargo command, test exit propagation and contents:read permission. Exit 0. This is one static validation script, not a Rust test or GitHub workflow execution.
2. git diff --check: exit 0 (run after edit and after YAML validation).
Only .github/workflows/windows-materialization-paths.yml is modified. No Rust source, dependencies, toolchain, XML or generator input changed. No Rust tests, builds, codegen/specmap, installs, Git mutations, push or CI triggers performed. No actionlint executable was available; no install attempted.

Evidence limits and next action
Native Windows code has still not been compiled or executed by this capture. Existing Linux results are attributed history, not newly observed test passes. Original acceptance slots 1 and 2 remain unresolved: long-root native positive proof, actual volume L/L+1 negative controls, exact diagnostic/new-slot pre-effect evidence, cleanup/retry and independent review require a commit-bound native run. macOS and any unsupported total-path ceiling/minimum root-shortening proof remain open. Workflow admission itself must be confirmed by GitHub after Root reviews, commits and performs the authorized mirror push. Root must inspect the exact-SHA selected Windows job log/JSON, retaining any failures and final acceptance authority.

Artifacts and progress
This report and a nonempty patch are published content only. Publication and progress acknowledgment do not establish acceptance. The source edit is complete locally; native proof and host verification verify.m06.g4.01 remain pending.
