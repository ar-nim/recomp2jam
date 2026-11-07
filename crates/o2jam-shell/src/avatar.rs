//! Avatar figure: decoded sprite-55 images, drawable on results.
//!
//! One parsed body becomes one figure: every raw-RGB555 image is
//! decoded at its record anchor, anything else (compressed/outlier
//! frames) is skipped — their codecs are still TBD in `o2jam-file`.
//! Equipped parts stack on as extra layers; which filenames those
//! are comes from [`pick_wear_sprites`].

use std::collections::HashMap;

use o2jam_file::{
    classify_sprite, find_opi_entry, opi_entry_body, pair_entries_with_rects, resolve_sprite_entry,
    AvatarItemTable, Bnd, BndRect, ControlBlock, ControlEntry, ControlList, OpiEntry,
    PlacedControl, Sprite55, SpriteResolution,
};
use o2jam_net::CharacterInfo;
use o2jam_render::{
    blit_rgb555, blit_rgb555_clipped, rect_from_corners, scale_rect_to_frame, Frame, Rect,
    Rgb555Image,
};

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

    /// One decoded layer in paint order (`None` past the end).
    /// Number-font bodies decode one layer per digit, so digit
    /// painters index here instead of re-stacking the whole body.
    #[must_use]
    pub fn layer(&self, index: usize) -> Option<&FigureLayer> {
        self.layers.get(index)
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

    /// Paint every layer with its top-left exactly at `(x, y)`, under
    /// its own source colorkey, ignoring the record anchors. For
    /// `.bnd`-claimed placements: 7.55 sprite records store absolute
    /// canvas anchors that duplicate their bound rect origins
    /// byte-exact (`Normal_TimeBar.ojs` (489,556) ==
    /// `State_Playing.bnd` rect[2], `Playing_Exit.ojs` (751,554) ==
    /// rect[0], `Wait_TeamA.ojs` (652,65) == `State_Waiting.bnd`
    /// rect[3]), so adding both doubles the origin and throws the art
    /// off-canvas. BpPoint bars differ (`BpPoint_Left_bar.ojs` anchor
    /// `(2,266)` vs rect `(200,3)`), so the rect — the verified draw
    /// origin (`docs/bnd-semantics.md`) — wins, and the anchor rides
    /// only unbound placements via [`Self::paint`] (native frames) or
    /// [`Self::paint_scaled_anchors`] (scaled frames).
    pub fn paint_without_anchors(&self, frame: &mut Frame<'_>, x: i32, y: i32) {
        for layer in &self.layers {
            blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
        }
    }

    /// Paint every layer with its top-left at `(x, y)` plus its
    /// record anchor scaled into the frame, each under its own source
    /// colorkey. For unbound placements: the exe canvas is native
    /// 800x600, so native anchors scale with the frame exactly like
    /// `.bnd` origins (`placement_origin_in_frame`), keeping the art's
    /// exe proportions at any frame size instead of drawing the raw
    /// native corner on a scaled frame.
    pub fn paint_scaled_anchors(
        &self,
        frame: &mut Frame<'_>,
        x: i32,
        y: i32,
        frame_w: usize,
        frame_h: usize,
    ) {
        for layer in &self.layers {
            let (dx, dy) = anchor_in_frame(layer.dx, layer.dy, frame_w, frame_h);
            blit_rgb555(
                frame,
                &layer.image.view(),
                x + dx,
                y + dy,
                Some(layer.colorkey),
            );
        }
    }
}

/// Frame-space offset of one record anchor: native anchors scale
/// with the frame through the same ratios [`scale_rect_to_frame`]
/// applies to `.bnd` origins, so unbound art keeps its exe
/// proportions at any frame size (a zero-size rect moves the
/// origin without sizing anything).
fn anchor_in_frame(dx: i32, dy: i32, frame_w: usize, frame_h: usize) -> (i32, i32) {
    let moved = scale_rect_to_frame(
        &Rect {
            x: dx,
            y: dy,
            w: 0,
            h: 0,
        },
        frame_w,
        frame_h,
    );
    (moved.x, moved.y)
}

/// Load one named sprite body from an OPI/OPA archive as a figure.
/// Returns `None` when the entry is missing or undecodable (callers
/// fall back to placeholder art). Lookup is exact first, then one
/// ASCII-case fallback (recipe step 6): 7.55 spells four bodies
/// differently from their block refs (`Main_level.ojs` vs
/// `main_level.ojs`, plus three Interface case-only refs;
/// docs/file-formats/opi.md). A leading `_` falls back to its bare
/// stem (`_Playing_BG.ojs` loads through `Playing_BG.ojs`): the
/// underscore marks runtime resolution, and the bare stem is the
/// stage-less arm (UI-preview contexts, exe archive flag clear).
/// The body slices `size`, never `reserved_size` (step 6; the
/// patcher keeps stale allocation bytes past `size`).
#[must_use]
pub fn load_archive_sprite(archive: &[u8], entry_name: &str) -> Option<AvatarFigure> {
    load_archive_sprite_for_stage(archive, entry_name, None)
}

/// [`load_archive_sprite`] through the room arena stage: a `_`
/// runtime ref resolves through [`resolve_sprite_entry`] with the
/// stage (applier `0x4e7850`) — the numbered sibling when the
/// archive names it (`_Playing_BG.ojs` + stage 7 → `Playing_BG7.ojs`),
/// an empty slot on a miss (the exe's slot loader leaves the slot
/// empty), and the bare stem for arms the stage cannot settle
/// (`Note_Click` variant kits, the `Note_LongCombo` bare pair) and
/// for stage-less callers.
#[must_use]
pub fn load_archive_sprite_for_stage(
    archive: &[u8],
    entry_name: &str,
    arena_stage: Option<u16>,
) -> Option<AvatarFigure> {
    let entries = o2jam_file::parse_opi(archive).ok()?;
    let entry = resolved_archive_entry(&entries, entry_name, arena_stage)?;
    let body = opi_entry_body(archive, &entry).ok()?;
    let sprite = o2jam_file::parse_sprite55(body).ok()?;
    let figure = AvatarFigure::from_sprite55(&sprite);
    if figure.is_empty() {
        return None;
    }
    Some(figure)
}

