//! One decoded `avatar.opa` source per install root (ticket #755).
//!
//! The boot and every channel login stage the shop catalog from this
//! archive. Without a cache each landing re-reads 139 MiB from disk
//! and re-decodes the item table and every shop portrait. This module
//! owns that decode and shares the immutable result for the process,
//! like the Interface/Playing archive cache in [`crate::staging`].

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};
use o2jam_file::{
    find_opi_entry, opi_entry_body, parse_avatar_item_table, parse_opi, AvatarItemTable,
};

use crate::avatar::{load_shop_featured_figures, load_shop_portraits, AvatarFigure};

/// Archive path under an install root.
pub const AVATAR_ARCHIVE_RELATIVE: &str = "O2Jam/Image/avatar.opa";

/// Item-table body inside the archive (7.55 spelling).
const ITEM_TABLE_ENTRY: &str = "Itemdata_Korea.dat";

/// Archive bytes, item table, and shop portraits decoded once.
#[derive(Debug, Clone)]
pub struct AvatarSource {
    archive: Arc<Vec<u8>>,
    table: AvatarItemTable,
    shop_portraits: Arc<HashMap<u32, AvatarFigure>>,
    shop_featured_figures: Arc<HashMap<u32, AvatarFigure>>,
}

impl AvatarSource {
    /// Decode the item table and every shop portrait from archive bytes.
    ///
    /// # Errors
    ///
    /// The archive directory or item table is not in 7.55 shape.
    pub fn decode(archive: Vec<u8>) -> Result<Self> {
        let table = decode_item_table(&archive)?;
        Ok(Self::with_table(archive, table))
    }

    /// Compose a source from an already parsed table (test staging and
    /// callers that kept the table beside the bytes).
    #[must_use]
    pub fn with_table(archive: Vec<u8>, table: AvatarItemTable) -> Self {
        let shop_portraits = Arc::new(load_shop_portraits(&archive, &table));
        let shop_featured_figures = Arc::new(load_shop_featured_figures(&archive, &table));
        Self {
            archive: Arc::new(archive),
            table,
            shop_portraits,
            shop_featured_figures,
        }
    }

    /// Raw archive bytes, shared without a per-landing copy.
    #[must_use]
    pub fn archive(&self) -> &[u8] {
        &self.archive
    }

    /// Parsed `Itemdata_Korea.dat` rows.
    #[must_use]
    pub fn table(&self) -> &AvatarItemTable {
        &self.table
    }

    /// Large shop portraits keyed by item id.
    #[must_use]
    pub fn shop_portraits(&self) -> &Arc<HashMap<u32, AvatarFigure>> {
        &self.shop_portraits
    }

    /// Full-body figures for the set-item cards, keyed by item id.
    #[must_use]
    pub fn shop_featured_figures(&self) -> &Arc<HashMap<u32, AvatarFigure>> {
        &self.shop_featured_figures
    }
}

/// Parse the item table out of an in-memory `avatar.opa` image.
///
/// # Errors
///
/// The bytes are not an `.opa` archive, or its item table is missing
/// or corrupt.
pub fn decode_item_table(archive: &[u8]) -> Result<AvatarItemTable> {
    let entries = parse_opi(archive).context("avatar.opa directory")?;
    let entry = find_opi_entry(&entries, ITEM_TABLE_ENTRY)
        .with_context(|| format!("avatar.opa lacks {ITEM_TABLE_ENTRY}"))?;
    let bytes = opi_entry_body(archive, entry).context("item table out of archive bounds")?;
    parse_avatar_item_table(bytes).context("avatar item table")
}

/// One decoded source per canonical archive path, filled on first use.
static DECODED_AVATAR_CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<AvatarSource>>>> = OnceLock::new();

/// Read and decode the installed `avatar.opa` once per process. A
/// missing or unparsable archive is not cached: a later landing retries
/// it, exactly as an uncached read would.
#[must_use]
pub fn read_installed_avatar(root: &Path) -> Option<Arc<AvatarSource>> {
    if !root.is_dir() {
        return None;
    }
    let path = root.join(AVATAR_ARCHIVE_RELATIVE);
    let key = path.canonicalize().unwrap_or_else(|_| path.clone());
    let cache = DECODED_AVATAR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match cache.entry(key) {
        Entry::Occupied(slot) => Some(Arc::clone(slot.get())),
        Entry::Vacant(slot) => {
            let archive = std::fs::read(&path).ok()?;
            let source = Arc::new(AvatarSource::decode(archive).ok()?);
            Some(Arc::clone(slot.insert(source)))
        }
    }
}

/// Drop every cached decoded source so the next read decodes again.
/// Test-only hook behind the `test-support` feature: downstream guards
/// force a cache miss to prove identity reuse is caller-owned (#862).
#[cfg(feature = "test-support")]
pub fn clear_decoded_avatar_cache() {
    if let Some(cache) = DECODED_AVATAR_CACHE.get() {
        let mut cache = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{read_installed_avatar, AVATAR_ARCHIVE_RELATIVE};
    use std::path::Path;
    use std::sync::Arc;

    #[test]
    fn repeated_identity_landings_share_one_decoded_avatar() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
        if !root.join(AVATAR_ARCHIVE_RELATIVE).is_file() {
            println!("skip: missing install-root avatar.opa");
            return;
        }
        let first = read_installed_avatar(&root).expect("installed avatar decodes");
        let second = read_installed_avatar(&root).expect("installed avatar decodes");
        assert!(Arc::ptr_eq(&first, &second));
    }
}
