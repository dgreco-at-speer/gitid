//! Styled-output helpers: colour gating, message glyphs, bordered-but-borderless
//! tables (via `comfy-table`), spinners (via `indicatif`), and a shared theme for
//! `inquire` prompts.
//!
//! Colour is governed by a single process-wide gate ([`color_enabled`]) so that
//! `owo-colors`, `comfy-table`, spinners, and prompts all agree. The gate honours
//! `--no-color`, `NO_COLOR`, and whether stdout is a terminal.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use owo_colors::OwoColorize;
use owo_colors::Stream::{Stderr, Stdout};

// 0 = auto-detect, 1 = force on, 2 = force off.
static COLOR: AtomicU8 = AtomicU8::new(0);

/// Whether coloured/styled output should be emitted. Single source of truth for
/// every styling decision in the crate.
pub fn color_enabled() -> bool {
    match COLOR.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => detect_color(),
    }
}

fn detect_color() -> bool {
    // NO_COLOR (any non-empty value) disables colour; otherwise require a tty.
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    std::io::stdout().is_terminal()
}

/// Force colour on or off, overriding auto-detection. Wires the same decision
/// into `owo-colors` so its `if_supports_color` calls agree.
pub fn set_color_override(on: bool) {
    COLOR.store(if on { 1 } else { 2 }, Ordering::Relaxed);
    owo_colors::set_override(on);
}

/// Resolve auto-detection once and pin `owo-colors` to the result, so colour is
/// consistent across the whole run (and matches the `comfy-table` gate).
pub fn init_color() {
    let on = detect_color();
    COLOR.store(if on { 1 } else { 2 }, Ordering::Relaxed);
    owo_colors::set_override(on);
}

/// Install the shared `inquire` prompt theme (a no-op-ish plain theme when colour
/// is disabled). Call once at startup.
pub fn init_prompts() {
    use inquire::ui::{Color, RenderConfig, StyleSheet, Styled};

    if !color_enabled() {
        inquire::set_global_render_config(RenderConfig::empty());
        return;
    }
    let rc = RenderConfig::default()
        .with_prompt_prefix(Styled::new("?").with_fg(Color::LightGreen))
        .with_answered_prompt_prefix(Styled::new("✓").with_fg(Color::LightGreen))
        .with_highlighted_option_prefix(Styled::new("›").with_fg(Color::LightCyan))
        .with_help_message(StyleSheet::new().with_fg(Color::DarkGrey));
    inquire::set_global_render_config(rc);
}

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

/// Like [`info`], but on stderr — for ambient notices (e.g. "update available")
/// that must never pollute stdout consumed by pipelines.
pub fn note(msg: &str) {
    eprintln!(
        "{} {msg}",
        "›".if_supports_color(Stderr, |t| t.blue().to_string())
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

/// Render `msg` dimmed (e.g. for detail labels), respecting the colour gate.
pub fn dim(msg: &str) -> String {
    msg.if_supports_color(Stdout, |t| t.dimmed().to_string())
        .to_string()
}

/// Run `f`, showing a subtle spinner on stderr while it works. The spinner is
/// only drawn when colour is enabled and stderr is a terminal, so piped/CI runs
/// emit no extra bytes.
pub fn with_spinner<T>(msg: &str, f: impl FnOnce() -> T) -> T {
    if !(color_enabled() && std::io::stderr().is_terminal()) {
        return f();
    }
    use indicatif::{ProgressBar, ProgressStyle};
    let pb = ProgressBar::new_spinner();
    if let Ok(style) = ProgressStyle::with_template("{spinner:.cyan} {msg}") {
        pb.set_style(style.tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "));
    }
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    let out = f();
    pb.finish_and_clear();
    out
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

/// Render rows as a borderless table with dim-bold headers and a subtle header
/// rule. See [`table_with_active`] to additionally highlight one row.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    table_with_active(headers, rows, None)
}

/// Like [`table`], but the row at `active` (if any) is rendered green — used to
/// mark the profile active for the current directory.
pub fn table_with_active(headers: &[&str], rows: &[Vec<String>], active: Option<usize>) -> String {
    use comfy_table::presets::NOTHING;
    use comfy_table::{Cell, ColumnConstraint, Table, TableComponent, Width};

    let color = color_enabled();
    let mut table = Table::new();
    table.load_preset(NOTHING);
    // A single horizontal rule beneath the header; no inter-row lines.
    table.set_style(TableComponent::HeaderLines, '─');

    let header_cells = headers.iter().map(|h| {
        let text = if color {
            h.if_supports_color(Stdout, |t| t.dimmed().bold().to_string())
                .to_string()
        } else {
            (*h).to_string()
        };
        Cell::new(text)
    });
    table.set_header(header_cells);

    for (i, row) in rows.iter().enumerate() {
        let is_active = color && active == Some(i);
        let cells = row.iter().map(|c| {
            let text = if is_active {
                c.if_supports_color(Stdout, |t| t.green().to_string())
                    .to_string()
            } else {
                c.clone()
            };
            Cell::new(text)
        });
        table.add_row(cells);
    }

    // Left padding 0, right padding 2 — a clean, greppable, left-aligned grid.
    for i in 0..headers.len() {
        if let Some(col) = table.column_mut(i) {
            col.set_padding((0, 2));
            col.set_constraint(ColumnConstraint::LowerBoundary(Width::Fixed(0)));
        }
    }

    format!("{table}\n")
}
