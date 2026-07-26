//! The checks themselves — C1 to C10, each writing into a [`Report`].
//!
//! Pass/fail checks state a fact about the batch. C2 and C10 are *surfaced*
//! rather than failed: they hand the reviewer a queue to judge, because
//! neither "is this structural insertion a repair" nor "is this `@unknown`
//! honest" is a question a checker may answer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::report::Report;
use super::text::*;
use super::{git_show, truncate, window};

// ---------------------------------------------------------------- checks
pub(super) fn c1_scope(files: &[String], scope: Option<&Vec<String>>, r: &mut Report) {
    let Some(scope) = scope else {
        r.ok("C1 scope", "no --scope given; containment not checked");
        return;
    };
    if scope.is_empty() {
        // An empty list must never read as "everything is out of scope". A
        // checker whose denominator silently became zero is this campaign's
        // single most repeated defect.
        r.fail(
            "C1 scope",
            "the --scope file is EMPTY -- refusing to check against a zero denominator",
        );
        return;
    }
    let sc: BTreeSet<&String> = scope.iter().collect();
    let fs: BTreeSet<&String> = files.iter().collect();
    let stray: Vec<_> = fs.difference(&sc).map(|s| s.as_str()).collect();
    let untouched: Vec<_> = sc.difference(&fs).map(|s| s.as_str()).collect();
    if stray.is_empty() {
        r.ok(
            "C1 scope",
            format!("{} changed file(s), all inside scope", fs.len()),
        );
    } else {
        r.fail(
            "C1 scope",
            format!(
                "{} file(s) outside scope: {}",
                stray.len(),
                stray.join(", ")
            ),
        );
    }
    if !untouched.is_empty() {
        // Not a failure: a file whose every unit was already marked, or whose
        // facts all came back unmarkable, is legitimately untouched.
        r.note(format!(
            "C1b {} scoped file(s) not touched: {}",
            untouched.len(),
            untouched.join(", ")
        ));
    }
}

pub(super) fn c2_lazy_continuation(
    files: &[String],
    base: Option<&str>,
    root: &Path,
    r: &mut Report,
) {
    let mut hits = Vec::new();
    for f in files {
        let Ok(new) = std::fs::read_to_string(root.join(f)) else {
            continue;
        };
        let old_keys: BTreeSet<String> = base
            .and_then(|b| git_show(root, b, f).ok())
            .map(|t| lazy_signature(&t).into_iter().map(|(_, k)| k).collect())
            .unwrap_or_default();
        for (lineno, key) in lazy_signature(&new) {
            if !old_keys.contains(&key) {
                hits.push(format!("{f}:{lineno}  {}", truncate(&key, 80)));
            }
        }
    }
    if !hits.is_empty() {
        r.note(format!(
            "C2 {} paragraph(s) sit directly after a list item (ruling-30 shape):",
            hits.len()
        ));
        for h in &hits {
            r.note(format!("     {h}"));
        }
        r.note("     -> confirm each is a lazy-continuation repair, not a content edit");
    }
    r.ok(
        "C2 lazy continuation",
        format!("{} candidate(s) surfaced for judgement", hits.len()),
    );
}

pub(super) fn c3_words(files: &[String], base: &str, root: &Path, r: &mut Report) {
    let (mut diverged, mut emphasis_lost) = (Vec::new(), Vec::new());
    for f in files {
        let old = match git_show(root, base, f) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let new = std::fs::read_to_string(root.join(f)).unwrap_or_default();
        let (a, b) = (word_stream(&old), word_stream(&new));
        if a != b {
            let at = a
                .iter()
                .zip(b.iter())
                .position(|(x, y)| x != y)
                .unwrap_or(a.len().min(b.len()));
            diverged.push(format!(
                "{f} @word {at}\n       HEAD: {}\n       WORK: {}",
                window(&a, at),
                window(&b, at)
            ));
        }
        if new.matches('*').count() < old.matches('*').count() {
            emphasis_lost.push(f.clone());
        }
    }
    if diverged.is_empty() {
        r.ok(
            "C3 words",
            format!("{} file(s) word-identical to {base}", files.len()),
        );
    } else {
        r.fail(
            "C3 words",
            format!(
                "{} file(s) diverge:\n     {}",
                diverged.len(),
                diverged.join("\n     ")
            ),
        );
    }
    if emphasis_lost.is_empty() {
        r.ok("C3b emphasis", "no file lost emphasis characters");
    } else {
        r.fail(
            "C3b emphasis",
            format!(
                "asterisk count DECREASED (ruling 12 permits only an increase): {}",
                emphasis_lost.join(", ")
            ),
        );
    }
}

