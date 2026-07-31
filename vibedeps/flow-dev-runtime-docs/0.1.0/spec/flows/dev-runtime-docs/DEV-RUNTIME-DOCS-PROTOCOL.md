# Load-bearing setup docs {#root}

<status stage="spec" state="done"/>

##SETUP-AND-RUNTIME-DOCUMENTATION-IS-LOAD-BEARING A project's **setup and runtime documentation** is load-bearing: it is the file a
contributor or user reaches for when the build breaks, the environment is wrong, or a
prerequisite is missing. @impl/done

##when-the-doc-drifts-the-failure-happens-anyway When that doc drifts from reality, the failure it was meant to
prevent happens anyway — with no signpost to the fix. @spec/done

## The obligation {#obligation}

##EVERY-SETUP-TOUCHING-CHANGE-UPDATES-THE-DOC-IN-THE-SAME-COMMIT **Every change that touches the toolchain, prerequisites, environment variables, paths, or
bootstrap steps updates the relevant setup/runtime doc in the SAME commit.** @impl/done

##NEVER-DEFER-THE-DOC-UPDATE-TO-LATER Never ship a
setup change with the doc update deferred to "later" — later is where the drift lives, and a
contributor hitting a broken setup with no matching doc is exactly the failure these files
exist to prevent. @impl/done

## Why pin it centrally {#why-central}

##PIN-THE-OBLIGATION-IN-THE-FOUNDATIONAL-CONVENTIONS Pin the obligation in the project's **foundational conventions** — the material read early,
at session boot — not only inside the guides themselves. @impl/done

##every-contributor-meets-the-rule-before-touching-env-code That way every contributor meets
the rule *before* touching env or toolchain code, rather than only when they happen to open
the guide after something has already broken. @spec/done

## The two audiences {#audiences}

##SETUP-DOCS-SERVE-TWO-DISTINCT-READERS Setup docs serve two distinct readers; a change may touch one or both, in the one commit that
makes the change: @impl/done

- ##AUDIENCE-CONTRIBUTOR-BUILD-DOCS **Contributor / build docs** — what to install to clone, build, test, and (if authorised)
  publish the project. A change to a build prerequisite, a pinned toolchain, or a bootstrap
  step lands here. @impl/done
- ##AUDIENCE-RUNTIME-USER-DOCS **Runtime / user docs** — what an end user installs and configures to run the shipped
  artifact. A change to a user-facing environment variable, path convention, prerequisite, or
  auth flow lands here. @impl/done

## Keeping them honest {#honest}

##THE-DOCS-ARE-ONLY-LOAD-BEARING-WHILE-THEY-ARE-TRUE The docs are only load-bearing while they are true. @impl/done

##two-habits-keep-them-so-lead Two habits keep them so: @impl/done

- ##HABIT-SAME-COMMIT-UPDATES **Same-commit updates** (the obligation above) stop new drift at the source. @impl/done
- ##HABIT-A-PERIODIC-READ-THROUGH **A periodic read-through** against the actual toolchain catches the drift that slipped in
  before the obligation was adopted, or through a change nobody recognised as setup-touching. @impl/done

## Re-derive for your project {#re-derive}

##NAME-YOUR-SETUP-DOCS-AND-CITE-THEM-FROM-YOUR-CONVENTIONS Name your setup docs (one for contributors, one for users, or a single file if the project is
small), state the same-commit obligation in your foundational conventions, and cite the two
docs from there. @impl/done

##THE-PATTERN-IS-FILENAME-AGNOSTIC The pattern is filename-agnostic — what matters is that a setup change and
its doc update are never separable. @impl/done
