//! Data-driven scene composition (7.55 client recipe, no drawing).
//!
//! Pure helpers over the verified parsers for `docs/opi-pattern.md`
//! steps 4–6, 8–9: the `BOUND` stem comes from the block itself
//! (never a hardcoded bind list), each entry pairs with the `.bnd`
//! rect its code selects — ordinal `(code & 0xFFF) − 1`, the
//! all-zero rect on a miss (exe `0x548640`, dossier
//! `ev_3a869bb7…`) — sprite names resolve against the same archive
//! case-insensitively, and the `O2_SBS_*` / `_`-prefix rules apply
//! before reporting a sprite missing (`_` names build their
//! stage-numbered sibling from the room arena stage,
//! `docs/stage-bga.md`). Step 7 (blit) stays
//! caller-side: this module only pairs entries with rects and
//! classifies sprite references.

use super::{Bnd, BndRect, ControlBlock, ControlEntry, FileError, OpiEntry};

/// Mask isolating a control code's `.bnd` rect ordinal: the exe
/// pairs by the low 12 bits (`0x548640`, ev `3a869bb7…`).
const RECT_ORDINAL_MASK: u32 = 0xFFF;

/// The exe's out-of-range rect: four zero dwords, copied verbatim
/// when an ordinal misses the table (`0x548640`, ev `3a869bb7…`).
static ZERO_RECT: BndRect = BndRect {
    x1: 0,
    y1: 0,
    x2: 0,
    y2: 0,
};

/// Lowest room stage: the waiting-room picker returns 1..=36 and
/// room entry replaces a zero value with a real stage
/// (`docs/stage-bga.md`, store `0x4aace5`).
const STAGE_MIN: u16 = 1;

/// Highest room stage: 36 numbered variants per stem
/// (`Playing_BG1..36.ojs`, Playing1.opi slots 55–90).
const STAGE_MAX: u16 = 36;

/// Length of the 4-char sprite extension the `_` runtime rule
/// drops together with the `_` (strip length strlen − 5,
/// `0x4e80ef`).
const EXTENSION_LEN: usize = 4;

/// One laid-out control: a block entry plus the `.bnd` rect its
/// code selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedControl<'a> {
    /// Block entry in file order (`ControlBlock::all_entries`).
    pub entry: &'a ControlEntry,
    /// The rect named by `(entry.code & 0xFFF) − 1`, else the
    /// all-zero rect on a miss. Always `Some`: the exe stores a
    /// rect for every entry, it never skips one.
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
    /// [`resolve_sprite_entry`] refines this arm through the room
    /// stage ([`resolve_stage_sprite`]); without a stage — or for
    /// stems whose exe arm needs more (variant/bare) — it stays
    /// deferred.
    RuntimeDeferred {
        /// Quoted name as written in the block.
        wanted: &'a str,
    },
    /// Archive entry the sprite resolves to (exact, else ASCII
    /// case-insensitive: three Interface stems differ only by case).
    Found(&'a OpiEntry),
    /// Named file with no archive entry after both rules pass.
    /// A stage-resolved `_` name that misses reports the reference
    /// as written in the block.
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

/// Pair every block entry with the `.bnd` rect its code selects
/// (step 5): ordinal = `(entry.code & 0xFFF) − 1`, bounds-checked
/// against the rect count, miss → the all-zero rect, never a skip
/// (exe pairing step VA `0x548640`, dossier `ev_3a869bb7…`). File
/// position plays no role in the choice; the result keeps file
/// order because paint order is file order (step 8). Entry 0 (the
/// block-self entry) pairs by its own code like any entry.
#[must_use]
pub fn pair_entries_with_rects<'a>(
    block: &'a ControlBlock,
    bind: &'a Bnd,
) -> Vec<PlacedControl<'a>> {
    block
        .all_entries()
        .into_iter()
        .map(|entry| PlacedControl {
            entry,
            rect: Some(selected_rect(entry, bind)),
        })
        .collect()
}