/// The archive entry one sprite reference loads through: exact
/// match first, then one ASCII-case pass (recipe step 6); a `_`
/// runtime ref settles through the exe applier rule
/// ([`resolve_sprite_entry`]) with the caller's stage. Owned so the
/// synthetic reference can drop before the body loads.
fn resolved_archive_entry(
    entries: &[OpiEntry],
    entry_name: &str,
    arena_stage: Option<u16>,
) -> Option<OpiEntry> {
    if let Some(entry) = find_opi_entry(entries, entry_name) {
        return Some(entry.clone());
    }
    let stem = entry_name.strip_prefix('_')?;
    let reference = ControlEntry {
        id: String::new(),
        code: 0,
        sprite: Some(entry_name.to_owned()),
    };
    match resolve_sprite_entry(&reference, entries, arena_stage) {
        SpriteResolution::Found(entry) => Some(entry.clone()),
        // A missed numbered name leaves the slot empty (exe slot
        // loader; no default-stage fallback image exists).
        SpriteResolution::Missing { .. } => None,
        // Stage-less callers and stage-unsettled arms take the bare
        // stem beside the numbered siblings.
        SpriteResolution::RuntimeDeferred { .. } => find_opi_entry(entries, stem).cloned(),
        // A `_`-prefixed scrollbar parameter names no file.
        SpriteResolution::NoArt | SpriteResolution::ScrollbarParams => None,
    }
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

/// Empty `0x00` stub blocks: present in `ControlList_Interface.txt`
/// with no body (`PLAYING`, `TUTORIAL`, `DRUM_PLAYING`, `EXIT`).
/// They draw nothing (recipe step 3); the real `STATE_PLAYING`
/// lives in `Playing1.opi`.
const STUB_BLOCK_NAMES: [&str; 4] = [
    "STATE_PLAYING",
    "STATE_TUTORIAL",
    "STATE_DRUM_PLAYING",
    "STATE_EXIT",
];

/// True for an empty stub block header (recipe step 3: draws nothing).
/// Only the bare `0x00` stubs match: a real block that happens to
/// share the name but carries entries still stages.
#[must_use]
pub fn is_stub_block(block: &ControlBlock) -> bool {
    block.all_entries().is_empty() && STUB_BLOCK_NAMES.contains(&block.name.as_str())
}

/// State id of one entry: the top byte of its `IDC_` code
/// (lifecycle §2: `0x12` channel … `0x1B` playing … `0x52` ranking).
/// The second byte names the `CC*` control class; exact dispatch is
/// TBD, so the composer only reads the state byte, never branches
/// on the class.
#[must_use]
pub fn entry_state_id(code: u32) -> u8 {
    (code >> 24) as u8
}

/// True when an entry's code leaves it a paintable control: the low
/// 12 bits name the control instance (verified pairing mask
/// `0x548640`: ordinal = `(code & 0xFFF) − 1`), so a zero low word
/// marks a scene/container row (entry 0 of every real block) that
/// takes no rect and draws nothing (recipe: entry codes gate
/// visibility). Verified on 7.55: the only zero-low-word rows in
/// either control list are the block self rows, so genuine art never
/// drops here while container rows stay unplaced even outside
/// position 0.
#[must_use]
pub fn entry_takes_rect(entry: &ControlEntry) -> bool {
    entry.code & 0xFFF != 0
}

/// Placements for blocks without a `.bnd` (recipe steps 4 + 7): a
/// bare `BOUND` line means no layout to claim, so each entry that
/// names loadable art paints 1:1 at its own record anchor (empty
/// rect, never full-bleed). Sprite-less entries draw no art (step
/// 6); scrollbar parameters draw no art either (step 9). Runtime-
/// deferred (`_`-prefix) and archive-missing names still stage:
/// the loader skips what it cannot decode.
#[must_use]
pub fn anchor_placements(block: &ControlBlock) -> Vec<ScenePlacement> {
    block
        .all_entries()
        .into_iter()
        .filter(|entry| entry_takes_rect(entry))
        .filter_map(anchor_placement)
        .collect()
}

/// The all-zero rect an entry carries when no `.bnd` rect claims it
/// (verified pairing: a miss stores `(0,0,0,0)` instead of skipping
/// the entry): the draw origin stays at the frame origin, so the
/// sprite's record anchor positions the art alone.
fn unplaced_rect() -> Rect {
    Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    }
}

/// One anchor placement for an unbound entry: real id and code,
/// all-zero rect (the sprite's own record anchor positions it).
/// Sprite-less entries and scrollbar parameters stage nothing.
fn anchor_placement(entry: &ControlEntry) -> Option<ScenePlacement> {
    match classify_sprite(entry) {
        SpriteResolution::NoArt | SpriteResolution::ScrollbarParams => None,
        SpriteResolution::RuntimeDeferred { .. }
        | SpriteResolution::Found(_)
        | SpriteResolution::Missing { .. } => Some(ScenePlacement {
            id: entry.id.clone(),
            code: entry.code,
            sprite: entry.sprite.clone(),
            rect: unplaced_rect(),
        }),
    }
}

/// One `ControlList` entry with its `.bnd` layout rect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePlacement {
    /// Control id (`IDC_...`).
    pub id: String,
    /// Control code (top byte names the state).
    pub code: u32,
    /// Linked sprite body, when the entry names one.
    pub sprite: Option<String>,
    /// Native-canvas layout rect: positions the draw origin, never
    /// clips (verified draw path).
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

/// First `STATE_PLAYING` entry of the bound run (`EXIT`): the
/// 17-rect `.bnd` window pairs entries 213–229 by code ordinal —
/// `EXIT` carries ordinal 1 and `VOLUME` ordinal 17, verified on
/// 7.55 (`ControlList_Playing.txt` bound run, decl `0xE6` loads
/// entries 0–229 including self; the last 5 ride unloaded).
const PLAYING_SUBSET_FIRST_ENTRY: usize = 213;

/// Static Playing backdrop panels and gauge shells that paint over
/// the highway without hiding it, in file order (back-to-front):
/// the bottom menu strip plus the life/jam edge gauges and the
/// level plate. Every name is a `STATE_PLAYING` entries 0–212
/// sprite, decodes as raw RGB555 in 7.55, and anchors clear of the
/// note lanes (native `x5..193 × y0..487`), the score digits at
/// `(16, 16)`/`(16, 32)`, and the judgement word under the
/// judgement row — so falling notes and live UI ink stay visible.
/// Lane-covering panels stay out on purpose: they belong to the
/// behind-highway scene below, never to this overlay set.
/// Right-hand gauge bars ride unreferenced by the block (only the
/// left shells are named), so only staged — never invented — art
/// appears below.
pub const PLAYING_BACKDROP_PANELS: [&str; 6] = [
    "Playing_Menu.ojs",
    "Life_gauge_back.ojs",
    "Life_gauge_left_bar.ojs",
    "Jamgauge_left_bar.ojs",
    "Jam_gauge_text.ojs",
    "Main_level.ojs",
];

/// True for a static Playing backdrop panel sprite.
fn is_playing_backdrop_panel(sprite: &str) -> bool {
    PLAYING_BACKDROP_PANELS.contains(&sprite)
}

/// Behind-highway scene: the venue `BACK1` runtime ref plus the
/// note-background `BACK2`/`3Key`/`P2P` siblings (`STATE_PLAYING`
/// entries 113–114 and 209–212, quoted names in file order).
/// `_Playing_BG.ojs` resolves through the room arena stage at load
/// (`Playing_BG<stage>.ojs`, [`load_archive_sprite_for_stage`]) and
/// anchors at native `(204, 0)` clear of the note lanes (`x5..193`),
/// so the right side shows the room's venue art; the exe paints it
/// first of all (file order `BACK1`), so it sits behind every later
/// pass. The `Note_BG` siblings overlap the lanes on some layers,
/// so behind painters clip the lane columns the caller passes (see
/// `paint_behind_placements`). Every name resolves in
/// `Playing1.opi`. Couple-mode variants ride unreferenced by the
/// block, so only these five stage here — never invented art.
pub const PLAYING_BEHIND_SPRITES: [&str; 5] = [
    "Playing_BG.ojs",
    "Note_BG.ojs",
    "3Key_Note_BG.ojs",
    "P2P3KeyNote_BG.ojs",
    "P2P7KeyNote_BG.ojs",
];

