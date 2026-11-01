//! The comparison every golden capture of the terminal display language
//! (PROP-067 `#acceptance`) goes through: the volatile parts of a run
//! normalised away, then the plain text compared with the stored capture.

#![allow(dead_code, reason = "each golden target uses a different subset")]

use std::fs;
use std::path::Path;

pub const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/display");

/// Times, run stamps, temporary paths and the length of a rule are the
/// parts of a capture that change from run to run.
pub fn normalise(text: &str, paths: &[(&Path, &str)]) -> String {
    let mut text = text.to_owned();
    for (path, name) in paths {
        text = text.replace(&path.display().to_string(), name);
    }
    text.lines()
        .map(|line| {
            let line = stamps(line);
            let line = line
                .split(" · ")
                .map(|part| if is_time(part) { "<time>" } else { part })
                .collect::<Vec<_>>()
                .join(" · ");
            rule_end(line.trim_end())
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
        + "\n"
}

/// Whether `part` is a time in the shortest exact form of PROP-067
/// `#meta`: `0s`, `2m 10s`, `1h 4m`.
fn is_time(part: &str) -> bool {
    !part.is_empty()
        && part.split(' ').all(|word| {
            word.is_ascii()
                && word.len() > 1
                && word[..word.len() - 1].bytes().all(|b| b.is_ascii_digit())
                && matches!(word.as_bytes()[word.len() - 1], b's' | b'm' | b'h')
        })
}

/// A run directory's stamp, `20261101T085226.796Z`, as `<stamp>`.
fn stamps(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < line.len() {
        let rest = &bytes[index..];
        let stamp = rest.len() >= 20
            && rest[..8].iter().all(u8::is_ascii_digit)
            && rest[8] == b'T'
            && rest[9..15].iter().all(u8::is_ascii_digit)
            && rest[15] == b'.'
            && rest[16..19].iter().all(u8::is_ascii_digit)
            && rest[19] == b'Z';
        if stamp {
            out.push_str("<stamp>");
            index += 20;
        } else {
            let ch = line[index..].chars().next().unwrap();
            out.push(ch);
            index += ch.len_utf8();
        }
    }
    out
}

/// A rule's run of `─` to the viewport, shortened to four.
fn rule_end(line: &str) -> String {
    if !line.starts_with("── ") {
        return line.to_owned();
    }
    let kept = line.trim_end_matches('─');
    format!("{kept}────")
}

/// Compare with the stored capture; on a difference the new capture is
/// written beside the build for review.
pub fn golden(name: &str, actual: &str) {
    let path = Path::new(GOLDEN).join(format!("{name}.txt"));
    let expected = fs::read_to_string(&path).unwrap_or_default();
    if expected != actual {
        let review = Path::new(env!("CARGO_TARGET_TMPDIR")).join("display-golden");
        fs::create_dir_all(&review).unwrap();
        fs::write(review.join(format!("{name}.txt")), actual).unwrap();
        panic!(
            "{name} differs from {}; the new capture is in {}:\n{actual}",
            path.display(),
            review.display()
        );
    }
}
