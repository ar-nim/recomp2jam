//! Avatar figure: decoded sprite-55 images, drawable on results.
//!
//! One parsed body becomes one figure: every raw-RGB555 image is
//! decoded at its record anchor, anything else (compressed/outlier
//! frames) is skipped — their codecs are still TBD in `o2jam-file`.
//! Equipped parts stack on as extra layers; which filenames those
//! are comes from [`pick_wear_sprites`].

use std::collections::HashMap;

use o2jam_file::{
    AvatarItemTable, Bnd, BndRect, ControlBlock, ControlEntry, ControlList, Sprite55,
};
use o2jam_net::CharacterInfo;
use o2jam_render::{blit_rgb555, rect_from_corners, scale_rect_to_frame, Frame, Rect, Rgb555Image};

/// Archive body name for a table filename: same stem, `.ojs`
/// container (the table spells `.oji`).
fn archive_name(filename: &str) -> String {
    match filename.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.ojs"),
        None => filename.to_owned(),
    }
}

/// One composited part: image plus its record anchor offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FigureLayer {
    /// Decoded pixels.
    pub image: Rgb555Image,
    /// Placement anchor from the sprite record.
    pub dx: i32,
    /// Placement anchor from the sprite record.
    pub dy: i32,
    /// Transparent colour key of the source body.
    pub colorkey: u16,
}

/// Drawable decoded sprite-55 body: layers painted back to front at
/// their anchors (single-image bodies are unaffected; whether
/// multi-image plain bodies are layers or animation frames is OPEN —
/// true animation lives in the strided/`.oja` types).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AvatarFigure {
    /// Layers in paint order.
    layers: Vec<FigureLayer>,
}

impl AvatarFigure {
    /// Decode every raw image at its record anchor; empty when none
    /// decodes.
    #[must_use]
    pub fn from_sprite55(sprite: &Sprite55<'_>) -> Self {
        let layers = sprite
            .images
            .iter()
            .filter(|image| sprite.is_raw_rgb555(image))
            .filter_map(|image| {
                Rgb555Image::decode(sprite.pixel_data(image), image.w as usize, image.h as usize)
                    .map(|decoded| FigureLayer {
                        image: decoded,
                        dx: i32::from(image.x),
                        dy: i32::from(image.y),
                        colorkey: sprite.colorkey,
                    })
            })
            .collect();
        Self { layers }
    }

    /// Stack an extra layer on top (equipped parts resolve here).
    pub fn push_layer(&mut self, layer: FigureLayer) {
        self.layers.push(layer);
    }

    /// Decoded layer count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Whether any layer decoded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Pixel extent covering every layer anchor plus size.
    #[must_use]
    pub fn extent(&self) -> (usize, usize) {
        let (mut w, mut h) = (0usize, 0usize);
        for layer in &self.layers {
            w = w.max(layer.dx.max(0) as usize + layer.image.w);
            h = h.max(layer.dy.max(0) as usize + layer.image.h);
        }
        (w, h)
    }

    /// Paint every layer with its top-left at `(x, y)` plus anchor,
    /// each under its own source colorkey.
    pub fn paint(&self, frame: &mut Frame<'_>, x: i32, y: i32) {
        for layer in &self.layers {
            blit_rgb555(
                frame,
                &layer.image.view(),
                x + layer.dx,
                y + layer.dy,
                Some(layer.colorkey),
            );
        }
    }
}

/// Load one named sprite body from an OPI/OPA archive as a figure.
/// Returns `None` when the entry is missing or undecodable (callers
/// fall back to placeholder art).
#[must_use]
pub fn load_archive_sprite(archive: &[u8], entry_name: &str) -> Option<AvatarFigure> {
    let entries = o2jam_file::parse_opi(archive).ok()?;
    let entry = entries.iter().find(|entry| entry.name == entry_name)?;
    let body = archive.get(entry.data_offset..entry.data_offset + entry.size as usize)?;
    let sprite = o2jam_file::parse_sprite55(body).ok()?;
    let figure = AvatarFigure::from_sprite55(&sprite);
    if figure.is_empty() {
        return None;
    }
    Some(figure)
}

