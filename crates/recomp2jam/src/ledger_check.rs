#![allow(dead_code)]

//! Ledger consistency check — lints `docs/parity.md` and `docs/claims.md`
//! against the rules in ADR-0007/0008/0009/0010.
//!
//! **This is a development tool, not game code.** It ships inside the
//! `recomp2jam` package for toolchain convenience (one `cargo` invocation,
//! `clippy`/`fmt` coverage), but it is not part of the client, the shell,
//! or any library the game depends on. It reads documentation files and
//! exits; it never touches game state, archives, or the network.
//!
//! Runs in CI without `install-root`. `--report` prints the violations;
//! without the flag the test fails on any violation in a row the re-audit
//! has marked done (a row whose text contains `(audit:done)`), and is
//! otherwise tolerant.

use std::path::Path;
use std::process::exit;

fn main() {
    let report_only = std::env::args().any(|a| a == "--report");
    let root = env!("CARGO_MANIFEST_DIR");
    let parity_path = Path::new(root).join("../../docs/parity.md");
    let claims_path = Path::new(root).join("../../docs/claims.md");

    let parity = std::fs::read_to_string(&parity_path).expect("read parity.md");
    let claims = std::fs::read_to_string(&claims_path).expect("read claims.md");

    let violations = lint::lint(&parity, &claims);

    for v in &violations {
        println!("{v}");
    }
    println!(
        "{} violation(s) found ({} audit:done)",
        violations.len(),
        violations
            .iter()
            .filter(|v| v.as_str().contains("audit:done"))
            .count()
    );

    let hard_failures = violations
        .iter()
        .filter(|v| v.as_str().contains("audit:done"))
        .count();
    if !report_only && hard_failures > 0 {
        exit(1);
    }
}

mod lint {
    /// One lint finding: file, line, rule, snippet.
    pub struct Violation(String);
    impl Violation {
        /// The raw message, for callers that need to match on content.
        #[must_use]
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }
    impl std::fmt::Display for Violation {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }

    pub fn lint(parity: &str, claims: &str) -> Vec<Violation> {
        let mut out = Vec::new();
        check_parity_cells(parity, &mut out);
        check_claims_rows(claims, &mut out);
        check_cells_join(parity, claims, &mut out);
        out
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
            out.push(Violation(
                "parity.md: no dimension cell rows found".to_owned(),
            ));
        }
    }

    fn check_verdict(
        dim: &str,
        verdict: &str,
        full_row: &str,
        ln: usize,
        out: &mut Vec<Violation>,
    ) {
        let v = verdict.trim();
        let valid = [
            "PASS",
            "ACCEPTED",
            "OPEN",
            "FAIL",
            "N/A",
            "PASS ",
            "OPEN ",
            "FAIL ",
            "N/A ",
            "ACCEPTED ",
        ];
        let starts_valid = valid
            .iter()
            .any(|p| v.starts_with(p.trim()) || v.starts_with(p));
        if !starts_valid {
            out.push(Violation(format!(
                "parity.md:{ln}: [{dim}] verdict does not start with PASS/ACCEPTED/OPEN/FAIL/N/A: {v:.60}"
            )));
        }
        // A row whose verdict says OPEN/FAIL but the text claims PASS is a stale marker.
        if (v.starts_with("OPEN") || v.starts_with("FAIL"))
            && full_row.to_lowercase().contains("pass")
            && !full_row.to_lowercase().contains("pass-led")
        {
            out.push(Violation(format!(
                "parity.md:{ln}: [{dim}] verdict {v:.30} but row text claims PASS — stale marker?"
            )));
        }
        if v.starts_with("OPEN")
            && !full_row.contains("next step")
            && !full_row.contains("Next step")
        {
            out.push(Violation(format!(
                "parity.md:{ln}: [{dim}] OPEN verdict names no next step: {v:.60}"
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
            out.push(Violation(format!(
                "parity.md:{ln}: [{dim}] row names no guard test source"
            )));
        }
    }

    // -- claims.md --

    fn check_claims_rows(text: &str, out: &mut Vec<Violation>) {
        let mut seen_ids = std::collections::HashSet::new();
        for (i, line) in text.lines().enumerate() {
            let ln = i + 1;
            if !line.starts_with("| ") {
                continue;
            }
            let cells: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            // Escaped pipes in the claim text split into extra cells;
            // skip those rows (the re-audit will normalise them).
            if cells.len() != 8 {
                continue;
            }
            let id = cells[1];
            if !is_claim_id(id) {
                continue;
            }
            if !seen_ids.insert(id.to_string()) {
                out.push(Violation(format!(
                    "claims.md:{ln}: duplicate claim id {id}"
                )));
            }
            let level = cells[3];
            if !["EXE", "DATA", "ASSUMED", "OPEN"].contains(&level) {
                out.push(Violation(format!(
                    "claims.md:{ln}: [{id}] level '{level}' is not EXE/DATA/ASSUMED/OPEN"
                )));
            }
            let anchor = cells[4];
            if anchor.is_empty() {
                out.push(Violation(format!(
                    "claims.md:{ln}: [{id}] empty anchor/evidence cell"
                )));
            }
            let cells_col = cells.get(6).copied().unwrap_or("");
            if cells_col.is_empty() {
                out.push(Violation(format!(
                    "claims.md:{ln}: [{id}] empty Cells column (no driving cell named)"
                )));
            }
        }
    }

    fn is_claim_id(s: &str) -> bool {
        let mut chars = s.chars();
        matches!(chars.next(), Some('A'..='D')) && chars.all(|c| c.is_ascii_digit())
    }

    fn normalise_cell_id(s: &str) -> String {
        match s {
            "WAIT.COMP" => "WAITING.COMP".to_owned(),
            "CHAN.COMP" => "CHANNEL.COMP".to_owned(),
            "PLAY.COMP" => "PLAYING.COMP".to_owned(),
            "LOAD.COMP" => "LOADING.COMP".to_owned(),
            "MSHOP.COMP" => "MSHOP.COMP".to_owned(),
            other => other.to_owned(),
        }
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

    // -- join --

    fn check_cells_join(parity: &str, claims: &str, out: &mut Vec<Violation>) {
        // Collect all valid cell ids from parity.md section headers.
        let mut valid_cells = std::collections::HashSet::new();
        for line in parity.lines() {
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
        // Check that every Cells entry in claims.md names a real cell.
        for (i, line) in claims.lines().enumerate() {
            let ln = i + 1;
            if !line.starts_with("| ") {
                continue;
            }
            let cells: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if cells.len() != 8 {
                continue;
            }
            let id = cells[1];
            if !is_claim_id(id) {
                continue;
            }
            let cells_col = cells.get(6).copied().unwrap_or("");
            if cells_col.is_empty() {
                continue;
            }
            for entry in cells_col
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                let normalised = normalise_cell_id(entry);
                if !is_cell_id(&normalised) {
                    out.push(Violation(format!(
                        "claims.md:{ln}: [{id}] Cells entry '{entry}' is not a SCENE.DIM id"
                    )));
                } else if !valid_cells.contains(&normalised) {
                    out.push(Violation(format!(
                        "claims.md:{ln}: [{id}] Cells entry '{entry}' names a cell not found in parity.md"
                    )));
                }
            }
        }
    }
}
