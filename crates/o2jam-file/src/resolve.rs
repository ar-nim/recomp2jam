//! Data-driven scene composition (7.55 client recipe, no drawing).
//!
//! Pure helpers over the verified parsers for `docs/opi-pattern.md`
//! steps 4–6, 8–9: the `BOUND` stem comes from the block itself
//! (never a hardcoded bind list), `.bnd` rect `i` lays out block
//! entry `i + 1` in file order, sprite names resolve against the
//! same archive case-insensitively, and the `O2_SBS_*` / `_`-prefix
//! rules apply before reporting a sprite missing. Step 7 (blit)
//! stays caller-side: this module only pairs entries with rects
//! and classifies sprite references.

use super::{Bnd, BndRect, ControlBlock, ControlEntry, FileError, OpiEntry};

/// One laid-out control: a block entry past self plus its `.bnd` rect, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedControl<'a> {
    /// Block entry (`all_entries()[i + 1]`).
    pub entry: &'a ControlEntry,
    /// `.bnd` rect `i`, when the bind section covers this entry.
    pub rect: Option<&'a BndRect>,
}

/// How one entry's quoted sprite resolves (recipe steps 6 + 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteResolution<'a> {
    /// Entry names no sprite: text, edits, and bare slots draw no art.
    NoArt,
    /// `O2_SBS_VERT`/`O2_SBS_HORZ` with numeric args: scrollbar
    /// parameters, never files.
    ScrollbarParams,
    /// Leading `_` marks runtime resolution (numbered/`.oja`
    /// siblings live beside the bare stem); not an archive miss.
    RuntimeDeferred {
        /// Quoted name as written in the block.
        wanted: &'a str,
    },
    /// Archive entry the sprite resolves to (exact, else ASCII
    /// case-insensitive: three Interface stems differ only by case).
    Found(&'a OpiEntry),
    /// Named file with no archive entry after both rules pass.
    Missing {
        /// Quoted name as written in the block.
        wanted: &'a str,
    },
}

/// Find an archive entry by name: exact match first, then one
/// ASCII-case-insensitive pass (step 6).
#[must_use]
pub fn find_opi_entry<'a>(directory: &'a [OpiEntry], wanted: &str) -> Option<&'a OpiEntry> {
    directory
        .iter()
        .find(|entry| entry.name == wanted)
        .or_else(|| {
            directory
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(wanted))
        })
}

/// Resolve the block's `BOUND "stem.bnd"` line against the same
/// archive, case-insensitively (step 4). No hardcoded bind names:
/// `None` means a bare `BOUND` line or an unresolvable stem.
#[must_use]
pub fn resolve_bound_entry<'a>(
    block: &ControlBlock,
    directory: &'a [OpiEntry],
) -> Option<&'a OpiEntry> {
    let wanted = block.bound_file.as_deref()?;
    find_opi_entry(directory, wanted)
}

/// Slice an archive body by `size`, never `reserved_size` (the
/// patcher keeps stale allocation bytes past `size`).
///
/// # Errors
///
/// `FileError::Truncated` when the `size` slice overruns `archive`.
pub fn opi_entry_body<'a>(archive: &'a [u8], entry: &OpiEntry) -> Result<&'a [u8], FileError> {
    let end = entry
        .data_offset
        .checked_add(entry.size as usize)
        .ok_or(FileError::Truncated {
            need: usize::MAX,
            have: archive.len(),
        })?;
    archive
        .get(entry.data_offset..end)
        .ok_or(FileError::Truncated {
            need: end,
            have: archive.len(),
        })
}

/// Pair `.bnd` rect `i` with block entry `i + 1` in file order
/// (step 5). Entry 0 names the block and takes no rect; `SET`
/// members count because [`ControlBlock::all_entries`] already
/// yields file order. Extra entries keep a `None` rect (the
/// `State_Playing` tail rides unloaded); extra rects are ignored.
/// File order is the paint order: the entry-1 background lands
/// first and later entries cover it (step 8).
#[must_use]
pub fn pair_entries_with_rects<'a>(
    block: &'a ControlBlock,
    bind: &'a Bnd,
) -> Vec<PlacedControl<'a>> {
    block
        .all_entries()
        .into_iter()
        .skip(1)
        .enumerate()
        .map(|(index, entry)| PlacedControl {
            entry,
            rect: bind.rects.get(index),
        })
        .collect()
}

