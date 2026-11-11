#![allow(dead_code)]

//! Ledger consistency check — lints the parity ledger and the
//! behavioral-claims ledger against the rules in
//! ADR-0007/0008/0009/0010, and generates their rendered files.
//!
//! **This is a development tool, not game code.** It ships inside the
//! `recomp2jam` package for toolchain convenience (one `cargo` invocation,
//! `clippy`/`fmt` coverage), but it is not part of the client, the shell,
//! or any library the game depends on. It reads documentation files and
//! exits; it never touches game state, archives, or the network.
//!
//! Source of truth (ADR-0007 as amended by ticket #77):
//! - `docs/claims/*.md` — the hand-kept claim fragments. The generator
//!   renders `docs/claims.md` from them (`gen claims`); the lint checks
//!   the fragments, never the rendered file, plus a freshness check
//!   that the rendered file matches.
//! - `docs/parity/*.md` — the per-section parity ledger (ticket #75).
//!   The generator renders the `docs/parity.md` index (`gen
//!   parity-index`); the lint checks the section files plus index
//!   freshness.
//!
//! Runs in CI without `install-root`. `--report` prints the violations;
//! without the flag the test fails on any violation in a row the re-audit
//! has marked done (a row whose text contains `(audit:done)`) and on any
//! stale generated file, and is otherwise tolerant.

use std::path::Path;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let docs_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    if args.first().map(String::as_str) == Some("gen") {
        generate(&docs_root, args.get(1).map(String::as_str));
        return;
    }
    let report_only = args.iter().any(|a| a == "--report");
    let violations = lint(&docs_root);
    for v in &violations {
        println!("{v}");
    }
    let hard = violations.iter().filter(|v| v.is_hard()).count();
    let done = violations
        .iter()
        .filter(|v| v.as_str().contains("audit:done"))
        .count();
    println!(
        "{} violation(s) found ({hard} hard, {done} audit:done)",
        violations.len()
    );
    if !report_only && hard > 0 {
        exit(1);
    }
}

fn generate(docs_root: &Path, target: Option<&str>) {
    match target {
        Some("claims") => {
            let rendered = claims::render_claims(docs_root);
            let path = docs_root.join("claims.md");
            std::fs::write(&path, rendered).expect("write claims.md");
            println!("wrote {}", path.display());
        }
        Some("parity-index") => {
            let rendered = parity::render_index(docs_root);
            let path = docs_root.join("parity.md");
            std::fs::write(&path, rendered).expect("write parity.md");
            println!("wrote {}", path.display());
        }
        other => {
            eprintln!("usage: ledger-check gen [claims|parity-index] (got {other:?})");
            exit(2);
        }
    }
}

fn lint(docs_root: &Path) -> Vec<Violation> {
    let parity_sections = parity::read_sections(docs_root);
    let parity_text = parity::concat_sections(&parity_sections);
    let claim_fragments = claims::read_fragments(docs_root);
    let mut out = Vec::new();
    check_parity_cells(&parity_text, &mut out);
    check_claims_rows(&claim_fragments, &mut out);
    check_cells_join(&parity_text, &claim_fragments, &mut out);
    check_generated_files_fresh(docs_root, &mut out);
    out
}

/// One lint finding: file, line, rule, snippet, plus whether it fails
/// the run even without an `(audit:done)` marker (stale generated
/// files).
pub struct Violation {
    message: String,
    hard: bool,
}

impl Violation {
    fn row(message: String) -> Self {
        Self {
            message,
            hard: false,
        }
    }

    fn stale(message: String) -> Self {
        Self {
            message,
            hard: true,
        }
    }