/// The rect the exe copies for one entry: its code low word minus
/// one selects the record; an out-of-range or zero low word falls
/// back to the all-zero rect.
fn selected_rect<'a>(entry: &ControlEntry, bind: &'a Bnd) -> &'a BndRect {
    rect_ordinal(entry)
        .and_then(|ordinal| bind.rects.get(ordinal))
        .unwrap_or(&ZERO_RECT)
}

/// Rect ordinal for one entry's control code (`0x548640`, ev
/// `3a869bb7…`): low word minus one; `None` when the low word is
/// zero — the exe's wrapped `− 1` then misses the bounds check.
fn rect_ordinal(entry: &ControlEntry) -> Option<usize> {
    let ordinal = entry.code & RECT_ORDINAL_MASK;
    let ordinal = ordinal.checked_sub(1)?;
    usize::try_from(ordinal).ok()
}

/// Classify one entry's sprite reference without touching the
/// archive (steps 6 + 9): no art, scrollbar params, runtime
/// `_`-prefix, else a named file needing a directory lookup. The
/// `_` arm stays deferred here — classification sees no stage;
/// [`resolve_sprite_entry`] refines it.
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

/// Build the stage-numbered archive name for one `_` runtime
/// sprite reference (applier `0x4e7850`, `docs/stage-bga.md`):
/// drop the `_` and the 4-char extension (`_Playing_BG.ojs` →
/// `Playing_BG`; `strncpy` length strlen − 5, `0x4e80ef`), clamp
/// the room stage into 1..=36, and rebuild with the original
/// extension → `Playing_BG7.ojs`. A reference without the full
/// `_` + stem + 4-char-extension shape comes back unchanged: the
/// exe names no numbered sibling for it either.
#[must_use]
pub fn resolve_stage_sprite(arena_stage: u16, stem: &str) -> String {
    let Some((core, extension)) = runtime_stem(stem) else {
        return stem.to_string();
    };
    let stage = arena_stage.clamp(STAGE_MIN, STAGE_MAX);
    format!("{core}{stage}{extension}")
}

/// The exe's post-strip parts of one `_` runtime reference: the
/// stem between the `_` and the trailing 4-char extension
/// (`_Playing_BG.ojs` → `("Playing_BG", ".ojs")`; strip length
/// strlen − 5 at `0x4e80ef`). `None` without the full shape — `_`
/// plus at least one stem character — or when a cut would split a
/// multi-byte character (exe names are ASCII; a split names no
/// archive file).
fn runtime_stem(wanted: &str) -> Option<(&str, &str)> {
    // `_` plus a bare 4-char extension (5 bytes) strips to an
    // empty stem: the exe's `strlen − 5` then names nothing.
    let stem_end = wanted.len().checked_sub(EXTENSION_LEN)?;
    if stem_end < 2 {
        return None;
    }
    // `get` rejects cuts inside a multi-byte character.
    Some((wanted.get(1..stem_end)?, wanted.get(stem_end..)?))
}

/// True when this runtime stem's in-song arm appends the stage
/// number. The exe classifies stems by exact case-insensitive
/// equality (`_stricmp` wrapper `0x5f34ff`); two arms take more
/// than the stage and stay deferred here (`docs/stage-bga.md`
/// table): `Note_Click` builds `%s%d_%d.ojt` from a random
/// variant, `Note_LongCombo`/`Note_LongComboNum` stay bare
/// (`%s.ojt`).
fn is_stage_numbered_stem(stem: &str) -> bool {
    !(stem.eq_ignore_ascii_case("Note_Click")
        || stem.eq_ignore_ascii_case("Note_LongCombo")
        || stem.eq_ignore_ascii_case("Note_LongComboNum"))
}

/// Resolve one entry's sprite against the same archive (steps 6 +
/// 9): special names first, then a case-insensitive lookup.
/// Entries without a sprite draw no art; they are never missing.
/// A `_` runtime reference resolves through the room arena stage
/// when the caller passes one (applier `0x4e7850`,
/// [`resolve_stage_sprite`]): the numbered name is looked up like
/// any other and a miss reports the written reference. Without a
/// stage — UI-preview contexts, the exe's bare-name arm — or for
/// stems whose arm needs more than the stage, the entry stays
/// [`SpriteResolution::RuntimeDeferred`].
#[must_use]
pub fn resolve_sprite_entry<'a>(
    entry: &'a ControlEntry,
    directory: &'a [OpiEntry],
    arena_stage: Option<u16>,
) -> SpriteResolution<'a> {
    match classify_sprite(entry) {
        SpriteResolution::RuntimeDeferred { wanted } => {
            resolve_runtime_entry(wanted, arena_stage, directory)
        }
        SpriteResolution::Missing { wanted } => match find_opi_entry(directory, wanted) {
            Some(found) => SpriteResolution::Found(found),
            None => SpriteResolution::Missing { wanted },
        },
        settled => settled,
    }
}