pub(super) fn c4_gate(
    gate: &str,
    files: &[String],
    expect_unmarked: Option<usize>,
    expect_files: Option<&Vec<String>>,
    expect_total: Option<usize>,
    r: &mut Report,
) {
    let rows: Vec<&str> = gate
        .lines()
        .filter(|l| l.starts_with("packages/") || l.starts_with("spec/"))
        .collect();
    let fset: BTreeSet<&String> = files.iter().collect();
    let mine: Vec<&&str> = rows
        .iter()
        .filter(|l| {
            l.split(':')
                .next()
                .is_some_and(|p| fset.contains(&p.to_string()))
        })
        .collect();

    match expect_total {
        Some(n) if n == rows.len() => {
            r.ok("C4 corpus total", format!("{n} unmarked, as predicted"))
        }
        Some(n) => r.fail(
            "C4 corpus total",
            format!(
                "{} unmarked, predicted {n} (delta {:+})",
                rows.len(),
                rows.len() as i64 - n as i64
            ),
        ),
        None => r.ok(
            "C4 corpus total",
            format!("{} unmarked (no prediction given)", rows.len()),
        ),
    }
    if let Some(n) = expect_unmarked {
        if mine.len() == n {
            r.ok(
                "C4b batch residual",
                format!("{n} unmarked in the batch, as predicted"),
            );
        } else {
            r.fail(
                "C4b batch residual",
                format!("{} unmarked in the batch, predicted {n}", mine.len()),
            );
        }
    }
    if let Some(want) = expect_files {
        let got: BTreeSet<String> = mine
            .iter()
            .filter_map(|l| l.split(':').next().map(str::to_string))
            .collect();
        let want: BTreeSet<String> = want.iter().cloned().collect();
        if got == want {
            r.ok(
                "C4c residual files",
                "residual sits exactly in the predicted file(s)",
            );
        } else {
            r.fail(
                "C4c residual files",
                format!("residual in {got:?}, predicted {want:?}"),
            );
        }
    }
}

pub(super) fn c5_error_classes(gate: &str, files: &[String], r: &mut Report) {
    let fset: BTreeSet<&String> = files.iter().collect();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for line in gate.lines() {
        if !(line.starts_with("packages/") || line.starts_with("spec/")) {
            continue;
        }
        let Some(path) = line.split(':').next() else {
            continue;
        };
        if !fset.contains(&path.to_string()) {
            continue;
        }
        if let Some(open) = line.find('[')
            && let Some(close) = line[open..].find(']')
        {
            let class = line[open + 1..open + close].to_string();
            *classes.entry(class).or_default() += 1;
        }
    }
    let bad: BTreeMap<_, _> = classes
        .iter()
        .filter(|(k, _)| k.as_str() != "unmarked")
        .collect();
    if bad.is_empty() {
        r.ok(
            "C5 error classes",
            format!(
                "batch files carry only [unmarked] ({})",
                classes.get("unmarked").copied().unwrap_or(0)
            ),
        );
    } else {
        r.fail(
            "C5 error classes",
            format!("unexpected classes in batch files: {bad:?}"),
        );
    }
}

pub(super) fn c6_vocabulary(files: &[String], root: &Path, r: &mut Report) {
    let mut bad = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(root.join(f)) else {
            continue;
        };
        let text = blank_fences(&text);
        for el in find_status_elements(&text) {
            for (k, v) in attributes(&el.body) {
                let outside = match k.as_str() {
                    "stage" | "actionstage" => !STAGES.contains(&v.as_str()),
                    "state" => !STATES.contains(&v.as_str()),
                    "action" => !ACTIONS.contains(&v.as_str()),
                    "audience" => !AUDIENCES.contains(&v.as_str()),
                    _ => false,
                };
                if outside {
                    bad.push(format!("{f}: {k}={v:?}"));
                }
            }
        }
        for (_, stage, state) in marker_shorthands(&blank_code_spans_outside_fences(&text)) {
            if !STAGES.contains(&stage.as_str()) {
                bad.push(format!("{f}: @{stage}"));
            } else if let Some(s) = state
                && !STATES.contains(&s.as_str())
            {
                bad.push(format!("{f}: @{stage}/{s}"));
            }
        }
    }
    if bad.is_empty() {
        r.ok(
            "C6 vocabulary",
            "every stage/state/action/audience is inside PROP-043 3.3-3.6",
        );
    } else {
        let shown: Vec<_> = bad.iter().take(8).cloned().collect();
        r.fail(
            "C6 vocabulary",
            format!(
                "{} value(s) outside the closed vocabulary: {}",
                bad.len(),
                shown.join(", ")
            ),
        );
    }
}