    /// The raw message, for callers that need to match on content.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.message
    }

    /// Hard violations fail the run regardless of `(audit:done)` marks.
    #[must_use]
    pub fn is_hard(&self) -> bool {
        self.hard
    }
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Splits a markdown table row on its unescaped pipes and trims each
/// cell. `"| a | b\\|c | d |"` yields `["a", "b\\|c", "d"]`.
fn split_row_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                current.push(c);
                if let Some(&next) = chars.peek() {
                    current.push(next);
                    chars.next();
                }
            }
            '|' => {
                cells.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    cells.push(current.trim().to_owned());
    cells
}

/// A claim table row: 7 columns (`ID | Claim | Level | Anchor |
/// Drives | Cells | Sibling`), parsed from one fragment line.
struct ClaimRow {
    line: usize,
    cells: Vec<String>,
}

impl ClaimRow {
    fn id(&self) -> &str {
        &self.cells[0]
    }

    fn level(&self) -> &str {
        &self.cells[2]
    }

    fn anchor(&self) -> &str {
        &self.cells[3]
    }

    fn cells_column(&self) -> &str {
        &self.cells[5]
    }
}

/// Parses every claim row in a fragment. Rows whose first cell is not
/// an `A`/`B`/`C`/`D` + digits id (table headers, prose) are skipped.
fn parse_claim_rows(text: &str) -> Vec<ClaimRow> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if !line.starts_with("| ") {
            continue;
        }
        let cells = split_row_cells(line);
        if cells.len() != 9 {
            continue;
        }
        let id = cells[1].as_str();
        let mut id_chars = id.chars();
        let is_claim_id =
            matches!(id_chars.next(), Some('A'..='D')) && id_chars.all(|c| c.is_ascii_digit());
        if !is_claim_id {
            continue;
        }
        rows.push(ClaimRow {
            line: i + 1,
            cells: cells[1..8].to_vec(),
        });
    }
    rows
}

/// The `—`/`–`/`-` marker: the claim drives no parity cell.
fn is_no_cell_marker(entry: &str) -> bool {
    matches!(entry, "—" | "–" | "-")
}

fn normalise_cell_id(s: &str) -> String {
    // Pre-re-audit aliases rename the scene part for every dimension.
    let Some((scene, dim)) = s.split_once('.') else {
        return s.to_owned();
    };
    let scene = match scene {
        "WAIT" => "WAITING",
        "CHAN" => "CHANNEL",
        "PLAY" => "PLAYING",
        "LOAD" => "LOADING",
        other => other,
    };
    format!("{scene}.{dim}")
}

fn is_cell_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 2 {
        return false;
    }
    let scenes = [
        "AVI", "CHANNEL", "ROOM", "WAITING", "LOADING", "PLAYING", "RESULT", "PLANET", "SHOP",
        "MSHOP", "MYROOM", "LANKING", "PAYMENT", "PAYMENT2", "TUTORIAL",
        // aliases used in the Cells column before the re-audit
        "WAIT", "CHAN", "PLAY", "LOAD",
    ];
    let dims = ["COMP", "INPUT", "TIME", "TRANS", "GAME", "MUSIC", "CHAT"];
    scenes.contains(&parts[0]) && dims.contains(&parts[1])
}

// -- parity.md --

fn check_parity_cells(text: &str, out: &mut Vec<Violation>) {
    let mut checked = 0usize;
    // Dimension cells are 4-column tables under a "## " section.
    // We look for lines that start "| <DIM> |" where DIM is a known
    // dimension name and check the verdict column.
    let known_dims = [
        "Composition",
        "Input",
        "Timing",
        "Transitions",
        "Gameplay",
        "Music-select",
        "Chat",
    ];
    for (i, line) in text.lines().enumerate() {
        let ln = i + 1;
        if !line.starts_with("| ") {
            continue;
        }
        let raw: Vec<&str> = line.split('|').collect();
        // "| a | b | c | d |" -> 6 parts; data rows have exactly 6.
        if raw.len() != 6 {
            continue;
        }
        let cells: Vec<&str> = raw.iter().map(|s| s.trim()).collect();
        let dim = cells[1];
        if !known_dims.contains(&dim) {
            continue;
        }
        checked += 1;
        let verdict = cells[4];
        check_verdict(dim, verdict, &cells.join(" "), ln, out);
        check_cells_have_guards(dim, &cells, ln, out);
    }
    if checked == 0 {
        out.push(Violation::row(
            "parity: no dimension cell rows found".to_owned(),
        ));
    }
}

