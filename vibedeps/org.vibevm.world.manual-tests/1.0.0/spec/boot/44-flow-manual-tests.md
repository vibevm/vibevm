# Flow: Manual Tests {#root}

<status stage="impl" state="done"/>

@fact:THE-PROJECT-KEEPS-A-SECOND-TEST-TIER This project keeps a **second test tier**: human-run markdown
walkthroughs that prove the integration surfaces the automated suite
cannot reach. @status:impl/done

@fact:THE-AUTOMATED-TIER-PROVES-THE-LOGIC-THE-MANUAL-TIER-PROVES-THE-WORLD The automated tier proves the logic; the manual tier
proves the world. @status:impl/done

@fact:THE-MANUAL-TIER-COMPLEMENTS-AND-NEVER-REPLACES It complements the automated suite — it never
replaces it. @status:impl/done

## When to propose writing one {#when}

@fact:propose-a-manual-test-lead Propose a manual test — do not wait to be asked — whenever: @status:impl/done

- @fact:TRIGGER-A-NEW-INTEGRATION-SURFACE-LANDS **A new integration surface lands.** Real authentication, the
  per-user state directory on a real filesystem, a lockfile as a
  downstream consumer sees it, network-facing I/O — anything the
  automated tier fakes now has a real-world form that nothing proves. @status:impl/done
- @fact:TRIGGER-A-MILESTONE-APPROACHES **A milestone approaches.** Before tagging, every run the index
  marks required for the shipped features must have been executed. @status:impl/done
- @fact:TRIGGER-A-USER-REPORTS-AN-INTEGRATION-BUG **A user reports an integration bug.** Its reproduction steps
  become a manual test, so the next session can replay them exactly. @status:impl/done

@fact:sibling-document-pointers The format, authoring rules, and copy-ready skeleton live under
@spec://org.vibevm.world/manual-tests/flows/manual-tests/MANUAL-TESTS-PROTOCOL#root. @status:impl/done

## Agent pre-runs, human signs off {#roles}

@fact:the-whole-point-is-human-eyes-on-real-output The whole point of the tier is human eyes on real output. @status:spec/done

@fact:AN-AGENT-MAY-PRE-RUN-AND-FLAG-DIVERGENCES An agent
may **pre-run** a manual test end to end and flag any step whose
result diverges from its "Expected" paragraph — that is useful
triage. @status:impl/done

@fact:THE-SIGN-OFF-IS-A-HUMANS But the sign-off is a human's: only a person can look at the
tool's output and say "yes, that is what I meant". @status:impl/done

@fact:REPORT-THE-PRE-RUN-NEVER-RECORD-THE-PASS Report the
pre-run; never record the pass. @status:impl/done

## Never {#never}

- @fact:NEVER-LET-A-MANUAL-TEST-TOUCH-REAL-USER-STATE **Never let a manual test touch real user state.** Every run
  isolates its project into a scratch directory and redirects the
  tool's per-user cache into that scratch. A test that mutates the
  real per-user state is a bug in the test. @status:impl/done
- @fact:NEVER-WRITE-A-STEP-WITHOUT-AN-EXPECTED-PARAGRAPH **Never write a step without an "Expected" paragraph.** A command
  with no stated outcome cannot pass or fail; it is not a test step. @status:impl/done
- @fact:NEVER-TAG-A-MILESTONE-WITH-REQUIRED-RUNS-UNEXECUTED **Never tag a milestone with the index's required runs
  unexecuted.** Green automated suite plus unrun manual tests is not
  a shippable milestone. @status:impl/done
- @fact:NEVER-DELETE-A-FAILING-MANUAL-TEST-TO-MAKE-THE-PANEL-GREEN **Never delete a failing manual test to make the panel green.** A
  test that caught something is working; file what it caught and fix
  the product, not the test. @status:impl/done
