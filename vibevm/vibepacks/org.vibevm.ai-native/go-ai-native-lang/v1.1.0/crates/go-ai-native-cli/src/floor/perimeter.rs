//! The floor's shared package perimeter — the four tool steps
//! (`vet`/`test`/`staticcheck`/`exhaustive`) judge the same
//! `[go].exclude_substrings` boundary `gofmt` does. `go list ./...` is
//! resolved once here, reduced by that filter, and handed back to the floor
//! as `./...` (when nothing was excluded) or an explicit package list.

specmark::scope!("spec://go-ai-native-lang/go/GUIDE-AI-NATIVE-GO#baseline");

use std::path::Path;
use std::process::Command;

/// The packages `go list ./...` reports, reduced to the floor's perimeter
/// by the same `[go].exclude_substrings` rule `filter_gofmt_listed` applies
/// to files — so `vet`/`test`/`staticcheck`/`exhaustive` judge the identical
/// boundary gofmt did. `go list` prints one import path per line
/// (forward-slash, never back-slashed); each line is trimmed so a stray
/// `\r` (Windows pipe capture) or blank cannot leak into a `go vet <pkg>`
/// argument and break the package lookup.
///
/// The return carries the *short-vs-long* decision, not just the survivors:
/// `All` (nothing was excluded ⇒ the perimeter is plain `./...`) vs
/// `Packages` (at least one was excluded ⇒ the step must name the survivors
/// explicitly). Splitting them here keeps the common case — a project whose
/// excludes match nothing — on the zero-risk `./...` path, so an explicit
/// argument list (and its command-line length cost) appears only where an
/// exclusion actually fired. Pure (no I/O) — unit-tested below.
fn filter_go_list_packages(raw: &str, excludes: &[String]) -> Perimeter {
    let mut kept: Vec<String> = Vec::new();
    let mut dropped_any = false;
    for line in raw.lines() {
        let pkg = line.trim();
        if pkg.is_empty() {
            continue;
        }
        if excludes.iter().any(|s| pkg.contains(s.as_str())) {
            dropped_any = true;
        } else {
            kept.push(pkg.to_owned());
        }
    }
    match (kept.is_empty(), dropped_any) {
        (true, _) => Perimeter::Empty,
        (false, false) => Perimeter::All,
        (false, true) => Perimeter::Packages(kept),
    }
}

/// The shared package perimeter the floor resolves once from `go list`,
/// then hands to the vet/test/staticcheck/exhaustive steps so all four
/// judge the same boundary gofmt already did.
#[derive(Debug, PartialEq)]
pub(super) enum Perimeter {
    /// `go list ./...` succeeded and the exclusion filter dropped nothing —
    /// the perimeter is the whole root, so the step runs over `./...`. This
    /// is the zero-risk short path: a project whose excludes match nothing
    /// keeps exactly today's behaviour and never pays the command-line
    /// length an explicit list would cost.
    All,
    /// `go list ./...` succeeded and at least one package was excluded —
    /// the step runs over these survivors only, named explicitly. See
    /// [`add_packages_or_wildcard`] for the residual command-line limit
    /// this explicit form carries on very large trees.
    Packages(Vec<String>),
    /// `go list` succeeded but no package survived (empty output, or every
    /// package excluded) — each consuming step is a *visible skip*, never a
    /// silent green (the vacuum-green the exclusions exist to prevent).
    Empty,
    /// `go list` itself did not run (Go missing) or failed (broken module).
    /// Diagnosed once at resolution; consuming steps fail with the recipe,
    /// never a silent fallback to `./...`.
    Unavailable,
}