fn check_verdict(dim: &str, verdict: &str, full_row: &str, ln: usize, out: &mut Vec<Violation>) {
    let v = verdict.trim();
    // Strip a leading "(TIER)" parenthetical so "PASS (R+G, v: …)" and
    // "(R+G) PASS" both parse; the verdict word must still come first
    // or second.
    let stripped = if v.starts_with('(') {
        match v.find(')') {
            Some(close) => v[close + 1..].trim().to_owned(),
            None => v.to_owned(),
        }
    } else {
        v.to_owned()
    };
    let valid = ["PASS", "ACCEPTED", "OPEN", "FAIL", "N/A"];
    let starts_valid = valid.iter().any(|p| stripped.starts_with(p));
    if !starts_valid {
        out.push(Violation::row(format!(
            "parity:{ln}: [{dim}] verdict does not start with PASS/ACCEPTED/OPEN/FAIL/N/A: {v:.60}"
        )));
    }
    // A row whose verdict says OPEN/FAIL but the text claims PASS is a stale marker.
    if (stripped.starts_with("OPEN") || stripped.starts_with("FAIL"))
        && full_row.to_lowercase().contains("pass")
        && !full_row.to_lowercase().contains("pass-led")
    {
        out.push(Violation::row(format!(
            "parity:{ln}: [{dim}] verdict {v:.30} but row text claims PASS — stale marker?"
        )));
    }
    if stripped.starts_with("OPEN")
        && !full_row.contains("next step")
        && !full_row.contains("Next step")
    {
        out.push(Violation::row(format!(
            "parity:{ln}: [{dim}] OPEN verdict names no next step: {v:.60}"
        )));
    }
}

fn check_cells_have_guards(dim: &str, cells: &[&str], ln: usize, out: &mut Vec<Violation>) {
    let shell = cells.get(3).unwrap_or(&"");
    let verdict = cells.get(4).unwrap_or(&"");
    let combined = format!("{shell} {verdict}");
    let has_guard_mention = combined.contains("guard")
        || combined.contains("Guard")
        || combined.contains("soak")
        || combined.contains("shots")
        || combined.contains("test")
        || combined.contains("walk")
        || combined.contains("n/a (see")
        || combined.contains("guards above")
        || combined.contains("Loading")
        || combined.contains("loading_phase_status")
        || combined.contains("sample clock")
        || combined.contains("backend-sample")
        || combined.contains("Song position");
    if !has_guard_mention {
        out.push(Violation::row(format!(
            "parity:{ln}: [{dim}] row names no guard test source"
        )));
    }
}

// -- claims fragments --

fn check_claims_rows(fragments: &[(String, String)], out: &mut Vec<Violation>) {
    let mut seen_ids = std::collections::HashSet::new();
    for (name, text) in fragments {
        for row in parse_claim_rows(text) {
            let id = row.id();
            let ln = row.line;
            if !seen_ids.insert(id.to_string()) {
                out.push(Violation::row(format!(
                    "claims/{name}:{}: duplicate claim id {id}",
                    row.line
                )));
            }
            let level = row.level();
            if !["EXE", "DATA", "ASSUMED", "OPEN"].contains(&level) {
                out.push(Violation::row(format!(
                    "claims/{name}:{ln}: [{id}] level '{level}' is not EXE/DATA/ASSUMED/OPEN"
                )));
            }
            if row.anchor().is_empty() {
                out.push(Violation::row(format!(
                    "claims/{name}:{ln}: [{id}] empty anchor/evidence cell"
                )));
            }
            if row.cells_column().is_empty() {
                out.push(Violation::row(format!(
                    "claims/{name}:{ln}: [{id}] empty Cells column (no driving cell named)"
                )));
            }
        }
    }
}