/// True for a behind-highway scene sprite, quoted or resolved: the
/// `_Playing_BG.ojs` block ref and its bare `Playing_BG.ojs` stem
/// both match.
#[must_use]
pub fn is_playing_behind_sprite(sprite: &str) -> bool {
    PLAYING_BEHIND_SPRITES.contains(&sprite)
        || sprite
            .strip_prefix('_')
            .is_some_and(|stem| PLAYING_BEHIND_SPRITES.contains(&stem))
}

/// Anchor placement for one static backdrop entry: no `.bnd` rect
/// covers entries 0–212, so the sprite paints 1:1 at its own
/// record anchor (all-zero rect, never full-bleed).
fn place_backdrop_panel(entry: &ControlEntry, sprite: &str) -> ScenePlacement {
    ScenePlacement {
        id: entry.id.clone(),
        code: entry.code,
        sprite: Some(sprite.to_owned()),
        rect: unplaced_rect(),
    }
}

/// Anchor placements for the static Playing backdrop panels among
/// entries `0..213`, in file order. Entries without a panel sprite
/// contribute nothing, so short or sprite-less blocks stage no
/// backdrop; container-coded entries stay out like the bound runs.
fn bind_playing_backdrops(entries: &[&ControlEntry]) -> Vec<ScenePlacement> {
    entries
        .iter()
        .take(PLAYING_SUBSET_FIRST_ENTRY)
        .filter(|entry| entry_takes_rect(entry))
        .filter_map(|entry| {
            let sprite = entry.sprite.as_deref()?;
            is_playing_backdrop_panel(sprite).then(|| place_backdrop_panel(entry, sprite))
        })
        .collect()
}

/// Anchor placements for the behind-highway scene among entries
/// `0..213`, in file order. Staging keeps the quoted block ref
/// (`_Playing_BG.ojs` resolves at load through
/// [`load_archive_sprite`]); container-coded entries stay out like
/// the bound runs.
fn bind_playing_behind(entries: &[&ControlEntry]) -> Vec<ScenePlacement> {
    entries
        .iter()
        .take(PLAYING_SUBSET_FIRST_ENTRY)
        .filter(|entry| entry_takes_rect(entry))
        .filter_map(|entry| {
            let sprite = entry.sprite.as_deref()?;
            is_playing_behind_sprite(sprite).then(|| place_backdrop_panel(entry, sprite))
        })
        .collect()
}

/// Pair block entries with `.bnd` rects by code ordinal on the
/// native 800x600 canvas (verified pairing): rect index
/// `(code & 0xFFF) − 1` per entry; a zero low word underflows and an
/// out-of-range ordinal misses, both staging the all-zero rect
/// (anchor draw from the origin, never a skip). Stub blocks stage
/// nothing (step 3); container-coded entries (zero low word) stay
/// unplaced. `STATE_PLAYING` keeps the verified window: its 17 rects
/// lay out the `EXIT`..`VOLUME` run (entries 213–229, ordinals
/// 1–17); the static backdrop panels among entries 0–212 stage as
/// anchor placements ahead of them, and the behind-highway scene
/// stages first of all. Rects no entry claims are ignored (the
/// Playing tail past `VOLUME` rides unloaded).
#[must_use]
pub fn bind_scene_rects(block: &ControlBlock, bnd: &Bnd) -> Vec<ScenePlacement> {
    if is_stub_block(block) {
        return Vec::new();
    }
    if block.name == "STATE_PLAYING" {
        return bind_playing_subset(block, bnd);
    }
    bind_entries_by_ordinal(block, bnd)
}

/// Every paintable entry paired with its ordinal rect through the
/// shared [`pair_entries_with_rects`] pairing (verified, exe
/// `0x548640`): ordinal misses stage the all-zero rect (anchor draw
/// from the origin), and container-coded entries (zero low word)
/// stay out. Paint stays in file order, so `all_entries` order is
/// the order that matters.
fn bind_entries_by_ordinal(block: &ControlBlock, bnd: &Bnd) -> Vec<ScenePlacement> {
    pair_entries_with_rects(block, bnd)
        .into_iter()
        .filter(|placed| entry_takes_rect(placed.entry))
        .map(entry_placement)
        .collect()
}

/// Native rect of one exe-paired entry: the `.bnd` corners, or the
/// all-zero rect on the never-expected missing pair.
fn paired_rect(rect: Option<&BndRect>) -> Rect {
    rect.map(|corners| rect_from_corners(corners.x1, corners.y1, corners.x2, corners.y2))
        .unwrap_or_else(unplaced_rect)
}

/// One scene placement from a shared-pairing result.
fn entry_placement(placed: PlacedControl<'_>) -> ScenePlacement {
    place_entry(placed.entry, paired_rect(placed.rect))
}

/// Playing subset: behind-highway scene first (entries 0–212,
/// anchor-placed, clipped clear of lanes and HUD at paint), then the
/// static backdrop panels, then the bound run (entries 213–229)
/// paired with the 17 rects by code ordinal (`EXIT` = ordinal 1 →
/// rect 0; the run is capped at the rect count, so the tail past
/// `VOLUME` rides unloaded). Backdrops paint first so the bound
/// gauges and buttons keep their top slot. Container-coded entries
/// stay out in every run; a window ordinal missing the table stages
/// the all-zero rect (anchor draw), like the bound path.
fn bind_playing_subset(block: &ControlBlock, bnd: &Bnd) -> Vec<ScenePlacement> {
    let entries = block.all_entries();
    let mut placed = bind_playing_behind(&entries);
    placed.extend(bind_playing_backdrops(&entries));
    placed.extend(
        pair_entries_with_rects(block, bnd)
            .into_iter()
            .skip(PLAYING_SUBSET_FIRST_ENTRY)
            .take(bnd.rects.len())
            .filter(|bound| entry_takes_rect(bound.entry))
            .map(entry_placement),
    );
    placed
}

/// One placement from an entry plus its native layout rect.
fn place_entry(entry: &ControlEntry, rect: Rect) -> ScenePlacement {
    ScenePlacement {
        id: entry.id.clone(),
        code: entry.code,
        sprite: entry.sprite.clone(),
        rect,
    }
}

/// Load every named sprite body, skipping missing or undecodable
/// bodies (one bad sprite never blanks the scene). Names that miss
/// the archive — including scrollbar parameters, which are never
/// files (recipe step 9) — or fail to decode contribute nothing.
/// Stage-less form: `_` runtime refs take the bare stem (see
/// [`load_archive_sprite_for_stage`]).
#[must_use]
pub fn load_scene_figures(archive: &[u8], names: &[String]) -> HashMap<String, AvatarFigure> {
    load_scene_figures_for_stage(archive, names, None)
}