pub(super) fn c7_anchors(files: &[String], root: &Path, r: &mut Report) {
    // Deliberately redundant with the gate's own anchor laws. A cross-check
    // written from the spec rather than from the parser is what catches a
    // parser blind to its own grammar -- found three times in this campaign.
    //
    // `##FENCE-AWARE` covers inline code spans as well as fenced blocks, and
    // this check blanked only the fences until B9 caught it: a protocol
    // document that DOCUMENTS the anchor syntax carries `` `{#id}` `` three
    // times in prose, and C7 reported a duplicate id named `id` while the gate
    // reported nothing at all. C6 and C9 were given code-span blanking during
    // the port and C7 was left behind -- so the two disagreed, which is the
    // only reason it surfaced.
    let mut bad = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(root.join(f)) else {
            continue;
        };
        let text = blank_fences(&blank_code_spans_outside_fences(&text));
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for (_, _, id) in fact_anchors(&text) {
            *seen.entry(id).or_default() += 1; // case-SENSITIVE, per F-085
        }
        for id in heading_anchors(&text) {
            *seen.entry(id).or_default() += 1;
        }
        let dupes: Vec<_> = seen
            .iter()
            .filter(|(_, n)| **n > 1)
            .map(|(k, _)| k.as_str())
            .take(5)
            .collect();
        if !dupes.is_empty() {
            bad.push(format!("{f}: {}", dupes.join(", ")));
        }
    }
    if bad.is_empty() {
        r.ok(
            "C7 anchors",
            "no id collides with another in its file (case-sensitive)",
        );
    } else {
        r.fail(
            "C7 anchors",
            format!(
                "duplicate id(s) in {} file(s): {}",
                bad.len(),
                bad.join("; ")
            ),
        );
    }
}

pub(super) fn c8_encoding(files: &[String], root: &Path, r: &mut Report) {
    let mut bad = Vec::new();
    for f in files {
        let Ok(raw) = std::fs::read(root.join(f)) else {
            continue;
        };
        if raw.starts_with(b"\xef\xbb\xbf") {
            bad.push(format!("{f}: BOM"));
        }
        if raw.windows(2).any(|w| w == b"\r\n") {
            bad.push(format!("{f}: CRLF"));
        }
    }
    if bad.is_empty() {
        r.ok("C8 encoding", "no BOM, no CRLF");
    } else {
        r.fail("C8 encoding", bad.join(", "));
    }
}

pub(super) fn c9_markers_in_fences(files: &[String], root: &Path, r: &mut Report) {
    let mut bad = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(root.join(f)) else {
            continue;
        };
        let text = blank_code_spans_outside_fences(&text);
        let blanked = blank_fences(&text);
        if find_status_elements(&text).len() != find_status_elements(&blanked).len() {
            bad.push(format!("{f}: status inside a fence"));
        }
        if fact_anchors(&text).len() != fact_anchors(&blanked).len() {
            bad.push(format!("{f}: anchor inside a fence"));
        }
        if marker_shorthands(&text).len() != marker_shorthands(&blanked).len() {
            bad.push(format!("{f}: shorthand inside a fence"));
        }
    }
    if bad.is_empty() {
        r.ok("C9 fences", "no marker or anchor inside a fenced block");
    } else {
        r.fail("C9 fences", bad.join(", "));
    }
}

pub(super) fn c10_unknowns(files: &[String], root: &Path, r: &mut Report) {
    // Not pass/fail. The judgement queue.
    let mut found = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(root.join(f)) else {
            continue;
        };
        for (n, line) in text.split('\n').enumerate() {
            if line.contains("@unknown") || line.contains("state=\"hold\"") {
                found.push(format!("{f}:{}  {}", n + 1, truncate(line.trim(), 110)));
            }
        }
    }
    r.ok(
        "C10 unknowns",
        format!("{} unit(s) held for triage", found.len()),
    );
    for line in found {
        r.note(format!("C10 {line}"));
    }
}

// ------------------------------------------------------------- controls
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# T {#root}

<status stage=\"spec\" state=\"done\"/>

##FACT-ONE The *quick* brown fox jumps. @impl/done