/// How a perimeter-consuming step's header describes its scope — honest in
/// both forms: `./...` when nothing was excluded (the short path), or the
/// explicit survivor count when something was. Surfacing the form at the
/// header line keeps a reduced perimeter visible, so a narrow floor can
/// never masquerade as the full one.
pub(super) fn perimeter_scope(perimeter: Option<&Perimeter>) -> String {
    match perimeter {
        Some(Perimeter::All) => "./...".to_string(),
        Some(Perimeter::Packages(p)) => format!("{} explicit package(s)", p.len()),
        Some(Perimeter::Empty) => "0 packages".to_string(),
        Some(Perimeter::Unavailable) => "perimeter unavailable".to_string(),
        // Unreachable in a consuming block (perimeter is `None` only when
        // every needing step is disabled).
        None => "perimeter not resolved".to_string(),
    }
}

/// Resolves the floor's shared package perimeter — `go list ./...` once,
/// then the same exclusion filter gofmt applied. Lazy: returns `None`
/// unless at least one perimeter-consuming step is policy-enabled, so a
/// fully-disabled tail never spawns `go list`. A failed or absent `go list`
/// is `Unavailable` carrying the install recipe — never a silent `./...`
/// fallback, which would restore the very defect this closes.
pub(super) fn resolve_perimeter(
    root: &Path,
    excludes: &[String],
    needed: bool,
) -> Option<Perimeter> {
    if !needed {
        return None;
    }
    let mut cmd = crate::tools::go_command(root);
    cmd.args(["list", "./..."]);
    match cmd.output() {
        Ok(out) if out.status.success() => {
            let raw = String::from_utf8_lossy(&out.stdout);
            Some(filter_go_list_packages(&raw, excludes))
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!(
                "floor: `go list ./...` exited {:?} — install go >= 1.24 and put it on \
                 PATH, or repair the module. The floor perimeter is unavailable; \
                 vet/test/staticcheck cannot run. {}",
                out.status.code(),
                stderr.trim()
            );
            Some(Perimeter::Unavailable)
        }
        Err(e) => {
            eprintln!(
                "floor: `go list ./...` did not spawn ({e}) — install go >= 1.24 and \
                 put it on PATH. The floor perimeter is unavailable; \
                 vet/test/staticcheck cannot run."
            );
            Some(Perimeter::Unavailable)
        }
    }
}

/// Appends the package arguments a perimeter step runs over to `cmd`:
/// `./...` when the filter dropped nothing (the short path), or the
/// explicit survivor list when it did.
///
/// Residual limit, recorded not hidden: the explicit list travels as
/// command-line *arguments*, so it is bounded by the OS command-line limit
/// (~32 767 chars on Windows); at ~80–120 chars per Go import path that
/// bites around a few hundred packages that survived exclusion — i.e. only
/// on a tree both large and partially excluded. Chunking the run is
/// deliberately out of scope here; the `All` short path keeps the common,
/// no-exclusion case off this limit entirely.
pub(super) fn add_packages_or_wildcard(cmd: &mut Command, pkgs: Option<&[String]>) {
    match pkgs {
        Some(list) => {
            cmd.args(list);
        }
        None => {
            cmd.arg("./...");
        }
    }
}

