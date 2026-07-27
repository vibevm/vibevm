# `flow:manual-tests` — prove the world, not just the logic {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-INSTALLS-A-SECOND-TEST-TIER A `flow` package that installs a **second test tier** into a project:
human-run markdown walkthroughs for the integration surfaces the
automated suite cannot reach. @impl/done

##the-automated-suite-is-fast-and-hermetic-because-it-substitutes The automated suite is fast and hermetic because it substitutes fakes
for real dependencies, temp directories for the real per-user layout,
and local fixtures for real remotes. @spec/done

##that-is-the-good-refactor-loop-and-why-it-cannot-prove-the-real-surfaces That is what makes it a good
refactor loop — and exactly why it cannot prove real authentication,
the on-disk directory layout a user actually gets, a consumer-facing
artifact byte-for-byte, or a human reading the output and confirming
it says what they meant. @spec/done

##THE-MANUAL-TIER-IS-THAT-LAST-MILE The manual tier is that last mile. @impl/done

##THE-MANUAL-TIER-COMPLEMENTS-AND-NEVER-REPLACES It
complements the automated suite; it never replaces it. @impl/done

##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done

- ##CONTENT-THE-FULL-PROTOCOL `spec/flows/manual-tests/MANUAL-TESTS-PROTOCOL.md` — full protocol:
  why a second tier exists and the surfaces it covers, what a manual
  test is and is not, the three triggers for running one, human
  sign-off versus agent pre-run, the directory convention, and a
  re-derive prompt for adapting the tier to any project. @impl/done
- ##CONTENT-THE-AUTHORING-RULES `spec/flows/manual-tests/authoring-rules.md` — the four rules with
  worked fragments: clean slate (scratch project plus an env redirect
  of the tool's per-user state), self-contained walkthrough (command
  block plus Expected paragraph per step), platform coverage
  (POSIX-compatible, primary-platform-first with portable notes), and
  exit discipline (one-command teardown plus a what-to-collect list). @impl/done
- ##CONTENT-THE-TEST-TEMPLATE `spec/flows/manual-tests/test-template.md` — the copy-ready
  skeleton, a clause-by-clause account of each section, and a short
  worked example (a generic CLI's first-run smoke test). @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/44-flow-manual-tests.md` — boot snippet loaded at session
  start: when to propose a manual test, the pre-run/sign-off split,
  and the never-do list. @impl/done

## Install {#install}

```bash
vibe install flow:manual-tests
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:manual-tests
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @impl/done

##USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @impl/done

## Composition {#composition}

- ##COMPOSES-CAMPAIGN-PLANS `flow:campaign-plans` — a campaign's whole-campaign acceptance
  script is the automated cousin of a manual test: both verify the
  finished work end to end. The script judges what a machine can
  judge; the manual test covers what it cannot — human-facing output
  and real integration surfaces. @spec/done
- ##COMPOSES-HEALTH-AUDIT `flow:health-audit` — unexecuted manual tests the index marks
  required, and Expected blocks that have rotted out of sync with the
  product, are audit findings under the "rot outside the gate"
  category: green automated panel, stale reality. @impl/done
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` — a manual test that exists because of an
  incident cites the decision or issue that spawned it, so the
  reproducer and the reasoning that demanded it stay linked. @impl/done

## Philosophical background {#background}

##crystallized-from-the-origin-projects-manual-test-protocol The tier crystallized in the origin project's manual-test protocol —
the discovery that a green automated suite and a shippable milestone
are not the same claim, and that the gap between them is exactly the
real world the fakes stood in for. @spec/done

##collections-spirit-is-the-redbook Its spirit is the collection's
book: write down what a cold reader must do to trust the work, then
let a human do it (*AI-native development*, ships in Russian inside
`flow:redbook` at `spec/book/ru/`). @spec/done

## License {#license}

##license-line UPL-1.0. See [`LICENSE.md`](LICENSE.md). @impl/done