/// Find one avatar-shop row by id (file order; `None` when unknown).
/// Single-lookup companion to the bulk map inside [`load_worn_figure`];
/// side screens list equipment through it.
#[must_use]
pub fn avatar_item_by_id(
    table: &AvatarItemTable,
    item_id: u32,
) -> Option<&o2jam_file::AvatarItemRow> {
    table.rows.iter().find(|row| row.item_id == item_id)
}

/// Build the worn figure for an equipment table: per nonzero item
/// id (in slot order) resolve the item row, pick its wear sprites,
/// and stack every decodable body as layers. Unresolvable pieces —
/// unknown ids, missing archive entries, non-sprite or undecodable
/// bodies — are skipped, never fatal: one bad sprite must not blank
/// the whole avatar. Empty when nothing resolves.
#[must_use]
pub fn load_worn_figure(
    archive: &[u8],
    table: &AvatarItemTable,
    equipment: &[i32],
    male: bool,
) -> AvatarFigure {
    let rows: HashMap<u32, _> = table.rows.iter().map(|row| (row.item_id, row)).collect();
    let entries = o2jam_file::parse_opi(archive).unwrap_or_default();
    let mut figure = AvatarFigure::default();
    for id in equipment.iter().copied().filter(|id| *id != 0) {
        let Some(row) = rows.get(&(id as u32)) else {
            continue;
        };
        for filename in pick_wear_sprites(row, male) {
            let wanted = archive_name(&filename);
            let Some(entry) = entries.iter().find(|entry| entry.name == wanted) else {
                continue;
            };
            let Some(body) =
                archive.get(entry.data_offset..entry.data_offset + entry.size as usize)
            else {
                continue;
            };
            let Ok(sprite) = o2jam_file::parse_sprite55(body) else {
                continue;
            };
            figure
                .layers
                .extend(AvatarFigure::from_sprite55(&sprite).layers);
        }
    }
    figure
}

/// My-room figure from fetched character data (`GetCharacterInfo`
/// 2001): the server's 17-slot equipment table composited exactly
/// like [`load_worn_figure`]. Unresolvable pieces are skipped, never
/// fatal; empty when nothing resolves.
#[must_use]
pub fn load_my_room_figure(
    archive: &[u8],
    table: &AvatarItemTable,
    character: &CharacterInfo,
    male: bool,
) -> AvatarFigure {
    load_worn_figure(archive, table, &character.equipment, male)
}

/// ControlList block per lobby scene (see `docs/file-formats/opi.md`).
pub const AVI_BLOCK: &str = "STATE_AVI";
/// Splash logo sprite bound by `STATE_AVI` (`IDC_BACK_LOGO`).
pub const AVI_LOGO_SPRITE: &str = "O2mediaLogo.ojs";
/// Channel-select block (no `.bnd` bind file: bare `BOUND` line).
pub const CHANNEL_BLOCK: &str = "STATE_CHANNEL";
/// Multiplayer-lobby block laid out by `State_Room.bnd`.
pub const ROOM_BLOCK: &str = "STATE_ROOM";
/// Room layout bind file (75 rects for 76 entries minus self).
pub const ROOM_BIND_FILE: &str = "State_Room.bnd";
/// Pre-game room block laid out by `State_Waiting.bnd`.
pub const WAITING_BLOCK: &str = "STATE_WAITING";
/// Loading-gate art shown while the song table stages.
pub const LOADING_SPRITE: &str = "MusicLoading.ojs";
/// Results block (`Result_WinLose.ojs`, number fonts, avatar).
pub const RESULT_BLOCK: &str = "STATE_RESULT";

/// One `ControlList` entry with its `.bnd` layout rect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePlacement {
    /// Control id (`IDC_...`).
    pub id: String,
    /// Control code (top byte names the state).
    pub code: u32,
    /// Linked sprite body, when the entry names one.
    pub sprite: Option<String>,
    /// Native-canvas layout rect.
    pub rect: Rect,
}

