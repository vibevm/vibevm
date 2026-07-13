# Flow: Manual Tests {#root}

This project keeps a **second test tier**: human-run markdown
walkthroughs that prove the integration surfaces the automated suite
cannot reach. The automated tier proves the logic; the manual tier
proves the world. It complements the automated suite — it never
replaces it.

## When to propose writing one {#when}

Propose a manual test — do not wait to be asked — whenever:

- **A new integration surface lands.** Real authentication, the
  per-user state directory on a real filesystem, a lockfile as a
  downstream consumer sees it, network-facing I/O — anything the
  automated tier fakes now has a real-world form that nothing proves.
- **A milestone approaches.** Before tagging, every run the index
  marks required for the shipped features must have been executed.
- **A user reports an integration bug.** Its reproduction steps
  become a manual test, so the next session can replay them exactly.

The format, authoring rules, and copy-ready skeleton live under
[`../flows/manual-tests/`](../flows/manual-tests/MANUAL-TESTS-PROTOCOL.md).

## Agent pre-runs, human signs off {#roles}

The whole point of the tier is human eyes on real output. An agent
may **pre-run** a manual test end to end and flag any step whose
result diverges from its "Expected" paragraph — that is useful
triage. But the sign-off is a human's: only a person can look at the
tool's output and say "yes, that is what I meant". Report the
pre-run; never record the pass.

## Never {#never}

- **Never let a manual test touch real user state.** Every run
  isolates its project into a scratch directory and redirects the
  tool's per-user cache into that scratch. A test that mutates the
  real per-user state is a bug in the test.
- **Never write a step without an "Expected" paragraph.** A command
  with no stated outcome cannot pass or fail; it is not a test step.
- **Never tag a milestone with the index's required runs
  unexecuted.** Green automated suite plus unrun manual tests is not
  a shippable milestone.
- **Never delete a failing manual test to make the panel green.** A
  test that caught something is working; file what it caught and fix
  the product, not the test.
