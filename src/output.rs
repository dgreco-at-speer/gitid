//! Small styled-output helpers. Colour is applied only when stderr/stdout is a
//! terminal and `NO_COLOR` is unset (handled by `owo-colors`'s support-colors
//! integration).

use owo_colors::OwoColorize;
use owo_colors::Stream::{Stderr, Stdout};

pub fn success(msg: &str) {
    println!(
        "{} {msg}",
        "✓".if_supports_color(Stdout, |t| t.green().to_string())
    );
}

pub fn info(msg: &str) {
    println!(
        "{} {msg}",
        "›".if_supports_color(Stdout, |t| t.blue().to_string())
    );
}

pub fn warn(msg: &str) {
    eprintln!(
        "{} {msg}",
        "warning:".if_supports_color(Stderr, |t| t.yellow().to_string())
    );
}

pub fn error(msg: &str) {
    eprintln!(
        "{} {msg}",
        "error:".if_supports_color(Stderr, |t| t.red().bold().to_string())
    );
}

pub fn hint(msg: &str) {
    eprintln!(
        "{} {msg}",
        "hint:".if_supports_color(Stderr, |t| t.cyan().to_string())
    );
}

/// Suggest the closest candidate to `target` within a small edit distance, for
/// "did you mean …?" hints on typos.
pub fn did_you_mean<'a>(target: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for cand in candidates {
        let d = levenshtein(target, cand);
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, cand));
        }
    }
    best.and_then(|(d, cand)| {
        let threshold = (target.len().max(1) / 2).max(2);
        (d <= threshold).then(|| cand.to_string())
    })
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Build a "no such profile" error message, appending a suggestion when one of
/// the existing names is a near-match.
pub fn no_such_profile<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> String {
    match did_you_mean(name, candidates) {
        Some(s) => format!("no such profile: {name} (did you mean {s:?}?)"),
        None => format!("no such profile: {name}"),
    }
}

/// Render rows as a left-aligned column table padded to the widest cell.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let cols = headers.len();
    let mut widths = vec![0usize; cols];
    for (i, h) in headers.iter().enumerate() {
        widths[i] = h.chars().count();
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(cols) {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let mut out = String::new();
    push_row(&mut out, headers.iter().map(|s| s.to_string()), &widths);
    for row in rows {
        push_row(&mut out, row.iter().cloned(), &widths);
    }
    out
}

fn push_row(out: &mut String, cells: impl Iterator<Item = String>, widths: &[usize]) {
    let parts: Vec<String> = cells.collect();
    let last = parts.len().saturating_sub(1);
    for (i, cell) in parts.iter().enumerate() {
        if i == last {
            out.push_str(cell);
        } else {
            let pad = widths[i].saturating_sub(cell.chars().count());
            out.push_str(cell);
            out.push_str(&" ".repeat(pad + 2));
        }
    }
    out.push('\n');
}