##fact-two It landed cleanly. @spec/done
";

    fn write(dir: &Path, body: &str) -> Vec<String> {
        std::fs::write(dir.join("probe.md"), body).unwrap();
        vec!["probe.md".to_string()]
    }

    #[test]
    fn a_bad_stage_is_caught() {
        let d = tempfile::tempdir().unwrap();
        let files = write(d.path(), &SAMPLE.replace("@impl/done", "@impll/done"));
        let mut r = Report::default();
        c6_vocabulary(&files, d.path(), &mut r);
        assert!(r.caught().iter().any(|c| c.starts_with("C6")));
    }

    #[test]
    fn a_bad_state_is_caught() {
        let d = tempfile::tempdir().unwrap();
        let files = write(d.path(), &SAMPLE.replace("@spec/done", "@spec/finished"));
        let mut r = Report::default();
        c6_vocabulary(&files, d.path(), &mut r);
        assert!(r.caught().iter().any(|c| c.starts_with("C6")));
    }

    /// `@ts-ignore` and `@typescript-eslint` are not shorthands. The first
    /// implementation flagged 45 of these on one batch.

    #[test]
    fn hyphenated_at_words_are_not_shorthands() {
        let d = tempfile::tempdir().unwrap();
        let body = "# T {#root}\n\n##A Ban `@ts-ignore`; prefer @ts-expect-error \
                    and @typescript-eslint rules. @impl/done\n";
        let files = write(d.path(), body);
        let mut r = Report::default();
        c6_vocabulary(&files, d.path(), &mut r);
        assert!(!r.failed(), "false positives: {:?}", r.caught());
    }

    #[test]
    fn a_duplicate_id_is_caught() {
        let d = tempfile::tempdir().unwrap();
        let files = write(d.path(), &SAMPLE.replace("##fact-two", "##FACT-ONE"));
        let mut r = Report::default();
        c7_anchors(&files, d.path(), &mut r);
        assert!(r.caught().iter().any(|c| c.starts_with("C7")));
    }

    /// Anchors are case-sensitive at every level (F-085): `##Foo` and `##foo`
    /// are two ids, not a collision.

    #[test]
    fn case_differing_ids_are_not_a_collision() {
        let d = tempfile::tempdir().unwrap();
        let files = write(d.path(), &SAMPLE.replace("##fact-two", "##Fact-One"));
        let mut r = Report::default();
        c7_anchors(&files, d.path(), &mut r);
        assert!(!r.failed(), "case folding crept in: {:?}", r.caught());
    }

    #[test]
    fn crlf_is_caught() {
        let d = tempfile::tempdir().unwrap();
        let files = write(d.path(), &SAMPLE.replace('\n', "\r\n"));
        let mut r = Report::default();
        c8_encoding(&files, d.path(), &mut r);
        assert!(r.caught().iter().any(|c| c.starts_with("C8")));
    }

    /// The false negative that mattered: the detector excluded every line
    /// beginning with `#`, and a marked ruling-30 paragraph begins with `##`.
    /// It reported clean on the one tree known to contain two real cases.

    #[test]
    fn a_clean_sample_stays_green() {
        let d = tempfile::tempdir().unwrap();
        let files = write(d.path(), SAMPLE);
        let mut r = Report::default();
        c6_vocabulary(&files, d.path(), &mut r);
        c7_anchors(&files, d.path(), &mut r);
        c8_encoding(&files, d.path(), &mut r);
        c9_markers_in_fences(&files, d.path(), &mut r);
        assert!(!r.failed(), "clean sample went red: {:?}", r.caught());
    }

    /// A document that DOCUMENTS the anchor syntax quotes it in prose. Those
    /// quotes live in inline code spans, which `##FENCE-AWARE` excludes — so
    /// they are not anchors and cannot collide. C7 blanked only fenced blocks
    /// until B9's protocol document reported a duplicate id named `id` while
    /// the gate reported nothing.
    #[test]
    fn an_anchor_quoted_in_an_inline_code_span_is_not_an_anchor() {
        let d = tempfile::tempdir().unwrap();
        let body = "# T {#root}

                    ##SYNTAX Heading anchors are written `{#id}` and facts `##ID`. @impl/done

                    ##ALSO Renderers turn `{#id}` into a link target. @impl/done
";
        let files = write(d.path(), body);
        let mut r = Report::default();
        c7_anchors(&files, d.path(), &mut r);
        assert!(
            !r.failed(),
            "quoted anchors read as real ones: {:?}",
            r.caught()
        );
    }

    #[test]
    fn a_marker_inside_a_fence_is_caught() {
        let d = tempfile::tempdir().unwrap();
        let body = "# T {#root}\n\n```\n##FAKE-ANCHOR text @impl/done\n```\n";
        let files = write(d.path(), body);
        let mut r = Report::default();
        c9_markers_in_fences(&files, d.path(), &mut r);
        assert!(r.caught().iter().any(|c| c.starts_with("C9")));
    }

    #[test]
    fn an_empty_scope_refuses_rather_than_passing() {
        let mut r = Report::default();
        c1_scope(&["a.md".to_string()], Some(&vec![]), &mut r);
        assert!(r.failed(), "a zero denominator must never read as clean");
    }

    #[test]
    fn the_gate_predictions_are_compared_exactly() {
        let gate = "packages/x/a.md:1: Error [unmarked] Para unit carries no marker\n\
                    packages/x/b.md:2: Error [unmarked] Para unit carries no marker\n";
        let files = vec!["packages/x/a.md".to_string()];
        let mut r = Report::default();
        c4_gate(gate, &files, Some(1), None, Some(2), &mut r);
        assert!(!r.failed());

        let mut r = Report::default();
        c4_gate(gate, &files, Some(2), None, Some(2), &mut r);
        assert!(r.failed(), "an off-by-one residual must fail");
    }
}
