//! Rect query: native coords for a BND control id, resolved from
//! staged placements (control-list entry paired with its `.bnd`
//! rect by code ordinal).

use anyhow::{Context, Result};
use o2jam_file::{
    decode_control_text, find_opi_entry, opi_entry_body, pair_entries_with_rects, parse_bnd,
    parse_control_list, parse_opi, ControlBlock, ControlList, PlacedControl,
};
use std::path::Path;

/// Native coords resolved for one control id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRect {
    /// Block that owns the control (`STATE_*` / `DIALOG_*`).
    pub block: String,
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
    /// Raw `.bnd` corners, for cross-checking.
    pub x1: u32,
    pub y1: u32,
    pub x2: u32,
    pub y2: u32,
}

/// Resolve `id` to its native rect inside `archive`.
///
/// # Errors
///
/// Returns an error when the archive cannot be parsed, no control
/// list carries `id`, or its block has no `.bnd` to pair.
pub fn query_control_rect(archive: &[u8], id: &str) -> Result<ControlRect> {
    let entries = parse_opi(archive).context("parsing archive")?;
    for list_name in control_list_names(&entries) {
        let list_entry =
            find_opi_entry(&entries, &list_name).context("control list entry missing")?;
        let body = opi_entry_body(archive, list_entry).context("reading control list")?;
        let text = decode_control_text(body).context("decoding control list")?;
        let list = parse_control_list(&text).context("parsing control list")?;
        if let Some(found) = search_list(&entries, archive, &list, id)? {
            return Ok(found);
        }
    }
    anyhow::bail!("unknown control id: {id}")
}

/// Names of every `ControlList_*.txt` entry in the archive.
fn control_list_names(entries: &[o2jam_file::OpiEntry]) -> Vec<String> {
    let mut names: Vec<String> = entries
        .iter()
        .map(|entry| entry.name.clone())
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            lower.starts_with("controllist_") && lower.ends_with(".txt")
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Search one parsed control list for `id`; return its rect when found.
fn search_list(
    entries: &[o2jam_file::OpiEntry],
    archive: &[u8],
    list: &ControlList,
    id: &str,
) -> Result<Option<ControlRect>> {
    for block in &list.blocks {
        if !block.all_entries().iter().any(|entry| entry.id == id) {
            continue;
        }
        let Some(bind_name) = block.bound_file.as_ref() else {
            anyhow::bail!("control {id} has no BOUND file");
        };
        let bind_entry = find_opi_entry(entries, bind_name)
            .with_context(|| format!("missing bind file {bind_name}"))?;
        let bind_body = opi_entry_body(archive, bind_entry)
            .with_context(|| format!("reading bind file {bind_name}"))?;
        let bind = parse_bnd(bind_body).context("parsing bind file")?;
        for placed in pair_entries_with_rects(block, &bind) {
            if placed.entry.id != id {
                continue;
            }
            return Ok(Some(rect_from_placed(block, placed)));
        }
    }
    Ok(None)
}

/// Convert a placed control's `.bnd` rect to native coords.
fn rect_from_placed(block: &ControlBlock, placed: PlacedControl<'_>) -> ControlRect {
    let corners = placed.rect.expect("pairing always yields a rect");
    ControlRect {
        block: block.name.clone(),
        x: corners.x1.min(corners.x2) as i32,
        y: corners.y1.min(corners.y2) as i32,
        w: corners.x2.saturating_sub(corners.x1) as i32,
        h: corners.y2.saturating_sub(corners.y1) as i32,
        x1: corners.x1,
        y1: corners.y1,
        x2: corners.x2,
        y2: corners.y2,
    }
}

/// Read an archive from disk.
pub fn read_archive(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("reading {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use o2jam_file::test_archive;

    fn bnd_bytes() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&10u32.to_le_bytes());
        out.extend_from_slice(&20u32.to_le_bytes());
        out.extend_from_slice(&30u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out
    }

    fn control_text() -> &'static str {
        "NUMBER_OF_STATE 0x1\nSTATE_TEST 0x1\nBOUND \"Test.bnd\"\nIDC_CHROME_FOO 0x00000001 \"Foo.ojs\"\n"
    }

    #[test]
    fn resolves_known_control_rect() {
        let archive = test_archive::build_archive(&[
            ("ControlList_Interface.txt", control_text().as_bytes()),
            ("Test.bnd", &bnd_bytes()),
        ]);
        let rect = query_control_rect(&archive, "IDC_CHROME_FOO").unwrap();
        assert_eq!(rect.block, "STATE_TEST");
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (10, 20, 20, 20));
        assert_eq!((rect.x1, rect.y1, rect.x2, rect.y2), (10, 20, 30, 40));
    }

    #[test]
    fn unknown_control_fails_loudly() {
        let archive = test_archive::build_archive(&[
            ("ControlList_Interface.txt", control_text().as_bytes()),
            ("Test.bnd", &bnd_bytes()),
        ]);
        assert!(query_control_rect(&archive, "IDC_MISSING").is_err());
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn no_coords_on_unknown() {
        let archive = test_archive::build_archive(&[
            ("ControlList_Interface.txt", control_text().as_bytes()),
            ("Test.bnd", &bnd_bytes()),
        ]);
        // Unknown id must error before any printing; function returns Err.
        let result = query_control_rect(&archive, "IDC_MISSING");
        assert!(result.is_err());
    }
}
