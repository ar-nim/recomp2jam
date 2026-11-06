//! Avatar figure: decoded sprite-55 images, drawable on results.
//!
//! One parsed body becomes one figure: every raw-RGB555 image is
//! decoded at its record anchor, anything else (compressed/outlier
//! frames) is skipped — their codecs are still TBD in `o2jam-file`.
//! Equipped parts stack on as extra layers; which filenames those
//! are comes from [`pick_wear_sprites`].

use std::collections::HashMap;

use o2jam_file::{AvatarItemTable, Sprite55};
use o2jam_render::{blit_rgb555, Frame, Rgb555Image};

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
    fn archive_sprite_loads_by_name() {
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[("Logo.ojs", &red)]);
        let figure = load_archive_sprite(&archive, "Logo.ojs").unwrap();
        assert_eq!(figure.len(), 1);
        assert!(load_archive_sprite(&archive, "Missing.ojs").is_none());
        assert!(load_archive_sprite(b"garbage", "Logo.ojs").is_none());
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
}