/// [`load_scene_figures`] through the room arena stage: `_` runtime
/// refs resolve their stage-numbered siblings (applier `0x4e7850`,
/// [`load_archive_sprite_for_stage`]) — `Playing_BG<stage>.ojs` and
/// the stage-keyed judgement/combo/click/jam kits — while every
/// other name loads verbatim.
#[must_use]
pub fn load_scene_figures_for_stage(
    archive: &[u8],
    names: &[String],
    arena_stage: Option<u16>,
) -> HashMap<String, AvatarFigure> {
    let mut figures = HashMap::new();
    for name in names {
        if figures.contains_key(name) {
            continue;
        }
        if let Some(figure) = load_archive_sprite_for_stage(archive, name, arena_stage) {
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

/// Frame-space origin of a placement: the native `.bnd` rect origin
/// scaled to the frame (the rect itself stays native 800x600; the
/// scale is the one present-tense stand-in). Bound placements paint
/// at this origin alone; unbound (all-zero rect) placements anchor
/// at the frame origin, so the record anchor — scaled by
/// [`anchor_in_frame`] — positions the art.
#[must_use]
pub fn placement_origin_in_frame(
    placement: &ScenePlacement,
    frame_w: usize,
    frame_h: usize,
) -> (i32, i32) {
    let scaled = scale_rect_to_frame(&placement.rect, frame_w, frame_h);
    (scaled.x, scaled.y)
}

/// Paint every placed figure in file order (verified step 8: the
/// background lands first and later entries cover it). Each layer
/// blits 1:1 at the frame-scaled rect origin — record anchors join
/// only on unbound placements, whose records carry absolute canvas
/// anchors (verified: `Playing_Menu.ojs` `(204,558)`) scaled to the
/// frame like bound origins ([`anchor_in_frame`]), while bound
/// bodies' anchors duplicate their rect origins byte-exact —
/// unclipped; the rect positions the blit and never clips (verified
/// draw path). Placements without a loaded figure are skipped, and
/// live UI ink paints on top in the callers.
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

/// Paint one placed figure, if its sprite loaded: every layer blits
/// 1:1 at the frame-scaled rect origin — plus its record anchor,
/// scaled to the frame, only while no rect claims the entry (bound
/// bodies' anchors duplicate their rect origins, so adding both
/// doubles them), unclipped.
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
    let (origin_x, origin_y) = placement_origin_in_frame(placement, frame_w, frame_h);
    if placement.rect.w != 0 || placement.rect.h != 0 {
        figure.paint_without_anchors(frame, origin_x, origin_y);
    } else {
        figure.paint_scaled_anchors(frame, origin_x, origin_y, frame_w, frame_h);
    }
}

/// True for a behind-highway placement: its sprite names the venue
/// or a note background, quoted or resolved (see
/// [`is_playing_behind_sprite`)).
fn placement_is_behind(placement: &ScenePlacement) -> bool {
    placement
        .sprite
        .as_deref()
        .is_some_and(is_playing_behind_sprite)
}

/// Paint the behind-highway placements, skipping every pixel inside
/// `exclusions` (the lane columns the Playing caller passes): the
/// exe paints the backdrop first of all and later passes cover it,
/// so the art fills the frame whole and only the falling lanes stay
/// uncovered. Like every painter, the rect only positions; the
/// clip comes from the exclusions and the frame alone.
pub fn paint_behind_placements(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
    exclusions: &[Rect],
) {
    for placement in placements
        .iter()
        .filter(|placed| placement_is_behind(placed))
    {
        paint_behind_placement(frame, placement, figures, exclusions);
    }
}

/// Paint every placement except the behind-highway scene, verbatim
/// in file order: overlay panels and bound gauges keep their top
/// slot over the clipped scene below.
pub fn paint_foreground_placements(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
) {
    let (frame_w, frame_h) = (frame.w, frame.h);
    for placement in placements
        .iter()
        .filter(|placed| !placement_is_behind(placed))
    {
        paint_placed_figure(frame, placement, figures, frame_w, frame_h);
    }
}

/// Paint one behind-highway figure, if its sprite loaded: each layer
/// blits 1:1 at its rect origin plus its record anchor (an all-zero
/// rect leaves the origin at the frame corner), clipped to the
/// fragments outside `exclusions`.
fn paint_behind_placement(
    frame: &mut Frame<'_>,
    placement: &ScenePlacement,
    figures: &HashMap<String, AvatarFigure>,
    exclusions: &[Rect],
) {
    let Some(name) = placement.sprite.as_ref() else {
        return;
    };
    let Some(figure) = figures.get(name) else {
        return;
    };
    let (origin_x, origin_y) = placement_origin_in_frame(placement, frame.w, frame.h);
    paint_behind_layers(frame, figure, origin_x, origin_y, exclusions);
}

/// Paint every layer of one figure through its visible fragments:
/// the layer box (rect origin plus frame-scaled anchor) minus the
/// exclusion zones.
fn paint_behind_layers(
    frame: &mut Frame<'_>,
    figure: &AvatarFigure,
    origin_x: i32,
    origin_y: i32,
    exclusions: &[Rect],
) {
    for index in 0..figure.len() {
        if let Some(layer) = figure.layer(index) {
            let (dx, dy) = anchor_in_frame(layer.dx, layer.dy, frame.w, frame.h);
            let layer_box = Rect {
                x: origin_x + dx,
                y: origin_y + dy,
                w: u32::try_from(layer.image.w).unwrap_or(u32::MAX),
                h: u32::try_from(layer.image.h).unwrap_or(u32::MAX),
            };
            for fragment in subtract_rect(&layer_box, exclusions) {
                blit_rgb555_clipped(
                    frame,
                    &layer.image.view(),
                    origin_x + dx,
                    origin_y + dy,
                    Some(layer.colorkey),
                    Some(&fragment),
                );
            }
        }
    }
}

/// Left/top/right/bottom edges of one rect in `i64` space (widths
/// add without overflowing `i32`).
fn rect_edges(rect: &Rect) -> (i64, i64, i64, i64) {
    let left = i64::from(rect.x);
    let top = i64::from(rect.y);
    (left, top, left + i64::from(rect.w), top + i64::from(rect.h))
}

/// Intersection of two rects (`None` when they do not overlap).
fn intersect_rects(first: &Rect, second: &Rect) -> Option<Rect> {
    let (first_left, first_top, first_right, first_bottom) = rect_edges(first);
    let (second_left, second_top, second_right, second_bottom) = rect_edges(second);
    let (left, top) = (first_left.max(second_left), first_top.max(second_top));
    let (right, bottom) = (
        first_right.min(second_right),
        first_bottom.min(second_bottom),
    );
    if right <= left || bottom <= top {
        return None;
    }
    Some(Rect {
        x: i32::try_from(left).unwrap_or(i32::MAX),
        y: i32::try_from(top).unwrap_or(i32::MAX),
        w: u32::try_from(right - left).unwrap_or(0),
        h: u32::try_from(bottom - top).unwrap_or(0),
    })
}

/// Fragments of `rect` outside every exclusion, in row order (top
/// strip, middle sides, bottom strip per cut).
fn subtract_rect(rect: &Rect, exclusions: &[Rect]) -> Vec<Rect> {
    let mut visible = vec![*rect];
    for cut in exclusions {
        visible = visible
            .into_iter()
            .flat_map(|kept| subtract_cut(&kept, cut))
            .collect();
    }
    visible
}

/// Fragments of `kept` outside one cut: the top strip above the
/// overlap, the bottom strip below it, and the middle row split
/// into left/right (empty pieces dropped).
fn subtract_cut(kept: &Rect, cut: &Rect) -> Vec<Rect> {
    let Some(overlap) = intersect_rects(kept, cut) else {
        return vec![*kept];
    };
    let (kept_left, kept_top, kept_right, kept_bottom) = rect_edges(kept);
    let (cut_left, cut_top, cut_right, cut_bottom) = rect_edges(&overlap);
    let mut fragments = Vec::new();
    push_fragment(&mut fragments, kept_left, kept_top, kept_right, cut_top);
    push_fragment(
        &mut fragments,
        kept_left,
        cut_bottom,
        kept_right,
        kept_bottom,
    );
    push_fragment(&mut fragments, kept_left, cut_top, cut_left, cut_bottom);
    push_fragment(&mut fragments, cut_right, cut_top, kept_right, cut_bottom);
    fragments
}

/// Push one edge-box fragment unless it is empty (coordinates stay
/// inside `i32` range: every edge comes from an input rect edge).
fn push_fragment(fragments: &mut Vec<Rect>, left: i64, top: i64, right: i64, bottom: i64) {
    if right <= left || bottom <= top {
        return;
    }
    fragments.push(Rect {
        x: i32::try_from(left).unwrap_or(i32::MAX),
        y: i32::try_from(top).unwrap_or(i32::MAX),
        w: u32::try_from(right - left).unwrap_or(0),
        h: u32::try_from(bottom - top).unwrap_or(0),
    });
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
    use std::path::Path;

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

    /// One red 1x1 layer carrying a nonzero record anchor (the 7.55
    /// corpus stores absolute placement anchors).
    fn anchored_red_figure(dx: i32, dy: i32) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx,
            dy,
            colorkey: 0xFFFF,
        });
        figure
    }

    fn timebar_like_placement(rect: Rect) -> ScenePlacement {
        ScenePlacement {
            id: "IDC_IMAGE_SPLAY_TIMEBAR".to_owned(),
            code: 0x1B50_0003,
            sprite: Some("Normal_TimeBar.ojs".to_owned()),
            rect,
        }
    }

    #[test]
    fn bound_placements_paint_at_rect_origin_without_anchors() {
        // Verified duplication on 7.55: bound bodies carry absolute
        // record anchors equal to their rect origins
        // (`Normal_TimeBar.ojs` (489,556) == `State_Playing.bnd`
        // rect[2], `Playing_Exit.ojs` (751,554) == rect[0],
        // `Wait_TeamA.ojs` (652,65) == `State_Waiting.bnd` rect[3]),
        // so the painter must not add the anchor again — the red
        // pixel lands on the rect origin, not rect + anchor.
        let figure = anchored_red_figure(489, 556);
        let figures = HashMap::from([("Normal_TimeBar.ojs".to_owned(), figure)]);
        let placement = timebar_like_placement(Rect {
            x: 489,
            y: 556,
            w: 254,
            h: 10,
        });
        let mut buf = vec![0u32; 800 * 600];
        paint_placed_figure(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placement,
            &figures,
            800,
            600,
        );
        assert_eq!(buf[556 * 800 + 489], o2jam_render::pack_xrgb(255, 0, 0));
    }

    #[test]
    fn unbound_placements_still_paint_at_record_anchors() {
        // The all-zero rect leaves the origin at the frame corner, so
        // the record anchor alone positions the art (verified:
        // `Playing_Menu.ojs` (204,558), `Playing_BG.ojs` (204,0)).
        // Native frames scale anchors 1:1.
        let figure = anchored_red_figure(489, 556);
        let figures = HashMap::from([("Normal_TimeBar.ojs".to_owned(), figure)]);
        let placement = timebar_like_placement(Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        });
        let mut buf = vec![0u32; 800 * 600];
        paint_placed_figure(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placement,
            &figures,
            800,
            600,
        );
        assert_eq!(buf[556 * 800 + 489], o2jam_render::pack_xrgb(255, 0, 0));
    }

    #[test]
    fn unbound_anchors_scale_with_the_frame() {
        // Native anchors keep exe proportions on scaled frames: the
        // venue anchor (204,0) lands at (102,0) on a 400x300 frame —
        // the same frame-relative spot the exe draws at 800x600.
        let figure = anchored_red_figure(204, 0);
        let figures = HashMap::from([("Normal_TimeBar.ojs".to_owned(), figure)]);
        let placement = timebar_like_placement(Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        });
        let mut buf = vec![0u32; 400 * 300];
        paint_placed_figure(
            &mut Frame {
                buf: &mut buf,
                w: 400,
                h: 300,
            },
            &placement,
            &figures,
            400,
            300,
        );
        assert_eq!(buf[102], o2jam_render::pack_xrgb(255, 0, 0));
        assert_eq!(buf[0], 0);
        assert_eq!(anchor_in_frame(204, 0, 400, 300), (102, 0));
        assert_eq!(anchor_in_frame(204, 558, 800, 600), (204, 558));
    }

    fn room_control_text() -> String {
        [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "IDC_LIST 0X13500001 \"Room_List.ojs\"",
            "IDC_BUTTON 0X13200002 \"Room_Button.ojs\"",
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
        // Verified pairing: rect index `(code & 0xFFF) − 1`. The
        // block-self row (low word 0) stays out; IDC_LIST (ordinal 1)
        // takes rect 0 and IDC_BUTTON (ordinal 2) takes rect 1.
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
        assert_eq!(placed[1].rect.x, 400);
    }

    #[test]
    fn scene_rect_misses_stage_the_all_zero_rect() {
        // Verified pairing: ordinal 2 misses the 1-rect table, and a
        // miss stores the all-zero rect instead of skipping the entry
        // — the sprite draws at its record anchor from the origin.
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(8, 8, 16, 16)])).unwrap();
        let placed = bind_scene_rects(block, &bnd);
        assert_eq!(placed.len(), 2);
        assert_eq!(placed[0].id, "IDC_LIST");
        assert_eq!(
            placed[0].rect,
            o2jam_render::Rect {
                x: 8,
                y: 8,
                w: 8,
                h: 8
            }
        );
        assert_eq!(placed[1].id, "IDC_BUTTON");
        assert_eq!(
            placed[1].rect,
            o2jam_render::Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0
            }
        );
    }

    /// Playing-shaped block: 235 entries in file order with the
    /// bound run at 213-229 (`EXIT`..`VOLUME`) carrying code
    /// ordinals 1-17, like the real block.
    fn playing_block() -> ControlBlock {
        let mut bound = Vec::new();
        bound.push(ControlEntry {
            id: "IDC_STATE_PLAY".to_owned(),
            code: 0x1B00_0000,
            sprite: None,
        });
        for index in 1..235 {
            bound.push(ControlEntry {
                id: format!("IDC_FILL_{index:03}"),
                code: 0x1B50_0000 + index,
                sprite: None,
            });
        }
        bound[213] = ControlEntry {
            id: "IDC_BUTTON_SPLAY_EXIT".to_owned(),
            code: 0x1B20_0001,
            sprite: Some("Playing_Exit.ojs".to_owned()),
        };
        for (index, entry) in bound.iter_mut().enumerate().take(229).skip(214) {
            *entry = ControlEntry {
                id: format!("IDC_BOUND_{index:03}"),
                code: 0x1B50_0000 + u32::try_from(index - 212).unwrap_or(0),
                sprite: None,
            };
        }
        bound[229] = ControlEntry {
            id: "IDC_TEXT_SPLAY_VLOLUME".to_owned(),
            code: 0x1B40_0011,
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
    fn playing_rects_map_the_bound_run_by_ordinal() {
        // Verified pairing: the run's codes carry ordinals 1-17, so
        // ordinal rect lookup lays out entries 213-229 over the 17
        // rects in file order — never the fill entries before them.
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
    fn playing_backdrops_stage_ahead_of_bound_rects() {
        // Static edge panels among entries 0-212 stage as anchor
        // placements ahead of the 17 bound rects; the behind scene
        // (`_Playing_BG.ojs`, `Note_BG.ojs`) stages first of all and
        // paints clipped clear of lanes at paint time.
        let mut block = playing_block();
        block.bound[113].sprite = Some("_Playing_BG.ojs".to_owned());
        block.bound[114].sprite = Some("Note_BG.ojs".to_owned());
        block.bound[115].sprite = Some("Playing_Menu.ojs".to_owned());
        block.bound[125].sprite = Some("Life_gauge_back.ojs".to_owned());
        block.bound[126].sprite = Some("Life_gauge_left_bar.ojs".to_owned());
        block.bound[127].sprite = Some("Jamgauge_left_bar.ojs".to_owned());
        block.bound[128].sprite = Some("Jam_gauge_text.ojs".to_owned());
        block.bound[129].sprite = Some("Main_level.ojs".to_owned());
        let corners: Vec<(u32, u32, u32, u32)> =
            (0..17).map(|i| (751 + i, 554, 793 + i, 596)).collect();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&corners)).unwrap();
        let placed = bind_scene_rects(&block, &bnd);
        assert_eq!(placed.len(), 2 + PLAYING_BACKDROP_PANELS.len() + 17);
        let behind = &placed[..2];
        assert_eq!(behind[0].sprite.as_deref(), Some("_Playing_BG.ojs"));
        assert_eq!(behind[1].sprite.as_deref(), Some("Note_BG.ojs"));
        assert!(is_playing_behind_sprite(
            behind[0].sprite.as_deref().unwrap()
        ));
        for staged in behind {
            assert_eq!(
                staged.rect,
                o2jam_render::Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0
                }
            );
        }
        let backdrops = &placed[2..2 + PLAYING_BACKDROP_PANELS.len()];
        for (backdrop, panel) in backdrops.iter().zip(PLAYING_BACKDROP_PANELS) {
            assert_eq!(backdrop.sprite.as_deref(), Some(panel));
            assert_eq!(
                backdrop.rect,
                o2jam_render::Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0
                }
            );
        }
        assert!(!backdrops
            .iter()
            .any(|backdrop| is_playing_behind_sprite(backdrop.sprite.as_deref().unwrap_or(""))));
        assert_eq!(
            placed[2 + PLAYING_BACKDROP_PANELS.len()].id,
            "IDC_BUTTON_SPLAY_EXIT"
        );
    }

    #[test]
    fn behind_names_match_quoted_and_resolved_forms() {
        assert!(is_playing_behind_sprite("_Playing_BG.ojs"));
        assert!(is_playing_behind_sprite("Playing_BG.ojs"));
        assert!(is_playing_behind_sprite("Note_BG.ojs"));
        assert!(is_playing_behind_sprite("3Key_Note_BG.ojs"));
        assert!(is_playing_behind_sprite("P2P3KeyNote_BG.ojs"));
        assert!(is_playing_behind_sprite("P2P7KeyNote_BG.ojs"));
        assert!(!is_playing_behind_sprite("Playing_Menu.ojs"));
        assert!(!is_playing_behind_sprite("Missing.ojs"));
    }

    #[test]
    fn archive_sprite_load_ignores_ascii_case() {
        // 7.55 spells the level plate `main_level.ojs` while the
        // block names `Main_level.ojs`: the case fallback still
        // stages it, and true misses still return `None`.
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[("main_level.ojs", &red)]);
        assert_eq!(
            load_archive_sprite(&archive, "Main_level.ojs")
                .unwrap()
                .len(),
            1
        );
        assert!(load_archive_sprite(&archive, "Missing.ojs").is_none());
    }

    #[test]
    fn runtime_behind_ref_loads_through_its_bare_stem() {
        // The `_Playing_BG.ojs` block ref is runtime-deferred, never
        // an archive miss: the stage-less loader falls back to the
        // bare stem beside its numbered siblings (the exe's flag-clear
        // bare arm). True misses stay `None`.
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[("Playing_BG.ojs", &red)]);
        assert_eq!(
            load_archive_sprite(&archive, "_Playing_BG.ojs")
                .unwrap()
                .len(),
            1
        );
        assert!(load_archive_sprite(&archive, "_Missing.ojs").is_none());
        assert!(load_archive_sprite(&archive, "Missing.ojs").is_none());
    }

    #[test]
    fn arena_stage_loads_the_numbered_backdrop_sibling() {
        // With the room stage, `_Playing_BG.ojs` resolves through the
        // applier rule to `Playing_BG7.ojs` — the numbered body wins,
        // the bare stem is never consulted.
        let bare = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let numbered = sprite55_blob(1, 0x0000, 1, 1, &[0xE0, 0x03]);
        let archive = opa_blob(&[("Playing_BG.ojs", &bare), ("Playing_BG7.ojs", &numbered)]);
        let figure = load_archive_sprite_for_stage(&archive, "_Playing_BG.ojs", Some(7)).unwrap();
        let mut buf = vec![0u32; 1];
        figure.paint(
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
    fn stage_resolved_backdrop_miss_leaves_the_slot_empty() {
        // Only stage 1 exists: stage 7's numbered name misses and the
        // exe's slot loader leaves the slot empty — no bare-stem
        // fallback once a stage names the sibling.
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[("Playing_BG.ojs", &red), ("Playing_BG1.ojs", &red)]);
        assert!(load_archive_sprite_for_stage(&archive, "_Playing_BG.ojs", Some(7)).is_none());
        assert!(load_archive_sprite_for_stage(&archive, "_Playing_BG.ojs", Some(1)).is_some());
    }

    #[test]
    fn stage_unsettled_kit_arms_take_the_bare_stem() {
        // `Note_Click` builds a variant kit the stage alone cannot
        // name (`%s%d_%d.ojt`), and `Note_LongCombo` stays bare: both
        // keep the bare-stem arm even with a stage.
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[("Note_Click.ojs", &red), ("Note_LongCombo.ojs", &red)]);
        assert!(load_archive_sprite_for_stage(&archive, "_Note_Click.ojs", Some(7)).is_some());
        assert!(load_archive_sprite_for_stage(&archive, "_Note_LongCombo.ojs", Some(7)).is_some());
    }

    #[test]
    fn scene_figures_resolve_stage_numbered_names() {
        // The scene loader passes the stage through: the venue entry
        // stages the numbered sibling's body under the written ref.
        let bare = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let numbered = sprite55_blob(1, 0x0000, 1, 1, &[0x1F, 0x00]);
        let archive = opa_blob(&[("Playing_BG.ojs", &bare), ("Playing_BG5.ojs", &numbered)]);
        let figures =
            load_scene_figures_for_stage(&archive, &["_Playing_BG.ojs".to_owned()], Some(5));
        let figure = figures.get("_Playing_BG.ojs").unwrap();
        let mut buf = vec![0u32; 1];
        figure.paint(
            &mut Frame {
                buf: &mut buf,
                w: 1,
                h: 1,
            },
            0,
            0,
        );
        assert_eq!(buf[0], o2jam_render::pack_xrgb(0, 0, 255));
    }

    #[test]
    fn behind_paint_clips_exclusion_zones() {
        // One 4-wide red layer at the origin with the middle two
        // columns excluded: only the outer pixels paint.
        let red = sprite55_blob(
            1,
            0xFFFF,
            4,
            1,
            &[0x00, 0x7C, 0x00, 0x7C, 0x00, 0x7C, 0x00, 0x7C],
        );
        let archive = opa_blob(&[("Playing_BG.ojs", &red)]);
        let figures = load_scene_figures(&archive, &["_Playing_BG.ojs".to_owned()]);
        let placed = vec![ScenePlacement {
            id: "IDC_IMAGE_SPLAY_BACK1".to_owned(),
            code: 0x1B50_0071,
            sprite: Some("_Playing_BG.ojs".to_owned()),
            rect: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
        }];
        let exclusions = vec![Rect {
            x: 1,
            y: 0,
            w: 2,
            h: 1,
        }];
        let mut buf = vec![0u32; 4];
        paint_behind_placements(
            &mut Frame {
                buf: &mut buf,
                w: 4,
                h: 1,
            },
            &placed,
            &figures,
            &exclusions,
        );
        let red_ink = o2jam_render::pack_xrgb(255, 0, 0);
        assert_eq!(buf, vec![red_ink, 0, 0, red_ink]);
    }

    #[test]
    fn foreground_paint_skips_behind_sprites() {
        // The split paints disjoint sets: foreground paints the
        // overlay sprite only, behind paints the scene sprite only.
        let mut behind = AvatarFigure::default();
        behind.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let mut overlay = AvatarFigure::default();
        overlay.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x1F, 0x00], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let placed = vec![
            ScenePlacement {
                id: "IDC_IMAGE_SPLAY_BACK1".to_owned(),
                code: 0,
                sprite: Some("Playing_BG.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
            },
            ScenePlacement {
                id: "IDC_IMAGE_SPLAY_BACK3".to_owned(),
                code: 0,
                sprite: Some("Playing_Menu.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
            },
        ];
        let figures = HashMap::from([
            ("Playing_BG.ojs".to_owned(), behind),
            ("Playing_Menu.ojs".to_owned(), overlay),
        ]);
        let mut foreground = vec![0u32; 1];
        paint_foreground_placements(
            &mut Frame {
                buf: &mut foreground,
                w: 1,
                h: 1,
            },
            &placed,
            &figures,
        );
        assert_eq!(foreground, vec![o2jam_render::pack_xrgb(0, 0, 255)]);
        let mut behind_buf = vec![0u32; 1];
        paint_behind_placements(
            &mut Frame {
                buf: &mut behind_buf,
                w: 1,
                h: 1,
            },
            &placed,
            &figures,
            &[],
        );
        assert_eq!(behind_buf, vec![o2jam_render::pack_xrgb(255, 0, 0)]);
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

    fn stub_block_text() -> String {
        [
            "NUMBER_OF_STATE 0x01",
            "STATE_PLAYING 0x00",
            "STATE_TUTORIAL 0x00",
            "STATE_ROOM 0x01",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "}",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn stub_blocks_stage_nothing() {
        // Recipe step 3: the bare `0x00` stubs draw nothing, even
        // with a bind section on hand.
        let list = o2jam_file::parse_control_list(&stub_block_text()).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(0, 0, 800, 600)])).unwrap();
        for stub in ["STATE_PLAYING", "STATE_TUTORIAL"] {
            let block = scene_block(&list, stub).unwrap();
            assert!(is_stub_block(block));
            assert!(bind_scene_rects(block, &bnd).is_empty());
            assert!(anchor_placements(block).is_empty());
        }
    }

    #[test]
    fn named_block_with_entries_is_no_stub() {
        let list = o2jam_file::parse_control_list(&stub_block_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        assert!(!is_stub_block(block));
    }

    #[test]
    fn container_coded_rows_take_no_rect() {
        // Entry codes gate visibility: a zero low word marks a
        // scene/container row, so it stays unplaced even with a
        // sprite and a spare rect.
        let list = o2jam_file::parse_control_list(
            &[
                "NUMBER_OF_STATE 0x01",
                "STATE_ROOM 0x02",
                "{",
                "BOUND \"State_Room.bnd\"",
                "IDC_ROOM 0X13000000",
                "IDC_LIST 0X13500001 \"Room_List.ojs\"",
                "IDC_CONTAINER 0X13000000 \"Room_List.ojs\"",
                "}",
                "",
            ]
            .join("\n"),
        )
        .unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        let bnd =
            o2jam_file::parse_bnd(&bnd_blob(&[(0, 0, 400, 300), (400, 0, 800, 600)])).unwrap();
        let placed = bind_scene_rects(block, &bnd);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].id, "IDC_LIST");
        assert!(entry_state_id(0x1B20_0001) == 0x1B);
        assert!(entry_takes_rect(block.all_entries()[1]));
    }

    #[test]
    fn bound_layers_paint_at_rect_origin_alone() {
        // Corrected draw math (corpus anchor audit): 7.55 sprite
        // records store absolute canvas anchors that duplicate their
        // bound rect origins (`Normal_TimeBar.ojs` (489,556) ==
        // `State_Playing.bnd` rect[2], `Wait_TeamA.ojs` (652,65) ==
        // `State_Waiting.bnd` rect[3]), so a bound body paints at the
        // rect origin alone — never rect + anchor, which doubles the
        // origin and lands off-canvas. The 2-wide body over the
        // 1-wide rect (10,20,11,21) paints both pixels; the button
        // pixel lands at its rect origin (40,30), its stale record
        // anchor (2,3) ignored.
        let wide = sprite55_blob(1, 0xFFFF, 2, 1, &[0x00, 0x7C, 0x00, 0x7C]);
        let archive = opa_blob(&[("Room_List.ojs", &wide)]);
        let mut figures = load_scene_figures(&archive, &["Room_List.ojs".to_owned()]);
        let mut button = AvatarFigure::default();
        button.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x1F, 0x00], 1, 1).unwrap(),
            dx: 2,
            dy: 3,
            colorkey: 0xFFFF,
        });
        figures.insert("Room_Button.ojs".to_owned(), button);
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(10, 20, 11, 21), (40, 30, 44, 34)])).unwrap();
        let placed = bind_scene_rects(block, &bnd);
        let mut buf = vec![0u32; 800 * 600];
        paint_scene_figures(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placed,
            &figures,
        );
        let red_ink = o2jam_render::pack_xrgb(255, 0, 0);
        let blue_ink = o2jam_render::pack_xrgb(0, 0, 255);
        assert_eq!(buf[20 * 800 + 10], red_ink);
        assert_eq!(buf[20 * 800 + 11], red_ink);
        assert_eq!(buf[21 * 800 + 10], 0);
        assert_eq!(buf[30 * 800 + 40], blue_ink);
        assert_eq!(buf[33 * 800 + 42], 0);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn unbound_layers_paint_at_the_record_anchor() {
        // No `.bnd`: the sprite's own anchor positions it, 1:1.
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 5,
            dy: 7,
            colorkey: 0xFFFF,
        });
        let placed = anchor_placements(
            &o2jam_file::parse_control_list(
                &[
                    "NUMBER_OF_STATE 0x01",
                    "STATE_CHANNEL 0x01",
                    "{",
                    "BOUND",
                    "IDC_STATE_CHANNEL 0X12000000",
                    "IDC_BAR 0X12000000",
                    "IDC_SELECT 0X12500001 \"Dialog_ChannelSelectBar.ojs\"",
                    "}",
                    "",
                ]
                .join("\n"),
            )
            .unwrap()
            .blocks
            .pop()
            .unwrap(),
        );
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].id, "IDC_SELECT");
        assert_eq!(placed[0].code, 0x1250_0001);
        let mut buf = vec![0u32; 800 * 600];
        paint_scene_figures(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placed,
            &std::collections::HashMap::from([("Dialog_ChannelSelectBar.ojs".to_owned(), figure)]),
        );
        assert_eq!(buf[7 * 800 + 5], o2jam_render::pack_xrgb(255, 0, 0));
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn anchor_placements_skip_artless_and_scrollbar_entries() {
        // Sprite-less text rows and scrollbar parameters draw no
        // art; runtime-deferred names still stage for the loader.
        let block = o2jam_file::parse_control_list(
            &[
                "NUMBER_OF_STATE 0x01",
                "STATE_CHANNEL 0x27",
                "{",
                "BOUND",
                "IDC_STATE_CHANNEL 0X12000000",
                "IDC_BACK 0X12100001",
                "IDC_SCROLL 0X12021002 \"O2_SBS_VERT\t4\t\t100\"",
                "IDC_BG 0X12500003 \"_Playing_BG.ojs\"",
                "}",
                "",
            ]
            .join("\n"),
        )
        .unwrap()
        .blocks
        .pop()
        .unwrap();
        let placed = anchor_placements(&block);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].sprite.as_deref(), Some("_Playing_BG.ojs"));
    }

    #[test]
    fn figure_layers_index_in_paint_order() {
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let sprite = o2jam_file::parse_sprite55(&red).unwrap();
        let figure = AvatarFigure::from_sprite55(&sprite);
        assert_eq!(figure.layer(0).unwrap().dx, 0);
        assert!(figure.layer(1).is_none());
        assert!(AvatarFigure::default().layer(0).is_none());
    }

    /// Archive bytes under git-ignored `install-root` (`None` skips).
    fn install_archive(name: &str) -> Option<Vec<u8>> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../install-root/O2Jam/Image")
                .join(name),
        )
        .ok()
    }

    /// Parse a `ControlList_*.txt` entry out of an archive.
    fn archive_control_list(archive: &[u8], name: &str) -> Option<o2jam_file::ControlList> {
        let entries = o2jam_file::parse_opi(archive).ok()?;
        let entry = entries.iter().find(|entry| entry.name == name)?;
        let size = usize::try_from(entry.size).ok()?;
        let end = entry.data_offset.checked_add(size)?;
        let body = archive.get(entry.data_offset..end)?;
        let text = o2jam_file::decode_control_text(body).ok()?;
        o2jam_file::parse_control_list(&text).ok()
    }

    /// Parse a `.bnd` layout entry out of an archive.
    fn archive_bind(archive: &[u8], name: &str) -> Option<o2jam_file::Bnd> {
        let entries = o2jam_file::parse_opi(archive).ok()?;
        let entry = entries.iter().find(|entry| entry.name == name)?;
        let size = usize::try_from(entry.size).ok()?;
        let end = entry.data_offset.checked_add(size)?;
        let body = archive.get(entry.data_offset..end)?;
        o2jam_file::parse_bnd(body).ok()
    }

    #[test]
    fn every_bound_entry_paints_on_its_rect_origin() {
        // Multi-entry proof of the shared placement rule on real 7.55
        // bytes (skips without `install-root`): every decodable
        // `.bnd`-claimed placement of `STATE_PLAYING` and `STATE_ROOM`
        // lands a pixel inside its own rect. Bound bodies' record
        // anchors duplicate their rect origins byte-exact
        // (docs/bnd-semantics.md; pairing `0x548640`, ev `3a869bb7…`),
        // so the painter paints at the rect origin alone.
        let Some(interface) = install_archive("Interface1.opi") else {
            return;
        };
        let Some(playing) = install_archive("Playing1.opi") else {
            return;
        };
        let Some(interface_list) = archive_control_list(&interface, "ControlList_Interface.txt")
        else {
            return;
        };
        let Some(playing_list) = archive_control_list(&playing, "ControlList_Playing.txt") else {
            return;
        };
        let scenes = [
            (&interface, &interface_list, "STATE_ROOM", "State_Room.bnd"),
            (
                &playing,
                &playing_list,
                "STATE_PLAYING",
                "State_Playing.bnd",
            ),
        ];
        let background = 0x11_1111;
        let mut proven = 0usize;
        for (archive, list, block_name, bind_name) in scenes {
            let Some(block) = scene_block(list, block_name) else {
                continue;
            };
            let Some(bind) = archive_bind(archive, bind_name) else {
                continue;
            };
            for placed in o2jam_file::pair_entries_with_rects(block, &bind) {
                let Some(sprite) = placed.entry.sprite.as_deref() else {
                    continue;
                };
                let Some(figure) = load_archive_sprite(archive, sprite) else {
                    continue;
                };
                if figure.is_empty() {
                    continue;
                }
                let Some(rect) = placed.rect else {
                    continue;
                };
                let width = rect.x2.saturating_sub(rect.x1);
                let height = rect.y2.saturating_sub(rect.y1);
                if width == 0 || height == 0 {
                    continue;
                }
                let placement = ScenePlacement {
                    id: placed.entry.id.clone(),
                    code: placed.entry.code,
                    sprite: Some(sprite.to_owned()),
                    rect: Rect {
                        x: i32::try_from(rect.x1).unwrap_or(0),
                        y: i32::try_from(rect.y1).unwrap_or(0),
                        w: width,
                        h: height,
                    },
                };
                let mut buf = vec![background; 800 * 600];
                paint_placed_figure(
                    &mut Frame {
                        buf: &mut buf,
                        w: 800,
                        h: 600,
                    },
                    &placement,
                    &HashMap::from([(sprite.to_owned(), figure)]),
                    800,
                    600,
                );
                let painted = (rect.y1..rect.y2)
                    .flat_map(|y| {
                        (rect.x1..rect.x2).map(move |x| {
                            (
                                i32::try_from(x).unwrap_or(-1),
                                i32::try_from(y).unwrap_or(-1),
                            )
                        })
                    })
                    .any(|(x, y)| {
                        if x < 0 || y < 0 {
                            return false;
                        }
                        let (ux, uy) = (
                            usize::try_from(x).unwrap_or(0),
                            usize::try_from(y).unwrap_or(0),
                        );
                        buf.get(uy * 800 + ux).copied().unwrap_or(background) != background
                    });
                assert!(
                    painted,
                    "{} ({sprite}) painted nothing inside its rect {:?}",
                    placed.entry.id, placement.rect
                );
                proven += 1;
            }
        }
        assert!(
            proven >= 40,
            "expected the bound-run proof to cover 40+ entries, got {proven}"
        );
    }
}