// -- join --

fn check_cells_join(
    parity_text: &str,
    claim_fragments: &[(String, String)],
    out: &mut Vec<Violation>,
) {
    let valid_cells = collect_valid_cells(parity_text);
    for (name, text) in claim_fragments {
        for row in parse_claim_rows(text) {
            let cells_col = row.cells_column();
            if cells_col.is_empty() {
                continue;
            }
            for entry in cells_col.split(',').map(str::trim) {
                if entry.is_empty() || is_no_cell_marker(entry) {
                    continue;
                }
                let normalised = normalise_cell_id(entry);
                if !is_cell_id(&normalised) {
                    out.push(Violation::row(format!(
                        "claims/{name}:{}: [{}] Cells entry '{entry}' is not a SCENE.DIM id",
                        row.line,
                        row.id()
                    )));
                } else if !valid_cells.contains(&normalised) {
                    out.push(Violation::row(format!(
                        "claims/{name}:{}: [{}] Cells entry '{entry}' names a cell not found in the parity ledger",
                        row.line,
                        row.id()
                    )));
                }
            }
        }
    }
}

/// Collects all valid cell ids from parity section headers.
fn collect_valid_cells(parity_text: &str) -> std::collections::HashSet<String> {
    let mut valid_cells = std::collections::HashSet::new();
    for line in parity_text.lines() {
        if line.starts_with("## `STATE_") {
            // e.g. ## `STATE_ROOM` — multiplayer lobby (`CStateRoom`)
            if let Some(scene) = line.split('`').nth(1) {
                let scene = scene.replace("STATE_", "");
                for dim in ["COMP", "INPUT", "TIME", "TRANS", "GAME", "MUSIC", "CHAT"] {
                    valid_cells.insert(format!("{scene}.{dim}"));
                }
            }
        }
        // Side-state bullets; two states can share one bullet.
        if line.starts_with("- `STATE_") {
            for part in line.split('`').filter(|p| p.starts_with("STATE_")) {
                let scene = part.replace("STATE_", "");
                let scene = if scene == "MUSICSHOP" {
                    "MSHOP"
                } else {
                    &scene
                };
                for dim in ["COMP", "INPUT", "TIME", "TRANS", "GAME", "MUSIC", "CHAT"] {
                    valid_cells.insert(format!("{scene}.{dim}"));
                }
            }
        }
        // The Loading gate section has no STATE_ prefix.
        if line.starts_with("## Loading gate") {
            for dim in ["COMP", "INPUT", "TIME", "TRANS", "GAME", "MUSIC", "CHAT"] {
                valid_cells.insert(format!("LOADING.{dim}"));
            }
        }
    }
    valid_cells
}

// -- freshness --

fn check_generated_files_fresh(docs_root: &Path, out: &mut Vec<Violation>) {
    let claims_path = docs_root.join("claims.md");
    let rendered_claims = claims::render_claims(docs_root);
    let on_disk = std::fs::read_to_string(&claims_path).unwrap_or_default();
    if on_disk != rendered_claims {
        out.push(Violation::stale(
            "claims.md is stale — regenerate with `cargo run -p recomp2jam --bin ledger-check -- gen claims`"
                .to_owned(),
        ));
    }
    let parity_path = docs_root.join("parity.md");
    let rendered_index = parity::render_index(docs_root);
    let on_disk = std::fs::read_to_string(&parity_path).unwrap_or_default();
    if on_disk != rendered_index {
        out.push(Violation::stale(
            "parity.md is stale — regenerate with `cargo run -p recomp2jam --bin ledger-check -- gen parity-index`"
                .to_owned(),
        ));
    }
}