/// Classify one entry's sprite reference without touching the
/// archive (steps 6 + 9): no art, scrollbar params, runtime
/// `_`-prefix, else a named file needing a directory lookup.
#[must_use]
pub fn classify_sprite(entry: &ControlEntry) -> SpriteResolution<'_> {
    let Some(wanted) = entry.sprite.as_deref() else {
        return SpriteResolution::NoArt;
    };
    if wanted.starts_with("O2_SBS_") {
        return SpriteResolution::ScrollbarParams;
    }
    if wanted.starts_with('_') {
        return SpriteResolution::RuntimeDeferred { wanted };
    }
    SpriteResolution::Missing { wanted }
}

/// Resolve one entry's sprite against the same archive (steps 6 +
/// 9): special names first, then a case-insensitive lookup.
/// Entries without a sprite draw no art; they are never missing.
#[must_use]
pub fn resolve_sprite_entry<'a>(
    entry: &'a ControlEntry,
    directory: &'a [OpiEntry],
) -> SpriteResolution<'a> {
    match classify_sprite(entry) {
        SpriteResolution::Missing { wanted } => match find_opi_entry(directory, wanted) {
            Some(found) => SpriteResolution::Found(found),
            None => SpriteResolution::Missing { wanted },
        },
        settled => settled,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{parse_bnd, parse_control_list};
    use super::*;

    const FIXTURE_LIST: &str = "\
NUMBER_OF_STATE 0x01\n\
NUMBER_OF_DIALOG 0x01\n\
STATE_ROOM 0x02\n\
{\n\
\tBOUND\t\t\t\t\"State_Room.bnd\"\n\
\tIDC_STATE_ROOM\t0X00200003\n\
\tIDC_BACK_SROOM_1\t0X10200001\t\"Back.ojs\"\n\
\tIDC_OK\t0X20200002\t\"Ok.ojs\"\n\
\tSET 0x00000001 // count\n\
\t{\n\
\t\tIDC_FRAME\t0X10200004\t\"frame.ojs\"\n\
\t}\n\
}\n\
";

    fn fixture_block() -> ControlBlock {
        parse_control_list(FIXTURE_LIST)
            .unwrap()
            .blocks
            .pop()
            .unwrap()
    }

    fn fixture_directory() -> Vec<OpiEntry> {
        for_entry(["State_Room.bnd", "Back.ojs", "ok.ojs", "frame.ojs"])
    }

    fn for_entry<const N: usize>(names: [&str; N]) -> Vec<OpiEntry> {
        names
            .into_iter()
            .enumerate()
            .map(|(index, name)| OpiEntry {
                name: name.to_string(),
                kind: 1,
                data_offset: index * 10,
                size: 4,
                reserved_size: 8,
            })
            .collect()
    }

    fn fixture_bind() -> Bnd {
        let mut blob = vec![0xFF; 4];
        blob.extend_from_slice(&3u16.to_le_bytes());
        for corners in [
            (0u32, 0u32, 800u32, 600u32),
            (10, 10, 100, 60),
            (200, 200, 300, 240),
        ] {
            for edge in [corners.0, corners.1, corners.2, corners.3] {
                blob.extend_from_slice(&edge.to_le_bytes());
            }
        }
        parse_bnd(&blob).unwrap()
    }

    #[test]
    fn bound_resolves_from_block_stem_without_hardcoded_names() {
        let block = fixture_block();
        let directory = for_entry(["state_room.BND"]);
        assert_eq!(
            resolve_bound_entry(&block, &directory).unwrap().name,
            "state_room.BND"
        );
    }

    #[test]
    fn bound_returns_none_for_bare_bound_line() {
        let mut block = fixture_block();
        block.bound_file = None;
        assert!(resolve_bound_entry(&block, &fixture_directory()).is_none());
    }

    #[test]
    fn bound_returns_none_when_stem_has_no_entry() {
        let block = fixture_block();
        assert!(resolve_bound_entry(&block, &for_entry(["Other.bnd"])).is_none());
    }

    #[test]
    fn pairing_maps_rect_index_to_entry_index_plus_one() {
        let block = fixture_block();
        let bind = fixture_bind();
        let placed = pair_entries_with_rects(&block, &bind);
        let ids: Vec<&str> = placed.iter().map(|pair| pair.entry.id.as_str()).collect();
        assert_eq!(ids, ["IDC_BACK_SROOM_1", "IDC_OK", "IDC_FRAME"]);
        assert_eq!(placed[0].rect.unwrap().x2, 800);
        assert_eq!(placed[1].rect.unwrap().x1, 10);
        assert_eq!(placed[2].rect.unwrap().x1, 200);
    }

    #[test]
    fn pairing_keeps_set_members_in_file_order_with_background_first() {
        let block = fixture_block();
        let bind = fixture_bind();
        let placed = pair_entries_with_rects(&block, &bind);
        assert_eq!(placed.first().unwrap().entry.id, "IDC_BACK_SROOM_1");
        assert_eq!(placed.last().unwrap().entry.id, "IDC_FRAME");
    }

    #[test]
    fn pairing_leaves_uncovered_tail_entries_rectless() {
        let short = parse_bnd(&[
            0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
        ])
        .unwrap();
        let block = fixture_block();
        let placed = pair_entries_with_rects(&block, &short);
        assert!(placed[0].rect.is_some());
        assert!(placed[1].rect.is_none());
        assert!(placed[2].rect.is_none());
    }

    #[test]
    fn lookup_prefers_exact_spelling_over_case_fallback() {
        let directory = for_entry(["Main_level.ojs", "main_level.ojs"]);
        assert_eq!(
            find_opi_entry(&directory, "main_level.ojs")
                .unwrap()
                .data_offset,
            10
        );
        assert_eq!(
            find_opi_entry(&directory, "MAIN_LEVEL.OJS")
                .unwrap()
                .data_offset,
            0
        );
        assert!(find_opi_entry(&directory, "Absent.ojs").is_none());
    }

    #[test]
    fn entry_without_sprite_draws_no_art() {
        let block = fixture_block();
        let directory = fixture_directory();
        let layout = block.all_entries();
        assert_eq!(
            resolve_sprite_entry(layout[1], &directory),
            SpriteResolution::Found(&directory[1])
        );
        assert_eq!(
            resolve_sprite_entry(
                &ControlEntry {
                    id: "IDC_TEXT".to_string(),
                    code: 1,
                    sprite: None,
                },
                &fixture_directory()
            ),
            SpriteResolution::NoArt
        );
    }

    #[test]
    fn scrollbar_params_never_hit_the_archive() {
        let entry = ControlEntry {
            id: "IDC_SCROLL".to_string(),
            code: 2,
            sprite: Some("O2_SBS_VERT\t4\t\t100".to_string()),
        };
        let directory = for_entry(["O2_SBS_VERT\t4\t\t100"]);
        assert_eq!(
            resolve_sprite_entry(&entry, &directory),
            SpriteResolution::ScrollbarParams
        );
    }

    #[test]
    fn underscore_prefix_defers_before_missing() {
        let entry = ControlEntry {
            id: "IDC_BG".to_string(),
            code: 3,
            sprite: Some("_Playing_BG.ojs".to_string()),
        };
        let directory = for_entry(["Playing_BG.ojs"]);
        assert_eq!(
            resolve_sprite_entry(&entry, &directory),
            SpriteResolution::RuntimeDeferred {
                wanted: "_Playing_BG.ojs"
            }
        );
    }

    #[test]
    fn named_sprite_resolves_case_insensitively() {
        let entry = ControlEntry {
            id: "IDC_OK".to_string(),
            code: 4,
            sprite: Some("OK.OJS".to_string()),
        };
        assert!(matches!(
            resolve_sprite_entry(&entry, &fixture_directory()),
            SpriteResolution::Found(_)
        ));
    }

    #[test]
    fn named_sprite_missing_reports_wanted_name() {
        let entry = ControlEntry {
            id: "IDC_OK".to_string(),
            code: 4,
            sprite: Some("Absent.ojs".to_string()),
        };
        assert_eq!(
            resolve_sprite_entry(&entry, &fixture_directory()),
            SpriteResolution::Missing {
                wanted: "Absent.ojs"
            }
        );
    }

    #[test]
    fn body_slices_size_not_reserved_size() {
        let archive = b"abcdefghij";
        let entry = OpiEntry {
            name: "a.ojs".to_string(),
            kind: 1,
            data_offset: 2,
            size: 3,
            reserved_size: 8,
        };
        assert_eq!(opi_entry_body(archive, &entry).unwrap(), b"cde");
        let past_end = OpiEntry { size: 99, ..entry };
        assert!(matches!(
            opi_entry_body(archive, &past_end),
            Err(FileError::Truncated { .. })
        ));
    }
}