/// The `_` runtime arm of [`resolve_sprite_entry`] (applier
/// `0x4e7850`): build the stage-numbered sibling and look it up.
fn resolve_runtime_entry<'a>(
    wanted: &'a str,
    arena_stage: Option<u16>,
    directory: &'a [OpiEntry],
) -> SpriteResolution<'a> {
    let Some(stage) = arena_stage else {
        return SpriteResolution::RuntimeDeferred { wanted };
    };
    let Some((stem, _extension)) = runtime_stem(wanted) else {
        return SpriteResolution::RuntimeDeferred { wanted };
    };
    if !is_stage_numbered_stem(stem) {
        return SpriteResolution::RuntimeDeferred { wanted };
    }
    let numbered = resolve_stage_sprite(stage, wanted);
    match find_opi_entry(directory, &numbered) {
        Some(found) => SpriteResolution::Found(found),
        None => SpriteResolution::Missing { wanted },
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
\tIDC_PANE\t0X10200000\n\
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

    fn rect(x1: u32, y1: u32, x2: u32, y2: u32) -> BndRect {
        BndRect { x1, y1, x2, y2 }
    }

    fn all_zero_rect() -> BndRect {
        rect(0, 0, 0, 0)
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
    fn pairing_selects_rects_by_code_low_word_minus_one() {
        let block = fixture_block();
        let bind = fixture_bind();
        let placed = pair_entries_with_rects(&block, &bind);
        assert_eq!(placed[0].entry.id, "IDC_STATE_ROOM");
        assert_eq!(placed[0].rect, Some(&rect(200, 200, 300, 240))); // low word 3 → ordinal 2
        assert_eq!(placed[1].rect, Some(&rect(0, 0, 800, 600))); // low word 1 → ordinal 0
        assert_eq!(placed[2].rect, Some(&rect(10, 10, 100, 60))); // low word 2 → ordinal 1
    }

    #[test]
    fn pairing_ignores_file_position_when_choosing_a_rect() {
        let block = fixture_block();
        let bind = fixture_bind();
        let placed = pair_entries_with_rects(&block, &bind);
        // The file-order model gave the first entry nothing and the
        // last (SET-child) entry rect 2; by code, rect 2 belongs to
        // the first entry's low word 3 and the last entry misses.
        assert_eq!(
            placed.first().unwrap().rect,
            Some(&rect(200, 200, 300, 240))
        );
        assert_eq!(placed.last().unwrap().rect, Some(&all_zero_rect()));
    }

    #[test]
    fn pairing_gives_missed_ordinals_the_all_zero_rect() {
        let block = fixture_block();
        let bind = fixture_bind();
        let placed = pair_entries_with_rects(&block, &bind);
        // SET child `IDC_FRAME`: low word 4 → ordinal 3, just past
        // the 3-rect table; the exe stores a rect anyway.
        assert_eq!(placed.last().unwrap().entry.id, "IDC_FRAME");
        assert_eq!(placed.last().unwrap().rect, Some(&all_zero_rect()));
    }

    #[test]
    fn pairing_maps_zero_low_word_to_the_all_zero_rect() {
        let block = fixture_block();
        let bind = fixture_bind();
        let placed = pair_entries_with_rects(&block, &bind);
        let pane = placed
            .iter()
            .find(|placed| placed.entry.id == "IDC_PANE")
            .unwrap();
        assert_eq!(pane.rect, Some(&all_zero_rect()));
    }

    #[test]
    fn pairing_gives_entries_past_the_table_the_all_zero_rect() {
        let short = parse_bnd(&[
            0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
        ])
        .unwrap();
        let block = fixture_block();
        let placed = pair_entries_with_rects(&block, &short);
        assert_eq!(placed[1].rect, Some(&rect(0, 0, 1, 1))); // low word 1 fits
        let zero = all_zero_rect();
        assert!(
            placed[0].rect == Some(&zero)
                && placed[2..].iter().all(|placed| placed.rect == Some(&zero))
        );
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
            resolve_sprite_entry(layout[1], &directory, None),
            SpriteResolution::Found(&directory[1])
        );
        assert_eq!(
            resolve_sprite_entry(
                &ControlEntry {
                    id: "IDC_TEXT".to_string(),
                    code: 1,
                    sprite: None,
                },
                &fixture_directory(),
                None
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
            resolve_sprite_entry(&entry, &directory, None),
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
            resolve_sprite_entry(&entry, &directory, None),
            SpriteResolution::RuntimeDeferred {
                wanted: "_Playing_BG.ojs"
            }
        );
    }

    #[test]
    fn stage_sprite_names_the_first_backdrop_for_stage_one() {
        assert_eq!(
            resolve_stage_sprite(1, "_Playing_BG.ojs"),
            "Playing_BG1.ojs"
        );
    }

    #[test]
    fn stage_sprite_names_the_last_backdrop_for_stage_thirty_six() {
        assert_eq!(
            resolve_stage_sprite(36, "_Playing_BG.ojs"),
            "Playing_BG36.ojs"
        );
    }

    #[test]
    fn stage_sprite_clamps_out_of_range_stages_into_the_exe_range() {
        assert_eq!(
            resolve_stage_sprite(0, "_Playing_BG.ojs"),
            "Playing_BG1.ojs"
        );
        assert_eq!(
            resolve_stage_sprite(37, "_Playing_BG.ojs"),
            "Playing_BG36.ojs"
        );
    }

    #[test]
    fn stage_sprite_keeps_non_ojs_extensions() {
        assert_eq!(resolve_stage_sprite(2, "_Note_Miss.ojt"), "Note_Miss2.ojt");
    }

    #[test]
    fn stage_prefix_resolves_through_the_stage_number_instead_of_deferring() {
        let entry = ControlEntry {
            id: "IDC_BG".to_string(),
            code: 3,
            sprite: Some("_Playing_BG.ojs".to_string()),
        };
        let directory = for_entry(["Playing_BG7.ojs"]);
        assert!(matches!(
            resolve_sprite_entry(&entry, &directory, Some(7)),
            SpriteResolution::Found(_)
        ));
    }

    #[test]
    fn stage_resolved_sprite_missing_reports_the_written_reference() {
        let entry = ControlEntry {
            id: "IDC_BG".to_string(),
            code: 3,
            sprite: Some("_Playing_BG.ojs".to_string()),
        };
        // Only stage 1 is present: stage 7 must miss, not find it.
        let directory = for_entry(["Playing_BG1.ojs"]);
        assert_eq!(
            resolve_sprite_entry(&entry, &directory, Some(7)),
            SpriteResolution::Missing {
                wanted: "_Playing_BG.ojs"
            }
        );
    }

    #[test]
    fn stage_arm_stems_needing_more_than_the_stage_stay_deferred() {
        let entry = ControlEntry {
            id: "IDC_CLICK".to_string(),
            code: 3,
            sprite: Some("_Note_Click.ojs".to_string()),
        };
        assert_eq!(
            resolve_sprite_entry(&entry, &[], Some(7)),
            SpriteResolution::RuntimeDeferred {
                wanted: "_Note_Click.ojs"
            }
        );
    }

    #[test]
    fn unshaped_underscore_reference_stays_deferred_even_with_a_stage() {
        let entry = ControlEntry {
            id: "IDC_ODD".to_string(),
            code: 3,
            sprite: Some("_Ab".to_string()),
        };
        assert_eq!(
            resolve_sprite_entry(&entry, &[], Some(7)),
            SpriteResolution::RuntimeDeferred { wanted: "_Ab" }
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
            resolve_sprite_entry(&entry, &fixture_directory(), None),
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
            resolve_sprite_entry(&entry, &fixture_directory(), None),
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