/// The claims ledger: hand-kept fragments under `docs/claims/`,
/// rendered into `docs/claims.md` by `gen claims`.
mod claims {
    use super::{parse_claim_rows, Path};

    /// Fragment render order. `00-header.md` is special: it opens the
    /// rendered file; the counts table is generated between it and the
    /// rest.
    pub const HEADER: &str = "00-header.md";
    pub const BODY: [&str; 6] = [
        "10-pairing-family.md",
        "20-rendering.md",
        "30-gameplay.md",
        "40-file-formats.md",
        "50-net.md",
        "60-top-risky.md",
    ];
    pub const TRAILER: &str = "70-maintenance.md";

    pub fn read_fragments(docs_root: &Path) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for name in [HEADER].into_iter().chain(BODY).chain([TRAILER]) {
            let path = docs_root.join("claims").join(name);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            out.push((name.to_owned(), text));
        }
        out
    }

    /// Renders `docs/claims.md` from the fragments plus the generated
    /// counts table.
    pub fn render_claims(docs_root: &Path) -> String {
        let fragments = read_fragments(docs_root);
        render_from_fragments(&fragments)
    }

    pub(crate) fn render_from_fragments(fragments: &[(String, String)]) -> String {
        let header = &fragments[0].1;
        // body = [pairing family, A rows, B rows, C rows, D rows,
        // top-risky, maintenance]; the row tables are body[1..5].
        let body = &fragments[1..];
        let mut rendered = String::new();
        rendered.push_str(header.trim_end());
        rendered.push_str("\n\n## Counts\n\n");
        rendered.push_str(&counts_table(&body[1..5]));
        for (_, text) in body {
            rendered.push('\n');
            rendered.push_str(text.trim_end());
            rendered.push('\n');
        }
        rendered
    }

    /// Counts every claim row by level and renders the summary table.
    fn counts_table(row_fragments: &[(String, String)]) -> String {
        let mut counts = [("EXE", 0usize), ("DATA", 0), ("ASSUMED", 0), ("OPEN", 0)];
        let mut total = 0usize;
        for (_, text) in row_fragments {
            for row in parse_claim_rows(text) {
                total += 1;
                if let Some((_, n)) = counts.iter_mut().find(|(l, _)| *l == row.level()) {
                    *n += 1;
                }
            }
        }
        let mut table = String::from("| Level | Claims |\n|---|---|\n");
        for (level, n) in counts {
            table.push_str(&format!("| {level} | {n} |\n"));
        }
        table.push_str(&format!("| Total | {total} |\n"));
        table
    }
}

/// The parity ledger: per-section files under `docs/parity/`, with the
/// `docs/parity.md` index generated by `gen parity-index`.
mod parity {
    use super::Path;

    /// Section files in ledger order; `00-intro.md` opens the rendered
    /// index.
    pub const INTRO: &str = "00-intro.md";
    pub const SECTIONS: [&str; 20] = [
        "art-delta-guards.md",
        "avi.md",
        "channel.md",
        "room.md",
        "waiting.md",
        "loading-gate.md",
        "playing.md",
        "result.md",
        "side-states.md",
        "shop.md",
        "musicshop.md",
        "myroom.md",
        "planet.md",
        "lanking.md",
        "payment.md",
        "tutorial.md",
        "verify-photo.md",
        "verify-waiting-musicselect.md",
        "verify-wine.md",
        "scoreboard.md",
    ];

    /// `playtest-fixes.md` closes the ledger.
    pub const TRAILER: &str = "playtest-fixes.md";

    /// One section file: file name plus contents.
    pub struct Section {
        pub name: String,
        pub text: String,
    }

    impl Section {
        fn read(docs_root: &Path, name: &str) -> Self {
            let path = docs_root.join("parity").join(name);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            Self {
                name: name.to_owned(),
                text,
            }
        }

        /// The section's first `## ` heading, without the marker.
        fn title(&self) -> &str {
            self.text
                .lines()
                .find_map(|l| l.strip_prefix("## "))
                .unwrap_or(&self.name)
                .trim()
        }
    }

