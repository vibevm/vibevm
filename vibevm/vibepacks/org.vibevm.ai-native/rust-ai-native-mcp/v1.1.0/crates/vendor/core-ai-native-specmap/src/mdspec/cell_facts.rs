//! Existing fact grammar at table body cells and quoted paragraph leads.

specmark::scope!(
    "spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#FACT-ANCHORS-MINT-THE-FINEST-GRAIN"
);

use super::lines::fact_anchor_at;

pub(super) struct CarrierFact {
    pub(super) line: usize,
    pub(super) id: String,
    pub(super) heading: String,
    pub(super) span: String,
}

pub(super) fn carriers(lines: &[&str], start: usize, end: usize) -> Vec<CarrierFact> {
    let mut out = Vec::new();
    let mut body_rows = false;
    for index in start..end {
        let line = lines[index];
        if line.trim_start().starts_with('|') {
            let cells = cells(line);
            if delimiter(&cells) {
                body_rows = true;
                continue;
            }
            if body_rows {
                for cell in cells {
                    if let Some((id, heading)) = fact_anchor_at(cell, 0) {
                        out.push(CarrierFact {
                            line: index,
                            id,
                            heading,
                            span: cell.trim().to_owned(),
                        });
                    }
                }
            }
            continue;
        }
        body_rows = false;
        let Some(content) = line.trim_start().strip_prefix('>') else {
            continue;
        };
        let opens = index == start
            || !lines[index - 1].trim_start().starts_with('>')
            || lines[index - 1].trim() == ">";
        if !opens {
            continue;
        }
        if let Some((id, heading)) = fact_anchor_at(content, 0) {
            let mut tail = index + 1;
            while tail < end
                && lines[tail].trim_start().starts_with('>')
                && lines[tail].trim() != ">"
            {
                tail += 1;
            }
            out.push(CarrierFact {
                line: index,
                id,
                heading,
                span: lines[index..tail].join("\n"),
            });
        }
    }
    out
}

fn delimiter(cells: &[&str]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let cell = cell.trim().trim_matches(':');
            cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-')
        })
}

/// Pipe boundaries outside escaped pipes and paired inline-code runs. An
/// unmatched backtick stays ordinary text instead of hiding later cells.
fn cells(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let mut bounds = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => i += 2,
            b'`' => {
                let start = i;
                while i < bytes.len() && bytes[i] == b'`' {
                    i += 1;
                }
                let width = i - start;
                let mut close = i;
                while close < bytes.len() {
                    if bytes[close] != b'`' {
                        close += 1;
                        continue;
                    }
                    let run = close;
                    while close < bytes.len() && bytes[close] == b'`' {
                        close += 1;
                    }
                    if close - run == width {
                        i = close;
                        break;
                    }
                }
            }
            b'|' => {
                bounds.push(i);
                i += 1;
            }
            _ => i += 1,
        }
    }
    let mut cells: Vec<_> = bounds
        .windows(2)
        .map(|pair| &line[pair[0] + 1..pair[1]])
        .collect();
    if let Some(&last) = bounds.last() {
        let tail = &line[last + 1..];
        if !tail.trim().is_empty() {
            cells.push(tail);
        }
    }
    cells
}
