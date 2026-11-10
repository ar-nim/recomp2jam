//! Synthetic OPI/OPA archives for tests (no game data).
//!
//! Builds valid archives from chosen entry names and bodies, plus
//! corrupt variants that fail to parse. Layout matches
//! [`super::opi`]: 16-byte header, bodies from +16, trailing
//! fixed-size table. A missing entry is built by omitting its name.

use super::opi::{
    OPI_ENTRY_LEN, OPI_HEADER_LEN, OPI_NAME_LEN, OPI_NAME_OFF, OPI_OFFSET_OFF, OPI_RESERVED_OFF,
    OPI_SIZE_OFF,
};

/// Archive version written by the builder (`.opi`).
const BUILDER_VERSION: u32 = 2;

/// Entry kind for a live file slot.
const FILE_KIND: u32 = 1;

/// Build a valid archive from chosen entry names and bodies.
///
/// Names longer than the table field keep their first
/// [`OPI_NAME_LEN`] bytes. Counts and offsets that do not fit
/// saturate instead of wrapping.
#[must_use]
pub fn build_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let slot_count = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    let mut archive = Vec::new();
    archive.extend_from_slice(&BUILDER_VERSION.to_le_bytes());
    archive.extend_from_slice(&slot_count.to_le_bytes());
    archive.extend_from_slice(&0u64.to_le_bytes());
    let mut table = Vec::new();
    for (name, body) in entries {
        let offset = u32::try_from(archive.len()).unwrap_or(u32::MAX);
        let size = u32::try_from(body.len()).unwrap_or(u32::MAX);
        archive.extend_from_slice(body);
        table.extend_from_slice(&one_slot(name, offset, size));
    }
    archive.extend_from_slice(&table);
    archive
}

/// Build an archive that fails to parse: a valid body with its last
/// table byte cut, so the trailing table overruns the buffer.
#[must_use]
pub fn build_corrupt_archive() -> Vec<u8> {
    let mut archive = build_archive(&[("a.ojs", b"data".as_slice())]);
    archive.pop();
    archive
}

/// Rewrite the header slot count in place. Ignores buffers shorter
/// than the header.
pub fn set_slot_count(archive: &mut [u8], count: u32) {
    if archive.len() >= OPI_HEADER_LEN {
        archive[4..8].copy_from_slice(&count.to_le_bytes());
    }
}

/// Rewrite the first table slot's body size in place. Ignores
/// buffers with no full table slot.
pub fn set_first_entry_size(archive: &mut [u8], size: u32) {
    let Some(table_start) = archive.len().checked_sub(OPI_ENTRY_LEN) else {
        return;
    };
    if table_start < OPI_HEADER_LEN {
        return;
    }
    let at = table_start + OPI_SIZE_OFF;
    if archive.len() >= at + 4 {
        archive[at..at + 4].copy_from_slice(&size.to_le_bytes());
    }
}

/// One trailing-table slot for a live file.
fn one_slot(name: &str, offset: u32, size: u32) -> Vec<u8> {
    let mut slot = vec![0u8; OPI_ENTRY_LEN];
    slot[..4].copy_from_slice(&FILE_KIND.to_le_bytes());
    let kept = name.len().min(OPI_NAME_LEN);
    slot[OPI_NAME_OFF..OPI_NAME_OFF + kept].copy_from_slice(&name.as_bytes()[..kept]);
    slot[OPI_OFFSET_OFF..OPI_OFFSET_OFF + 4].copy_from_slice(&offset.to_le_bytes());
    slot[OPI_SIZE_OFF..OPI_SIZE_OFF + 4].copy_from_slice(&size.to_le_bytes());
    slot[OPI_RESERVED_OFF..OPI_RESERVED_OFF + 4].copy_from_slice(&size.to_le_bytes());
    slot
}