/// Runs a perimeter-consuming step. `Some(ok)` ⇒ the step ran (record it);
/// `None` ⇒ the perimeter was empty and the step was a visible skip (do
/// *not* record — the same shape as the test-gate no-baseline skip, and
/// never a silent green). The closure receives the package arguments to
/// append: `None` for the `./...` short path, `Some(list)` for the explicit
/// survivors.
pub(super) fn run_perimeter_step(
    perimeter: Option<&Perimeter>,
    label: &str,
    verb: &str,
    run: impl FnOnce(Option<&[String]>) -> bool,
) -> Option<bool> {
    match perimeter {
        Some(Perimeter::All) => Some(run(None)),
        Some(Perimeter::Packages(pkgs)) => Some(run(Some(pkgs.as_slice()))),
        Some(Perimeter::Empty) => {
            eprintln!(
                "  {label}: floor perimeter empty after `[go].exclude_substrings` — \
                 nothing to {verb}; step skipped (not green)"
            );
            None
        }
        Some(Perimeter::Unavailable) => {
            eprintln!(
                "  {label}: step failed — floor perimeter unavailable (`go list \
                 ./...` produced no packages; see above)"
            );
            Some(false)
        }
        // Unreachable in a consuming block (perimeter is `None` only when
        // every needing step is disabled), but defended: never a silent pass.
        None => Some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Go default `[go].exclude_substrings`, as `run_floor` sees it —
    /// kept here as a fixture so the filter tests track the real policy.
    fn go_excludes() -> Vec<String> {
        vec![
            "/testdata/".to_string(),
            "/vendor/".to_string(),
            "/fixtures/".to_string(),
        ]
    }

    /// `go list ./...` reports one import path per line; the same
    /// `[go].exclude_substrings` perimeter that drops gofmt's fixture files
    /// drops the fixture packages — so vet/test/staticcheck never judge the
    /// deliberately-wrong exhibits. At least one was excluded ⇒ the step
    /// gets the explicit survivor list (`Packages`), never `./...`.
    #[test]
    fn go_list_packages_drop_fixture_packages() {
        let raw = "go-extract\n\
                   go-extract/test/fixtures/clean/internal/cells/greet\n\
                   go-extract/test/fixtures/dirty/internal/cells/plan\n";
        assert_eq!(
            filter_go_list_packages(raw, &go_excludes()),
            Perimeter::Packages(vec!["go-extract".to_string()])
        );
    }

    /// Nothing excluded ⇒ the perimeter is the whole root (`All` ⇒ `./...`):
    /// the zero-risk short path. A project whose excludes match nothing
    /// stays on `./...` and never builds a command-line argument list.
    #[test]
    fn go_list_packages_nothing_excluded_is_the_whole_root() {
        let raw = "example.com/proj/internal/cells/plan\n\
                   example.com/proj/internal/registry\n";
        assert_eq!(filter_go_list_packages(raw, &go_excludes()), Perimeter::All);
    }

    /// Non-empty `go list` output that is entirely excluded yields an empty
    /// perimeter — the consuming step becomes a visible skip, not a green.
    #[test]
    fn go_list_packages_all_excluded_yields_empty_perimeter() {
        let raw = "go-extract/test/fixtures/clean/src\n\
                   go-extract/test/fixtures/dirty/src\n";
        assert_eq!(
            filter_go_list_packages(raw, &go_excludes()),
            Perimeter::Empty
        );
    }

    /// Empty raw output (no module / no packages) ⇒ empty perimeter.
    #[test]
    fn go_list_packages_empty_input_yields_empty() {
        assert_eq!(
            filter_go_list_packages("", &go_excludes()),
            Perimeter::Empty
        );
    }

    /// Blank / whitespace-only lines are not packages and must NOT flip the
    /// perimeter to the explicit-list form: a tree whose only "dropped"
    /// lines are blanks stays on the `./...` short path (`All`).
    #[test]
    fn go_list_packages_blank_lines_do_not_force_explicit_list() {
        let raw = "\nexample.com/proj/a\n\n   \nexample.com/proj/b\n";
        assert_eq!(filter_go_list_packages(raw, &go_excludes()), Perimeter::All);
    }

    /// A trailing `\r` (Windows pipe capture) is trimmed, so it cannot leak
    /// into a `go vet <pkg>` argument. Paired with one excluded fixture so
    /// the assertion also checks the surviving packages come back clean and
    /// in order.
    #[test]
    fn go_list_packages_trims_crlf_and_reports_explicit_list() {
        let raw = "example.com/proj/a\r\n\
                   example.com/test/fixtures/clean/b\r\n\
                   example.com/proj/c\r\n";
        assert_eq!(
            filter_go_list_packages(raw, &go_excludes()),
            Perimeter::Packages(vec![
                "example.com/proj/a".to_string(),
                "example.com/proj/c".to_string(),
            ])
        );
    }
}