    pub fn read_sections(docs_root: &Path) -> Vec<Section> {
        let mut out = vec![Section::read(docs_root, INTRO)];
        out.extend(SECTIONS.iter().map(|n| Section::read(docs_root, n)));
        out.push(Section::read(docs_root, TRAILER));
        out
    }

    /// Concatenates every section for the row-level lints.
    pub fn concat_sections(sections: &[Section]) -> String {
        sections
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Renders the `docs/parity.md` index: generated banner, the intro
    /// prose inlined, then a linked table of the section files.
    pub fn render_index(docs_root: &Path) -> String {
        let sections = read_sections(docs_root);
        render_from_sections(&sections)
    }

    pub(crate) fn render_from_sections(sections: &[Section]) -> String {
        let mut rendered = String::from(
            "<!-- GENERATED from docs/parity/*.md by `ledger-check gen parity-index` — edit the section files, never this file. -->\n",
        );
        rendered.push_str(sections[0].text.trim_start());
        rendered.push_str("\n## Sections\n\n");
        for section in &sections[1..] {
            rendered.push_str(&format!(
                "- [{}](parity/{})\n",
                section.title(),
                section.name
            ));
        }
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_row_cells_keeps_escaped_pipes_inside_a_cell() {
        let cells =
            split_row_cells("| B16 | `(flags << 16) \\| (id + 1)` | EXE | anchor | G | — |  |");
        assert_eq!(cells.len(), 9);
        assert_eq!(cells[1], "B16");
        assert!(cells[2].contains("\\|"));
    }

    #[test]
    fn parse_claim_rows_reads_id_level_anchor_and_cells() {
        let rows = parse_claim_rows(
            "| ID | Claim | Level | Anchor | Drives | Cells | Sibling |\n|---|---|---|---|---|---|---|\n| A1 | c | EXE | a | R | ROOM.COMP |  |\n",
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id(), "A1");
        assert_eq!(rows[0].level(), "EXE");
        assert_eq!(rows[0].cells_column(), "ROOM.COMP");
    }

    #[test]
    fn claim_row_count_matches_the_committed_counts_table() {
        let docs_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        let fragments = claims::read_fragments(&docs_root);
        let rendered = claims::render_from_fragments(&fragments);
        let total_row = rendered
            .lines()
            .find(|l| l.starts_with("| Total |"))
            .expect("total row");
        let total: usize = total_row.split('|').nth(2).unwrap().trim().parse().unwrap();
        let rows: usize = fragments[2..6]
            .iter()
            .map(|(_, t)| parse_claim_rows(t).len())
            .sum();
        assert_eq!(total, rows, "counts table must sum the claim rows");
    }

    #[test]
    fn rendered_claims_md_is_fresh() {
        let docs_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        let on_disk = std::fs::read_to_string(docs_root.join("claims.md")).expect("claims.md");
        assert_eq!(
            on_disk,
            claims::render_claims(&docs_root),
            "docs/claims.md is stale — run `cargo run -p recomp2jam --bin ledger-check -- gen claims`"
        );
    }

    #[test]
    fn rendered_parity_md_is_fresh() {
        let docs_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        let on_disk = std::fs::read_to_string(docs_root.join("parity.md")).expect("parity.md");
        assert_eq!(
            on_disk,
            parity::render_index(&docs_root),
            "docs/parity.md is stale — run `cargo run -p recomp2jam --bin ledger-check -- gen parity-index`"
        );
    }

    #[test]
    fn parity_index_links_every_section_file() {
        let docs_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        let sections = parity::read_sections(&docs_root);
        let index = parity::render_from_sections(&sections);
        for section in &sections[1..] {
            assert!(
                index.contains(&format!("(parity/{})", section.name)),
                "index misses {}",
                section.name
            );
        }
    }
}