/// Find a `STATE_`/`DIALOG_` block by name.
#[must_use]
pub fn scene_block<'a>(list: &'a ControlList, name: &str) -> Option<&'a ControlBlock> {
    list.blocks.iter().find(|block| block.name == name)
}

/// Sprite bodies one block references, in file order without repeats.
#[must_use]
pub fn scene_sprite_names(block: &ControlBlock) -> Vec<String> {
    let mut names = Vec::new();
    for entry in block.all_entries() {
        push_unique_name(&mut names, entry.sprite.clone());
    }
    names
}

/// Push a sprite name once, skipping missing refs.
fn push_unique_name(names: &mut Vec<String>, sprite: Option<String>) {
    if let Some(name) = sprite {
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
}

/// First `STATE_PLAYING` entry the 17-rect `.bnd` covers (`EXIT`):
/// rect `i` lays out entry `213 + i` (`EXIT`..`VOLUME`), verified on
/// 7.55 (`ControlList_Playing.txt` bound run, decl `0xE6` loads
/// entries 0-229 including self; the last 5 ride unloaded).
const PLAYING_SUBSET_FIRST_ENTRY: usize = 213;

/// Pair block entries with `.bnd` rects verbatim on the native
/// 800x600 canvas: rect `i` lays out entry `i + 1` in file order
/// (entry 0 is the block itself and takes no rect; `SET` members
/// count in file order). `STATE_PLAYING` is the verified subset:
/// its 17 rects lay out entries 213-229 one-to-one, never the first
/// 17. Entries without a rect stay unplaced (the Playing tail past
/// `VOLUME` rides unloaded); rects without an entry are ignored.
#[must_use]
pub fn bind_scene_rects(block: &ControlBlock, bnd: &Bnd) -> Vec<ScenePlacement> {
    let entries = block.all_entries();
    if block.name == "STATE_PLAYING" {
        return bind_playing_subset(&entries, bnd);
    }
    bind_verbatim_entries(&entries, bnd)
}

/// Rect `i` lays out entry `i + 1`; entries without a rect stay
/// unplaced.
fn bind_verbatim_entries(entries: &[&ControlEntry], bnd: &Bnd) -> Vec<ScenePlacement> {
    entries
        .iter()
        .skip(1)
        .zip(bnd.rects.iter())
        .map(|(entry, corners)| place_entry(entry, corners))
        .collect()
}

/// Playing subset: rect `i` lays out entry `213 + i`.
fn bind_playing_subset(entries: &[&ControlEntry], bnd: &Bnd) -> Vec<ScenePlacement> {
    entries
        .iter()
        .skip(PLAYING_SUBSET_FIRST_ENTRY)
        .zip(bnd.rects.iter())
        .map(|(entry, corners)| place_entry(entry, corners))
        .collect()
}

/// One placement from an entry plus its native `.bnd` corners.
fn place_entry(entry: &ControlEntry, corners: &BndRect) -> ScenePlacement {
    ScenePlacement {
        id: entry.id.clone(),
        code: entry.code,
        sprite: entry.sprite.clone(),
        rect: rect_from_corners(corners.x1, corners.y1, corners.x2, corners.y2),
    }
}

/// Load every named sprite body, skipping missing or undecodable
/// bodies (one bad sprite never blanks the scene).
#[must_use]
pub fn load_scene_figures(archive: &[u8], names: &[String]) -> HashMap<String, AvatarFigure> {
    let mut figures = HashMap::new();
    for name in names {
        if figures.contains_key(name) {
            continue;
        }
        if let Some(figure) = load_archive_sprite(archive, name) {
            figures.insert(name.clone(), figure);
        }
    }
    figures
}

/// Load the splash logo (`O2mediaLogo.ojs`) from the archive.
#[must_use]
pub fn load_avi_logo(archive: &[u8]) -> Option<AvatarFigure> {
    load_archive_sprite(archive, AVI_LOGO_SPRITE)
}

/// Frame-space origin of a placement: the verbatim native `.bnd`
/// origin scaled to the frame only at present. The rect itself stays
/// native 800x600; the sprite paints 1:1 at this origin.
#[must_use]
pub fn placement_origin_in_frame(
    placement: &ScenePlacement,
    frame_w: usize,
    frame_h: usize,
) -> (i32, i32) {
    let scaled = scale_rect_to_frame(&placement.rect, frame_w, frame_h);
    (scaled.x, scaled.y)
}

/// Paint every placed figure at its verbatim native origin (scaled
/// to the frame only at present), in file order. No centering and no
/// full-canvas stretch: placements without a loaded figure are
/// skipped, and live UI ink paints on top in the callers.
pub fn paint_scene_figures(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
) {
    let (w, h) = (frame.w, frame.h);
    for placement in placements {
        paint_placed_figure(frame, placement, figures, w, h);
    }
}

/// Paint one placed figure, if its sprite loaded.
fn paint_placed_figure(
    frame: &mut Frame<'_>,
    placement: &ScenePlacement,
    figures: &HashMap<String, AvatarFigure>,
    frame_w: usize,
    frame_h: usize,
) {
    let Some(name) = placement.sprite.as_ref() else {
        return;
    };
    let Some(figure) = figures.get(name) else {
        return;
    };
    let (x, y) = placement_origin_in_frame(placement, frame_w, frame_h);
    figure.paint(frame, x, y);
}

/// Pick the sprite filenames one equipped item contributes to the
/// worn figure (heuristic from 1541 real rows, exe part logic OPEN):
/// slots 0–1 are icons (never worn); otherwise the wearer's parity
/// wins — even slots for male, odd for female (`27M…`/`27F…` split
/// filenames prove the parity); an empty parity falls back to the
/// other side (unisex single-variant items). Slot order is kept so
/// multi-part items (torso + arms) layer deterministically.
#[must_use]
pub fn pick_wear_sprites(row: &o2jam_file::AvatarItemRow, male: bool) -> Vec<String> {
    let mut picked: Vec<String> = row
        .sprites
        .iter()
        .enumerate()
        .filter(|(slot, (_, name))| *slot >= 2 && (*slot % 2 == 0) == male && !name.is_empty())
        .map(|(_, (_, name))| name.clone())
        .collect();
    if picked.is_empty() {
        picked = row
            .sprites
            .iter()
            .enumerate()
            .filter(|(slot, (_, name))| *slot >= 2 && !name.is_empty())
            .map(|(_, (_, name))| name.clone())
            .collect();
    }
    picked
}

#[cfg(test)]
mod avatar_figure_tests {
    use super::*;
    use o2jam_file::parse_sprite55;

    /// Minimal container: header + one record + blob (mirrors the
    /// `o2jam-file` synthetic builders).
    fn sprite55_blob(tag: u16, colorkey: u16, w: u16, h: u16, pixels: &[u8]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&tag.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&colorkey.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&w.to_le_bytes());
        blob.extend_from_slice(&h.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(pixels);
        blob
    }

    #[test]
    fn decodes_first_image_with_colorkey() {
        // 2x1: colorkey black, then red.
        let blob = sprite55_blob(1, 0x0000, 2, 1, &[0x00, 0x00, 0x00, 0x7C]);
        let sprite = parse_sprite55(&blob).unwrap();
        let figure = AvatarFigure::from_sprite55(&sprite);
        assert_eq!(figure.len(), 1);
        let mut buf = vec![0x123456u32; 2];
        figure.paint(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 1,
            },
            0,
            0,
        );
        assert_eq!(buf, vec![0x123456, o2jam_render::pack_xrgb(255, 0, 0)]);
    }

    #[test]
    fn skips_non_raw_images() {
        // Claims 2x2 but stores 2 bytes: not raw, skipped.
        let blob = sprite55_blob(0, 0x0000, 2, 2, &[0xAA, 0xBB]);
        let sprite = parse_sprite55(&blob).unwrap();
        assert!(AvatarFigure::from_sprite55(&sprite).is_empty());
    }

    fn wear_row(sprites: Vec<(u8, &str)>) -> o2jam_file::AvatarItemRow {
        o2jam_file::AvatarItemRow {
            item_id: 1,
            item_type: 0,
            price: 0,
            params: [0; 4],
            flags: (0, 0),
            name: String::new(),
            description: String::new(),
            sprites: sprites
                .into_iter()
                .map(|(flag, name)| (flag, name.to_owned()))
                .collect(),
        }
    }

    #[test]
    fn wear_picks_parity_skipping_icons() {
        let row = wear_row(vec![
            (1, "icon.oji"),
            (1, "icon.oji"),
            (1, "27M23001.oji"),
            (1, "27F23001.oji"),
            (1, "27M23001.oji"),
            (1, "27F23001.oji"),
        ]);
        assert_eq!(
            pick_wear_sprites(&row, true),
            vec!["27M23001.oji".to_owned(), "27M23001.oji".to_owned()]
        );
        assert_eq!(
            pick_wear_sprites(&row, false),
            vec!["27F23001.oji".to_owned(), "27F23001.oji".to_owned()]
        );
    }

    #[test]
    fn wear_falls_back_when_parity_empty() {
        let row = wear_row(vec![
            (1, "icon.oji"),
            (1, "icon.oji"),
            (1, ""),
            (1, "uni.oji"),
        ]);
        assert_eq!(pick_wear_sprites(&row, true), vec!["uni.oji".to_owned()]);
        let bare = wear_row(vec![(1, "icon.oji"), (1, "icon.oji")]);
        assert!(pick_wear_sprites(&bare, false).is_empty());
    }

    #[test]
    fn layers_composite_at_anchors_in_order() {
        use o2jam_render::Rgb555Image;
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C, 0x00, 0x7C], 2, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0xE0, 0x03, 0xE0, 0x03], 2, 1).unwrap(),
            dx: 1,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut buf = vec![0u32; 3];
        figure.paint(
            &mut Frame {
                buf: &mut buf,
                w: 3,
                h: 1,
            },
            0,
            0,
        );
        // Red base, then green over x 1..3 (colorkey black skips nothing here).
        assert_eq!(buf[0], o2jam_render::pack_xrgb(255, 0, 0));
        assert_eq!(buf[1], o2jam_render::pack_xrgb(0, 255, 0));
        assert_eq!(buf[2], o2jam_render::pack_xrgb(0, 255, 0));
    }

    /// Minimal `.opa`: header + bodies + trailing entry table.
    fn opa_blob(bodies: &[(&str, &[u8])]) -> Vec<u8> {
        use o2jam_file::{
            OPI_ENTRY_LEN, OPI_HEADER_LEN, OPI_NAME_OFF, OPI_OFFSET_OFF, OPI_RESERVED_OFF,
            OPI_SIZE_OFF,
        };
        let mut blob = vec![0u8; OPI_HEADER_LEN];
        blob[..4].copy_from_slice(&1u32.to_le_bytes());
        blob[4..8].copy_from_slice(&(bodies.len() as u32).to_le_bytes());
        let mut off = OPI_HEADER_LEN as u32;
        let mut table = Vec::new();
        for (name, body) in bodies {
            let mut entry = vec![0u8; OPI_ENTRY_LEN];
            entry[..4].copy_from_slice(&1u32.to_le_bytes());
            entry[OPI_NAME_OFF..OPI_NAME_OFF + name.len()].copy_from_slice(name.as_bytes());
            entry[OPI_OFFSET_OFF..OPI_OFFSET_OFF + 4].copy_from_slice(&off.to_le_bytes());
            entry[OPI_SIZE_OFF..OPI_SIZE_OFF + 4]
                .copy_from_slice(&(body.len() as u32).to_le_bytes());
            entry[OPI_RESERVED_OFF..OPI_RESERVED_OFF + 4]
                .copy_from_slice(&(body.len() as u32).to_le_bytes());
            table.push(entry);
            off += body.len() as u32;
        }
        for (name, body) in bodies {
            let _ = name;
            blob.extend_from_slice(body);
        }
        for entry in table {
            blob.extend_from_slice(&entry);
        }
        blob
    }

    fn item_row(id: u32, slots: &[&str]) -> o2jam_file::AvatarItemRow {
        o2jam_file::AvatarItemRow {
            item_id: id,
            item_type: 0,
            price: 0,
            params: [0; 4],
            flags: (0, 0),
            name: String::new(),
            description: String::new(),
            sprites: slots.iter().map(|name| (1u8, (*name).to_owned())).collect(),
        }
    }

    #[test]
    fn worn_figure_stacks_resolved_sprites() {
        let red = sprite55_blob(1, 0x0000, 2, 1, &[0x00, 0x00, 0x00, 0x7C]);
        let green = sprite55_blob(1, 0x0000, 1, 1, &[0xE0, 0x03]);
        let archive = opa_blob(&[("01F01001.ojs", &red), ("01F01002.ojs", &green)]);
        let table = o2jam_file::AvatarItemTable {
            rows: vec![item_row(
                5,
                &["icon.oji", "icon.oji", "01F01001.oji", "01F01002.oji"],
            )],
        };
        let male = load_worn_figure(&archive, &table, &[5], true);
        assert_eq!(male.len(), 1);
        let female = load_worn_figure(&archive, &table, &[5], false);
        assert_eq!(female.len(), 1);
        let mut buf = vec![0u32; 2];
        male.paint(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 1,
            },
            0,
            0,
        );
        assert_eq!(buf[1], o2jam_render::pack_xrgb(255, 0, 0));
        let mut buf = vec![0u32; 1];
        female.paint(
            &mut Frame {
                buf: &mut buf,
                w: 1,
                h: 1,
            },
            0,
            0,
        );
        assert_eq!(buf[0], o2jam_render::pack_xrgb(0, 255, 0));
    }

    #[test]
    fn worn_figure_skips_unresolvable_pieces() {
        let archive = opa_blob(&[("real.ojs", &sprite55_blob(1, 0, 1, 1, &[0x00, 0x00]))]);
        let table = o2jam_file::AvatarItemTable {
            rows: vec![
                item_row(5, &["x.oji", "x.oji", "missing.oji", "real.oji"]),
                item_row(6, &["x.oji", "x.oji", "junk.oji"]),
            ],
        };
        // Unknown id 999 skipped; missing entry skipped; junk.oji has
        // no archive body so id 6 contributes nothing either.
        let junk_archive = opa_blob(&[("junk.ojs", b"not a sprite")]);
        assert!(load_worn_figure(&archive, &table, &[999], true).is_empty());
        assert!(load_worn_figure(&junk_archive, &table, &[6], true).is_empty());
    }

    #[test]
    fn my_room_figure_reads_fetched_character_equipment() {
        let red = sprite55_blob(1, 0x0000, 2, 1, &[0x00, 0x00, 0x00, 0x7C]);
        let archive = opa_blob(&[("01F01001.ojs", &red)]);
        let table = o2jam_file::AvatarItemTable {
            rows: vec![item_row(
                5,
                &["icon.oji", "icon.oji", "01F01001.oji", "01F01001.oji"],
            )],
        };
        let mut equipment = [0i32; 17];
        equipment[0] = 5;
        let character = CharacterInfo {
            equipment,
            ..CharacterInfo::default()
        };
        assert_eq!(
            load_my_room_figure(&archive, &table, &character, true).len(),
            1
        );
        let bare = CharacterInfo::default();
        assert!(load_my_room_figure(&archive, &table, &bare, true).is_empty());
    }

    #[test]
    fn archive_sprite_loads_by_name() {
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[("Logo.ojs", &red)]);
        let figure = load_archive_sprite(&archive, "Logo.ojs").unwrap();
        assert_eq!(figure.len(), 1);
        assert!(load_archive_sprite(&archive, "Missing.ojs").is_none());
        assert!(load_archive_sprite(b"garbage", "Logo.ojs").is_none());
    }

    #[test]
    fn avatar_item_by_id_finds_rows_in_file_order() {
        let table = o2jam_file::AvatarItemTable {
            rows: vec![
                item_row(5, &["icon.oji", "icon.oji"]),
                item_row(6, &["icon.oji", "icon.oji"]),
            ],
        };
        assert_eq!(avatar_item_by_id(&table, 6).unwrap().item_id, 6);
        assert!(avatar_item_by_id(&table, 999).is_none());
    }

    #[test]
    fn empty_figure_paints_nothing() {
        let figure = AvatarFigure::default();
        let mut buf = vec![0x123456u32; 4];
        figure.paint(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 2,
            },
            0,
            0,
        );
        assert_eq!(buf, vec![0x123456u32; 4]);
    }

    fn room_control_text() -> String {
        [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X00100000",
            "IDC_LIST 0X10500001 \"Room_List.ojs\"",
            "IDC_BUTTON 0X20500002 \"Room_Button.ojs\"",
            "}",
            "",
        ]
        .join("\n")
    }

    fn bnd_blob(rects: &[(u32, u32, u32, u32)]) -> Vec<u8> {
        let mut blob = vec![0xFF; 4];
        blob.extend_from_slice(&(rects.len() as u16).to_le_bytes());
        for (x1, y1, x2, y2) in rects {
            blob.extend_from_slice(&x1.to_le_bytes());
            blob.extend_from_slice(&y1.to_le_bytes());
            blob.extend_from_slice(&x2.to_le_bytes());
            blob.extend_from_slice(&y2.to_le_bytes());
        }
        blob
    }

    #[test]
    fn scene_block_finds_room_by_name() {
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        assert_eq!(scene_block(&list, ROOM_BLOCK).unwrap().name, ROOM_BLOCK);
        assert!(scene_block(&list, RESULT_BLOCK).is_none());
    }

    #[test]
    fn scene_sprite_names_keep_file_order_without_repeats() {
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        assert_eq!(
            scene_sprite_names(block),
            vec!["Room_List.ojs".to_owned(), "Room_Button.ojs".to_owned()]
        );
    }

    #[test]
    fn scene_rects_pair_entries_with_bnd_corners() {
        // Entry 0 (the block itself) takes no rect; rect i covers
        // entry i + 1, so IDC_LIST lands on the first corner run.
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        let bnd =
            o2jam_file::parse_bnd(&bnd_blob(&[(0, 0, 400, 300), (400, 0, 800, 600)])).unwrap();
        let placed = bind_scene_rects(block, &bnd);
        assert_eq!(placed.len(), 2);
        assert_eq!(placed[0].id, "IDC_LIST");
        assert_eq!(placed[0].sprite.as_deref(), Some("Room_List.ojs"));
        assert_eq!(
            placed[0].rect,
            o2jam_render::Rect {
                x: 0,
                y: 0,
                w: 400,
                h: 300
            }
        );
        assert_eq!(placed[1].id, "IDC_BUTTON");
    }

    #[test]
    fn scene_rects_leave_extra_entries_unplaced() {
        // One rect lays out entry 1 verbatim; entry 2 has no rect
        // and stays unplaced (never silently dropped from a
        // different offset).
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(8, 8, 16, 16)])).unwrap();
        let placed = bind_scene_rects(block, &bnd);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].id, "IDC_LIST");
    }

    /// Playing-shaped block: 235 entries in file order with the
    /// bound run at 213-229 (`EXIT`..`VOLUME`).
    fn playing_block() -> ControlBlock {
        let mut bound = Vec::new();
        bound.push(ControlEntry {
            id: "IDC_STATE_PLAY".to_owned(),
            code: 0,
            sprite: None,
        });
        for index in 1..235 {
            bound.push(ControlEntry {
                id: format!("IDC_FILL_{index:03}"),
                code: 0,
                sprite: None,
            });
        }
        bound[213] = ControlEntry {
            id: "IDC_BUTTON_SPLAY_EXIT".to_owned(),
            code: 0,
            sprite: Some("Playing_Exit.ojs".to_owned()),
        };
        bound[229] = ControlEntry {
            id: "IDC_TEXT_SPLAY_VLOLUME".to_owned(),
            code: 0,
            sprite: None,
        };
        ControlBlock {
            name: "STATE_PLAYING".to_owned(),
            number: 0xE6,
            extra: Vec::new(),
            bound_file: Some("State_Playing.bnd".to_owned()),
            bound,
            sets: Vec::new(),
        }
    }

    #[test]
    fn playing_rects_map_trailing_subset_verbatim() {
        // 17 rects lay out entries 213-229 one-to-one, never the
        // first 17 entries.
        let block = playing_block();
        let corners: Vec<(u32, u32, u32, u32)> =
            (0..17).map(|i| (751 + i, 554, 793 + i, 596)).collect();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&corners)).unwrap();
        let placed = bind_scene_rects(&block, &bnd);
        assert_eq!(placed.len(), 17);
        assert_eq!(placed[0].id, "IDC_BUTTON_SPLAY_EXIT");
        assert_eq!(placed[0].sprite.as_deref(), Some("Playing_Exit.ojs"));
        assert_eq!(placed[16].id, "IDC_TEXT_SPLAY_VLOLUME");
        assert_eq!(placed[0].rect.x, 751);
        assert_eq!(placed[1].rect.x, 752);
        assert!(block.all_entries()[1].id.starts_with("IDC_FILL_"));
        assert_ne!(placed[0].id, block.all_entries()[1].id);
    }

    #[test]
    fn avi_logo_loads_under_its_control_name() {
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[(AVI_LOGO_SPRITE, &red)]);
        assert_eq!(load_avi_logo(&archive).unwrap().len(), 1);
        assert!(load_avi_logo(b"garbage").is_none());
    }

    #[test]
    fn scene_figures_paint_at_verbatim_origins() {
        // Native `.bnd` rects stay verbatim: x 400 paints at x 400
        // on the native canvas, scaled to x 40 only at present on
        // an 80-wide frame.
        let red = sprite55_blob(1, 0xFFFF, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[("Room_List.ojs", &red)]);
        let figures = load_scene_figures(&archive, &["Room_List.ojs".to_owned()]);
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(400, 0, 800, 600), (0, 0, 8, 8)])).unwrap();
        let placed = bind_scene_rects(block, &bnd);
        assert_eq!(placement_origin_in_frame(&placed[0], 800, 600), (400, 0));
        assert_eq!(placement_origin_in_frame(&placed[0], 80, 60), (40, 0));
        let mut buf = vec![0u32; 80 * 60];
        paint_scene_figures(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 60,
            },
            &placed,
            &figures,
        );
        assert_eq!(buf[40], o2jam_render::pack_xrgb(255, 0, 0));
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn scene_figures_skip_missing_sprites() {
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(0, 0, 8, 8), (0, 0, 8, 8)])).unwrap();
        let placed = bind_scene_rects(block, &bnd);
        let mut buf = vec![0x123456u32; 8 * 8];
        paint_scene_figures(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            &placed,
            &std::collections::HashMap::new(),
        );
        assert_eq!(buf, vec![0x123456u32; 8 * 8]);
    }

    #[test]
    fn figure_extent_covers_anchors_and_sizes() {
        // No centering: extent only measures native anchors plus
        // sizes; placement origins come from `.bnd` rects verbatim.
        assert_eq!(AvatarFigure::default().extent(), (0, 0));
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let sprite = o2jam_file::parse_sprite55(&red).unwrap();
        let figure = AvatarFigure::from_sprite55(&sprite);
        assert_eq!(figure.extent(), (1, 1));
    }
}
