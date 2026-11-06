//! The client key-name string table and its filter/letter lookups.

use crate::{decode_control_text, unknown_format, FileError};

/// Key-name table entries in the 7.55 client
/// (exe file `0x2454d8..0x2455dc`).
pub const KEY_NAME_COUNT: usize = 65;

/// Bytes per key-name table entry (one `u32` string VA).
pub const KEY_NAME_ENTRY_LEN: usize = 4;

/// Resolve one key-name VA into a pool offset.
fn va_offset(va: u32, pool_base: u32, have: usize) -> Result<usize, FileError> {
    let relative = va.checked_sub(pool_base).ok_or(FileError::UnknownFormat {
        magic: va.to_le_bytes().to_vec(),
    })?;
    let off = usize::try_from(relative).map_err(|_| FileError::Truncated {
        need: usize::MAX,
        have,
    })?;
    if off >= have {
        return Err(FileError::Truncated {
            need: off + 1,
            have,
        });
    }
    Ok(off)
}

/// VA of the key-name table base (entry 0, `ㄱ`).
///
/// The `+0x680` keyed rebuild indexes this base by search arm
/// (`mov 0x6454d8(,%edx,4),%eax` at file `0x448c2`, second loop at
/// file `0x44ecc`): arms 0..=13 select the fourteen chosung initials
/// (entries 0..=13, `ㄱ`..`ㅎ`), each `strncmp`-matched (`n = 2`)
/// against the CP949-converted song title with matches appended via
/// `0x452850` into dialog `+0x680`. This indexed load is the consumer
/// that pins the table's role: song filter (search input), not display.
/// The matching result-banner text (`"%s 검색 결과입니다."` family at
/// VAs `0x645a50`/`0x645a64`/`0x645a80`) reads back the selected names
/// through the sub-range bases below, so display is downstream of filter.
/// (Static disassembly survey; REA dossier TBD.)
pub const KEY_NAME_TABLE_BASE_VA: u32 = 0x6454d8;

/// Chosung initials covered by the `+0x680` indexed search (entries 0..=13).
pub const KEY_NAME_INITIAL_ENTRIES: usize = 14;

/// Table entry index of the first A–Z name (`A` at file `0x245484`).
///
/// The letter search arm (`0x4451af` `A`–`Z` range check, indexed load
/// `mov 0x645538(,%ecx,4),%edx` at file `0x451ed`, `strncmp` `n = 1`)
/// selects entries 24..=49.
pub const KEY_NAME_LETTER_BASE_ENTRY: usize = 24;

/// A–Z names in the key-name table (entries 24..=49).
pub const KEY_NAME_LETTER_ENTRIES: usize = 26;

/// Resolve a `+0x680` keyed-rebuild arm to its key-name table entry.
///
/// Arms 0..=13 index the table base (`KEY_NAME_TABLE_BASE_VA`) directly,
/// selecting the chosung initial the arm filters titles on; other arms
/// have no indexed entry (arms with a doubled form fall back to the fixed
/// `ssang` slots — see [`ojn_search_doubled_initial`]).
#[must_use]
pub fn key_name_filter_entry(arm: u32) -> Option<usize> {
    let entry = usize::try_from(arm).ok()?;
    (entry < KEY_NAME_INITIAL_ENTRIES).then_some(entry)
}

/// Resolve an uppercase ASCII letter to its key-name table entry.
///
/// `b'A'..=b'Z'` map onto entries 24..=49 (base `0x645538`); anything
/// else fails the exe's first-byte range check (file `0x4451af`)
/// and selects no entry.
#[must_use]
pub fn key_name_letter_entry(initial: u8) -> Option<usize> {
    if !initial.is_ascii_uppercase() {
        return None;
    }
    KEY_NAME_LETTER_BASE_ENTRY.checked_add(usize::from(initial - b'A'))
}

/// Parse the client's key-name table: `KEY_NAME_COUNT` little-endian
/// string VAs plus the pool they point into. Each target is a
/// NUL-terminated CP949 (EUC-KR) name; table order is preserved, so
/// aliased entries decode to equal strings (entries 19/50/57 share
/// one target on the real 7.55 file).
///
/// `pool_base` is the VA of `pool[0]` (image base + file offset).
///
/// # Errors
///
/// `FileError::Truncated` on a short table, a VA outside the pool,
/// or an unterminated name; `FileError::UnknownFormat` on a long
/// table, a VA below the pool, or undecodable text.
pub fn parse_key_name_table(
    table: &[u8],
    pool: &[u8],
    pool_base: u32,
) -> Result<Vec<String>, FileError> {
    let need = KEY_NAME_COUNT * KEY_NAME_ENTRY_LEN;
    if table.len() < need {
        return Err(FileError::Truncated {
            need,
            have: table.len(),
        });
    }
    if table.len() > need {
        return Err(unknown_format(table));
    }
    table
        .as_chunks::<KEY_NAME_ENTRY_LEN>()
        .0
        .iter()
        .map(|entry| {
            let va = u32::from_le_bytes(*entry);
            let off = va_offset(va, pool_base, pool.len())?;
            let end = pool[off..]
                .iter()
                .position(|b| *b == 0)
                .map(|len| off + len)
                .ok_or(FileError::Truncated {
                    need: off + 1,
                    have: pool.len(),
                })?;
            decode_control_text(&pool[off..end])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_POOL_BASE: u32 = 0x400000;

    fn key_name_fixture() -> (Vec<u8>, Vec<u8>) {
        // Pool: `B`, then CP949 `\xc0\xfc\xc3\xbc` ("all"),
        // then `A`; entries 1 and 2 alias one target like the real
        // entries 19/50/57.
        let pool = b"B\x00\xc0\xfc\xc3\xbc\x00A\x00".to_vec();
        let name_all = KEY_POOL_BASE + 2;
        let mut table = Vec::with_capacity(KEY_NAME_COUNT * KEY_NAME_ENTRY_LEN);
        for index in 0..KEY_NAME_COUNT {
            let va = match index {
                0 => KEY_POOL_BASE,
                1 | 2 => name_all,
                _ => KEY_POOL_BASE + 7,
            };
            table.extend_from_slice(&va.to_le_bytes());
        }
        (table, pool)
    }

    #[test]
    fn key_names_decode_letters_and_keep_aliases() {
        let (table, pool) = key_name_fixture();
        let names = parse_key_name_table(&table, &pool, KEY_POOL_BASE).unwrap();
        assert_eq!(names.len(), KEY_NAME_COUNT);
        assert_eq!(names[0], "B");
        assert_eq!(names[1], names[2]);
        assert_eq!(names[3], "A");
    }

    #[test]
    fn key_names_decode_korean_entry() {
        let (table, pool) = key_name_fixture();
        let names = parse_key_name_table(&table, &pool, KEY_POOL_BASE).unwrap();
        assert_eq!(names[1], "\u{c804}\u{ccb4}");
    }

    #[test]
    fn key_names_reject_bad_table_length() {
        let (table, pool) = key_name_fixture();
        assert!(matches!(
            parse_key_name_table(&table[..table.len() - 1], &pool, KEY_POOL_BASE),
            Err(FileError::Truncated { .. })
        ));
        let mut long = table.clone();
        long.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            parse_key_name_table(&long, &pool, KEY_POOL_BASE),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn key_names_reject_bad_offsets() {
        let (mut table, pool) = key_name_fixture();
        table[0..4].copy_from_slice(&(KEY_POOL_BASE - 1).to_le_bytes());
        assert!(matches!(
            parse_key_name_table(&table, &pool, KEY_POOL_BASE),
            Err(FileError::UnknownFormat { .. })
        ));
        let (mut table, pool) = key_name_fixture();
        let past_end = KEY_POOL_BASE + u32::try_from(pool.len()).unwrap();
        table[0..4].copy_from_slice(&past_end.to_le_bytes());
        assert!(matches!(
            parse_key_name_table(&table, &pool, KEY_POOL_BASE),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn filter_arms_index_the_chosung_entries() {
        // VA `0x444630` arms 0..=13 index the key-name table base
        // (`mov 0x6454d8(,%edx,4)` at file `0x448c2`) onto the fourteen
        // chosung initials (entries 0..=13, `ㄱ`..`ㅎ`).
        for arm in 0..u32::try_from(KEY_NAME_INITIAL_ENTRIES).unwrap() {
            assert_eq!(
                key_name_filter_entry(arm),
                Some(usize::try_from(arm).unwrap())
            );
        }
    }

    #[test]
    fn filter_arms_past_chosung_select_no_indexed_entry() {
        // Arms 14+ have no indexed slot at the table base: the fixed
        // `ssang` arms (14..18) and the word/letter rows load from
        // fixed slot VAs or the `0x645538` letter base instead.
        for arm in [
            u32::try_from(KEY_NAME_INITIAL_ENTRIES).unwrap(),
            19,
            24,
            50,
            u32::MAX,
        ] {
            assert_eq!(key_name_filter_entry(arm), None);
        }
    }

    #[test]
    fn letter_search_maps_ascii_uppercase_onto_entries() {
        // Letter arm (`0x4451af` range check, `mov 0x645538(,%ecx,4)`
        // at file `0x451ed`): `A`..`Z` select entries 24..=49.
        assert_eq!(key_name_letter_entry(b'A'), Some(24));
        assert_eq!(key_name_letter_entry(b'Z'), Some(49));
        assert_eq!(
            key_name_letter_entry(b'M'),
            Some(KEY_NAME_LETTER_BASE_ENTRY + 12)
        );
        assert_eq!(
            key_name_letter_entry(b'A').unwrap() + KEY_NAME_LETTER_ENTRIES - 1,
            49
        );
    }

    #[test]
    fn letter_search_rejects_non_uppercase_bytes() {
        // Anything outside `A`..`Z` fails the exe range check, so no
        // key-name entry applies (includes lowercase and CP949 lead).
        for initial in [b'a', b'0', b' ', 0xA4, 0xFF] {
            assert_eq!(key_name_letter_entry(initial), None);
        }
    }

    #[test]
    fn key_names_reject_unterminated_name() {
        let (table, _) = key_name_fixture();
        let pool = b"B\xc0\xfc".to_vec();
        assert!(matches!(
            parse_key_name_table(&table, &pool, KEY_POOL_BASE),
            Err(FileError::Truncated { .. })
        ));
    }
}
