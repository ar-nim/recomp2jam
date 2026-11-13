//! Avatar figure: decoded sprite-55 images, drawable on results.
//!
//! One parsed body becomes one figure: every raw-RGB555 image is
//! decoded at its record anchor, strided atlases contribute their
//! animation frames (see [`AvatarFigure::from_sprite55_strided`]),
//! prefixed `.oji` bodies compose their row spans onto full frames
//! (see [`AvatarFigure::from_sprite55_prefixed`]), `.oja` effect
//! animations decode their row-group planes per frame — tabulated
//! records plus the recordless trailing frames (see
//! [`AvatarFigure::from_oja55`]) — and anything else (the compressed
//! `.oji` outlier frames) is skipped — their codec is still TBD in
//! `o2jam-file`. Equipped parts stack on as extra layers; which
//! filenames those are comes from [`pick_wear_sprites`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use o2jam_file::{
    classify_sprite, find_opi_entry, opi_entry_body, pair_entries_with_rects, resolve_sprite_entry,
    AvatarItemTable, Bnd, BndRect, ControlBlock, ControlEntry, ControlList, Oja55, Oja55Frame,
    Oja55TrailingFrame, OpiEntry, PlacedControl, Sprite55, Sprite55Prefixed, Sprite55RowSpan,
    Sprite55Strided, SpriteResolution,
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

/// Idle-breathing cycle length for worn avatar figures (12 frames,
/// `docs/equip.md`: every probed worn body carries 12 chained
/// breathing frames). OPEN: the exe's ms/frame cadence is not yet
/// pinned (#176); the 250 ms value below reuses the net-drain
/// heartbeat until a Wine dwell capture pins it. Labelled OPEN.
pub const AVATAR_BREATHING_FRAME_COUNT: usize = 12;
/// Cadence of one breathing frame (ms). OPEN until a Wine dwell
/// capture pins it (`docs/equip.md`); 250 ms reuses the net-drain
/// heartbeat as the interim value.
pub const AVATAR_BREATHING_TICK_MS: u64 = 250;

/// Breathing frame index for a paint instant: advances one frame per
/// [`AVATAR_BREATHING_TICK_MS`] of paint-clock time, wrapping over
/// [`AVATAR_BREATHING_FRAME_COUNT`].
#[must_use]
pub fn avatar_breathing_frame(now_ms: u64) -> usize {
    (usize::try_from(now_ms / AVATAR_BREATHING_TICK_MS).unwrap_or(0)) % AVATAR_BREATHING_FRAME_COUNT
}

static AVATAR_BREATHING_NOW_MS: AtomicU64 = AtomicU64::new(0);

/// Record the paint-clock instant driving avatar breathing (called from
/// the paint seam). `0` until the first paint.
pub fn set_avatar_breathing_now_ms(now_ms: u64) {
    AVATAR_BREATHING_NOW_MS.store(now_ms, Ordering::Relaxed);
}

/// Paint-clock instant currently driving avatar breathing.
#[must_use]
pub fn avatar_breathing_now_ms() -> u64 {
    AVATAR_BREATHING_NOW_MS.load(Ordering::Relaxed)
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

impl FigureLayer {
    /// One `.oja` animation frame pasted onto its full `w*h` canvas:
    /// the stored transparent word 0 clears the canvas (`.oja`
    /// headers carry no colour-key word; claims A12 key `0x0000`),
    /// and each pixel-plane row span pastes at its `(x, y)` — the
    /// plane layout chains through the container parser
    /// ([`o2jam_file::parse_oja55`]). The record anchor becomes the
    /// layer anchor (negative anchors mark centred effects). The
    /// per-pixel alpha plane (claims C7: read straight after the
    /// pixels, uploaded raw) stays out — the layer model has no alpha
    /// channel yet, so frames paste opaque like the prefixed bodies
    /// and the intensity pass rides
    /// [`o2jam_file::Oja55Frame::alpha`] for a blended paint.
    #[must_use]
    pub(crate) fn from_oja_frame(frame: &Oja55Frame<'_>) -> Self {
        Self {
            image: oja_canvas(frame.w, frame.h, &frame.rows),
            dx: i32::from(frame.x),
            dy: i32::from(frame.y),
            colorkey: 0x0000,
        }
    }

    /// One recordless `.oja` trailing frame (frames past the record
    /// table; `Playing_Effect_Jam.oja` frames 10..15) pasted like a
    /// tabulated frame. No record exists, so the layer anchor is the
    /// body-wide `(0, 0)` every tabulated Jam record carries —
    /// inherited, never invented. The canvas-row mask plane rides
    /// [`o2jam_file::Oja55TrailingFrame::alpha`] unblended, like the
    /// tabulated alpha.
    #[must_use]
    pub(crate) fn from_oja_trailing_frame(frame: &Oja55TrailingFrame<'_>) -> Self {
        Self {
            image: oja_canvas(frame.w, frame.h, &frame.rows),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        }
    }
}

/// Paste `.oja` row spans onto a zero-cleared `w*h` canvas: the
/// stored transparent word 0 clears the canvas (`.oja` headers carry
/// no colour-key word; claims A12 key `0x0000`) and each row span
/// pastes at its `(x, y)`. The parser guarantees spans stay inside
/// their row, so uncovered pixels stay transparent — nothing is
/// invented.
fn oja_canvas(w: u16, h: u16, rows: &[Sprite55RowSpan<'_>]) -> Rgb555Image {
    let mut words = vec![0u16; usize::from(w) * usize::from(h)];
    for span in rows {
        let base = usize::from(span.y) * usize::from(w) + usize::from(span.x);
        let (pairs, _) = span.pixels.as_chunks::<2>();
        for (index, pair) in pairs.iter().enumerate() {
            words[base + index] = u16::from_le_bytes(*pair);
        }
    }
    Rgb555Image {
        w: usize::from(w),
        h: usize::from(h),
        words,
    }
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

    /// Decode every animation frame of a strided atlas: one layer per
    /// parser frame in stored order (record-major,
    /// [`Sprite55Strided::frame_data`]), each at its record anchor
    /// under the atlas colour key. Records sharing an anchor are
    /// animation frames — the `Note_Click` bodies pin that reading —
    /// so a single-frame view cuts through [`Self::layers_through`]
    /// (the exe image control paints one frame per view, loader
    /// default frame 0).
    #[must_use]
    pub fn from_sprite55_strided(sprite: &Sprite55Strided<'_>) -> Self {
        let mut layers = Vec::new();
        for image in &sprite.images {
            if !sprite.is_raw_rgb555(image) {
                continue;
            }
            for frame in 0..sprite.frames_per_image {
                if let Some(decoded) = Rgb555Image::decode(
                    sprite.frame_data(image, frame),
                    image.w as usize,
                    image.h as usize,
                ) {
                    layers.push(FigureLayer {
                        image: decoded,
                        dx: i32::from(image.x),
                        dy: i32::from(image.y),
                        colorkey: sprite.colorkey,
                    });
                }
            }
        }
        Self { layers }
    }

    /// Decode every frame of a `.oja` effect animation (codec gap ⑤):
    /// one layer per tabulated frame at its record anchor, then one
    /// layer per recordless trailing frame at the body-wide `(0, 0)`
    /// anchor — the tap/hold/jam frames chain through the container's
    /// plane layout ([`o2jam_file::parse_oja55`]); the paste helpers
    /// carry the canvas and the alpha caveats. For
    /// `Playing_Effect_Jam.oja` this yields all sixteen stored
    /// frames (ten tabulated + six recordless); gameplay consumers
    /// that stage the record table only (the Playing skin) keep the
    /// ten exe-visible frames.
    #[must_use]
    pub fn from_oja55(animation: &Oja55<'_>) -> Self {
        let mut layers: Vec<FigureLayer> = animation
            .frames
            .iter()
            .map(FigureLayer::from_oja_frame)
            .collect();
        layers.extend(
            animation
                .trailing_frames
                .iter()
                .map(FigureLayer::from_oja_trailing_frame),
        );
        Self { layers }
    }

    /// Decode every image of a prefixed body onto its full `w*h`
    /// canvas: the container colour key clears the canvas, then each
    /// row span pastes at its `(x, y)` — the parser guarantees spans
    /// stay inside their rows, and pixels no span covers stay
    /// colour-key (`Playing_Loding_Lankview.oji` trims both sides of
    /// its bottom rows for rounded corners; nothing is invented). One
    /// layer per image (a frame is one surface) at its record anchor
    /// under the container colour key.
    #[must_use]
    pub fn from_sprite55_prefixed(sprite: &Sprite55Prefixed<'_>) -> Self {
        let layers = sprite
            .images
            .iter()
            .zip(&sprite.rows)
            .map(|(image, rows)| {
                let mut words = vec![sprite.colorkey; image.w as usize * image.h as usize];
                for span in rows {
                    let base = span.y as usize * image.w as usize + span.x as usize;
                    let (pairs, _) = span.pixels.as_chunks::<2>();
                    for (index, pair) in pairs.iter().enumerate() {
                        words[base + index] = u16::from_le_bytes(*pair);
                    }
                }
                FigureLayer {
                    image: Rgb555Image {
                        w: image.w as usize,
                        h: image.h as usize,
                        words,
                    },
                    dx: i32::from(image.x),
                    dy: i32::from(image.y),
                    colorkey: sprite.colorkey,
                }
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

    /// The single-frame view of a multi-frame body: layers `0..=last`
    /// only. The exe image control paints one frame per view (the view
    /// draws the current frame's surface, `docs/ui-draw.md` steps 3–6;
    /// a frame setter swaps the image, it never stacks frames), so a
    /// mode-family frame cut loads through this
    /// ([`playing_member_frame`]).
    #[must_use]
    pub fn layers_through(&self, last: usize) -> Self {
        Self {
            layers: self
                .layers
                .iter()
                .take(last.saturating_add(1))
                .cloned()
                .collect(),
        }
    }

    /// Whether any layer carries a nonzero record anchor. Such a body
    /// authors its own absolute position (see
    /// [`placement_view_origin`]), so a bound rect never moves it;
    /// bodies without any anchor take their position from their
    /// placement's rect alone.
    #[must_use]
    pub fn has_anchor_offset(&self) -> bool {
        self.layers
            .iter()
            .any(|layer| layer.dx != 0 || layer.dy != 0)
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

    /// Anchored-extent top-left: the smallest layer anchor per axis
    /// (`(0, 0)` for an empty figure). Together with [`Self::extent`]
    /// this is the on-canvas box the layered body covers, so seat-
    /// based painters can center the whole figure, not its first
    /// layer.
    ///
    /// Visible-box origin: the smallest layer anchor per axis with
    /// no `(0, 0)` floor, so painters hanging the figure somewhere
    /// new (the result row slot) land its drawn pixels, not the
    /// canvas origin its layers were authored against. `(0, 0)` for
    /// an empty figure.
    #[must_use]
    pub fn visible_origin(&self) -> (i32, i32) {
        let mut origin: Option<(i32, i32)> = None;
        for layer in &self.layers {
            origin = Some(match origin {
                Some((x, y)) => (x.min(layer.dx), y.min(layer.dy)),
                None => (layer.dx, layer.dy),
            });
        }
        origin.unwrap_or((0, 0))
    }
    #[must_use]
    pub fn extent_origin(&self) -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        for layer in &self.layers {
            x = x.min(layer.dx);
            y = y.min(layer.dy);
        }
        (x, y)
    }

    /// Paint so the anchored-extent box centers in `seat`: the box
    /// ([`Self::extent_origin`]..[`Self::extent`]) centers per axis on
    /// the native canvas, the center scales into the frame like every
    /// record anchor, and every layer blits 1:1 at its anchor offset
    /// from the extent origin. Empty figures paint nothing. The
    /// Playing seat painter uses this — the worn figure's layers
    /// carry `avatar.opa` canvas anchors, so the extent, not the
    /// first layer, is what must land in the seat.
    pub fn paint_extent_centered(
        &self,
        frame: &mut Frame<'_>,
        seat: &Rect,
        frame_w: usize,
        frame_h: usize,
    ) {
        if self.layers.is_empty() {
            return;
        }
        let (extent_w, extent_h) = self.extent();
        let (origin_x, origin_y) = self.extent_origin();
        // Extent box centered in the seat, on the native canvas.
        self.paint_box_centered(
            frame,
            seat,
            frame_w,
            frame_h,
            (origin_x, origin_y),
            (
                u64::try_from(extent_w).unwrap_or(0),
                u64::try_from(extent_h).unwrap_or(0),
            ),
        );
    }

    /// Paint so the visible box centers in `seat`: the drawn-content
    /// box ([`Self::visible_origin`]..[`Self::extent`]) centers per
    /// axis on the native canvas, and every layer blits 1:1 at its
    /// anchor offset from the visible origin. Empty figures paint
    /// nothing. The Room panel and the waiting seats use this — the
    /// worn figure's layers carry `avatar.opa` canvas anchors with
    /// all-positive minima, so centering the whole authoring canvas
    /// shoved the figure past the slot's edges (ticket #171); the
    /// content box lands the head-to-feet stack inside its slot.
    /// Non-positive anchor minima keep the extent-centered box
    /// exactly, like [`Self::paint_extent_centered`].
    pub fn paint_visible_centered(
        &self,
        frame: &mut Frame<'_>,
        seat: &Rect,
        frame_w: usize,
        frame_h: usize,
    ) {
        if self.layers.is_empty() {
            return;
        }
        let (extent_w, extent_h) = self.extent();
        let (origin_x, origin_y) = self.visible_origin();
        // Visible box centered in the seat, on the native canvas:
        // the layers span `origin..extent`, so all-positive anchors
        // shrink the centered box to the drawn content.
        self.paint_box_centered(
            frame,
            seat,
            frame_w,
            frame_h,
            (origin_x, origin_y),
            (
                u64::try_from(extent_w)
                    .unwrap_or(0)
                    .saturating_sub(u64::try_from(origin_x.max(0)).unwrap_or(0)),
                u64::try_from(extent_h)
                    .unwrap_or(0)
                    .saturating_sub(u64::try_from(origin_y.max(0)).unwrap_or(0)),
            ),
        );
    }

    /// Blit every layer with the box centered in `seat`: each layer
    /// lands at its anchor offset from the box origin under its own
    /// source colorkey.
    fn paint_box_centered(
        &self,
        frame: &mut Frame<'_>,
        seat: &Rect,
        frame_w: usize,
        frame_h: usize,
        origin: (i32, i32),
        box_size: (u64, u64),
    ) {
        let center_native = |offset: u64, span: u32, start: i32| -> i32 {
            i32::try_from(i64::from(start) + (i64::from(span) - offset as i64) / 2).unwrap_or(0)
        };
        let left = center_native(box_size.0, seat.w, seat.x);
        let top = center_native(box_size.1, seat.h, seat.y);
        let (origin_x, origin_y) = origin;
        // Breathing: a worn figure stacks N parts x 12 frames. Paint
        // only the frame selected by the paint clock for each part;
        // static figures (not a multiple of 12 layers) paint all.
        let layers: Vec<&FigureLayer> = if self.layers.len() >= 12
            && self
                .layers
                .len()
                .is_multiple_of(AVATAR_BREATHING_FRAME_COUNT)
        {
            let frame = avatar_breathing_frame(avatar_breathing_now_ms());
            (0..self.layers.len() / AVATAR_BREATHING_FRAME_COUNT)
                .filter_map(|part| self.layers.get(part * AVATAR_BREATHING_FRAME_COUNT + frame))
                .collect()
        } else {
            self.layers.iter().collect()
        };
        for layer in layers {
            let (dx, dy) = anchor_in_frame(
                left.saturating_add(layer.dx.saturating_sub(origin_x)),
                top.saturating_add(layer.dy.saturating_sub(origin_y)),
                frame_w,
                frame_h,
            );
            blit_rgb555(frame, &layer.image.view(), dx, dy, Some(layer.colorkey));
        }
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

    /// Paint every layer with its top-left at `(x, y)` plus anchor,
    /// each under its own source colorkey, dropping pixels outside
    /// `clip` — the result row-slot clip (652×22 at the row
    /// origin); the frame clips every blit regardless.
    pub fn paint_clipped(&self, frame: &mut Frame<'_>, x: i32, y: i32, clip: &Rect) {
        for layer in &self.layers {
            blit_rgb555_clipped(
                frame,
                &layer.image.view(),
                x + layer.dx,
                y + layer.dy,
                Some(layer.colorkey),
                Some(clip),
            );
        }
    }

    /// Paint every layer with its top-left at the view origin `(x,
    /// y)` plus its record anchor scaled into the frame, each under
    /// its own source colorkey. The verified draw path for every
    /// placement: the exe blit adds the per-frame anchor to the view
    /// position (`docs/ui-draw.md` via `docs/stage-bga.md`); the exe
    /// canvas is native 800x600, so native anchors scale with the
    /// frame, keeping the art's exe proportions at any frame size. The
    /// view origin itself comes from [`placement_view_origin`].
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
/// applies to `.bnd` origins, so art keeps its exe proportions at
/// any frame size (a zero-size rect moves the origin without sizing
/// anything).
#[must_use]
pub fn anchor_in_frame(dx: i32, dy: i32, frame_w: usize, frame_h: usize) -> (i32, i32) {
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

/// Highest frame-per-image stride the body parser searches: the 7.55
/// combo atlases store ×4 (`Note_Combo4/5/6.ojt`, 196,736 B exact),
/// and the parser's exact blob accounting rejects every wrong
/// candidate, so the bound only caps wasted attempts.
const STRIDED_STRIDE_SEARCH_MAX: usize = 16;

/// Parse one archive body as a strided sprite-55 atlas: search the
/// frames-per-image stride whose exact accounting matches the body,
/// `None` when none does. Plain bodies account at stride 1 and never
/// reach the search; unknown bodies (the `.oji` outlier classes) fail
/// every candidate.
#[must_use]
pub(crate) fn strided_sprite55(body: &[u8]) -> Option<Sprite55Strided<'_>> {
    (2..=STRIDED_STRIDE_SEARCH_MAX)
        .map(|stride| o2jam_file::parse_sprite55_strided(body, stride))
        .find_map(Result::ok)
}

/// Decode one archive body as a figure: the plain sprite-55 layout
/// first, then — when the accounting holds but no frame decodes raw —
/// the prefixed `.oji` row layout (codec gap ③), and the strided-atlas
/// layout for plain-miss bodies (the combo glow atlases; exact blob
/// accounting separates all three). `.oja` effect animations decode
/// through the frame-plane reader (codec gap ⑤). `None` when no
/// layout decodes — unknown bodies (the compressed `.oji` outlier
/// frames) keep their codec gap.
fn sprite55_figure(body: &[u8]) -> Option<AvatarFigure> {
    if let Ok(sprite) = o2jam_file::parse_sprite55(body) {
        let figure = AvatarFigure::from_sprite55(&sprite);
        if !figure.is_empty() {
            return Some(figure);
        }
        let prefixed = o2jam_file::parse_sprite55_prefixed(body).ok()?;
        let figure = AvatarFigure::from_sprite55_prefixed(&prefixed);
        return (!figure.is_empty()).then_some(figure);
    }
    if let Ok(animation) = o2jam_file::parse_oja55(body) {
        let figure = AvatarFigure::from_oja55(&animation);
        return (!figure.is_empty()).then_some(figure);
    }
    strided_sprite55(body).map(|atlas| AvatarFigure::from_sprite55_strided(&atlas))
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
    let figure = sprite55_figure(body)?;
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
///
/// Kept as the archive name the bound record carries; bodies load
/// through the staging declaration, never through this const alone.
pub const AVI_LOGO_SPRITE: &str = "O2mediaLogo.ojs";
/// Channel-select block (no `.bnd` bind file: bare `BOUND` line).
/// `STATE_CHANNEL` names zero sprites on 7.55, so the shared
/// [`anchor_placements`] path stages nothing for it: sprite-less entries
/// draw no art (recipe step 6) and the scene keeps its flat background
/// plus the live channel list.
pub const CHANNEL_BLOCK: &str = "STATE_CHANNEL";
/// Multiplayer-lobby block laid out by `State_Room.bnd`.
pub const ROOM_BLOCK: &str = "STATE_ROOM";
/// Room layout bind file (75 rects for 76 entries minus self).
pub const ROOM_BIND_FILE: &str = "State_Room.bnd";
/// Pre-game room block laid out by `State_Waiting.bnd`.
pub const WAITING_BLOCK: &str = "STATE_WAITING";
/// Pre-game room bind file (116 rects for 117 entries minus self).
pub const WAITING_BIND_FILE: &str = "State_Waiting.bnd";
/// Solo-mission block laid out by `State_Planet.bnd`.
pub const PLANET_BLOCK: &str = "STATE_PLANET";
/// Solo-mission bind file (17 rects, `docs/planet.md`).
pub const PLANET_BIND_FILE: &str = "State_Planet.bnd";
/// Rankings block (display-only, no state class).
pub const LANKING_BLOCK: &str = "STATE_LANKING";
/// Rankings bind file (73 rects, `docs/lanking.md`).
pub const LANKING_BIND_FILE: &str = "State_Lanking.bnd";
/// Item-shop block (gapless codes `0x01..0xBA`, 186 rects).
pub const SHOP_BLOCK: &str = "STATE_SHOP";
/// Item-shop bind file (186 rects, `docs/shop.md`).
pub const SHOP_BIND_FILE: &str = "State_ItemShop.bnd";
/// Song-shop block (shop/owned/cart row SETs).
pub const MUSICSHOP_BLOCK: &str = "STATE_MUSICSHOP";
/// Song-shop bind file (138 rects, `docs/musicshop.md`).
pub const MUSICSHOP_BIND_FILE: &str = "State_MShop.bnd";
/// Avatar-room block (bind file declared but absent — anchor paint).
pub const MYROOM_BLOCK: &str = "STATE_MYROOM";
/// Waiting seat-cover body: one `Wait_Cover_Avatar.ojs` per seat
/// (`IDC_IMAGE_SWAIT_COVERAVATAR1..8`, codes `0x1650003D..44`,
/// `.bnd` rects 60..67), anchored at `(0,0)` so each bound rect
/// positions its own seat.
pub const WAIT_COVER_SPRITE: &str = "Wait_Cover_Avatar.ojs";
/// Waiting team-button bodies in seat order (`IDC_BUTTON_SWAIT_TEAMA..H`,
/// codes `0x16201004..0B`, `.bnd` rects 3..10).
pub const WAIT_TEAM_SPRITES: [&str; 8] = [
    "Wait_TeamA.ojs",
    "Wait_TeamB.ojs",
    "Wait_TeamC.ojs",
    "Wait_TeamD.ojs",
    "Wait_TeamE.ojs",
    "Wait_TeamF.ojs",
    "Wait_TeamG.ojs",
    "Wait_TeamH.ojs",
];
/// Loading-gate block probed before the single-sprite fallback: no
/// 7.55 `ControlList` carries it, so the scene stages the genuine
/// `MusicLoading.ojs` body through the fallback.
pub const LOADING_BLOCK: &str = "STATE_LOADING";
/// Loading-gate art shown while the song table stages.
pub const LOADING_SPRITE: &str = "MusicLoading.ojs";
/// Loading-gate song illustration slot: the walked chart's OJN cover
/// section (`o2jam_file::ojn_cover_jpeg`, docs/file-formats/ojn.md),
/// staged under this synthetic name — never an archive body, named by
/// no `ControlList` block. The Wine loading capture pins it: the
/// full-bleed woman/beads illustration with the baked `3-KEY` badge
/// and Artist/Remix/Pattern credits is byte-composition-identical to
/// the decoded OJN cover (`extracted/cover757-view.jpg`, 800x600),
/// while `MusicLoading.ojs` is the planet fallback the exe mounts only
/// when the song load returns 0 (file `0xe47a0`, ev `68d920c2…`).
pub const LOADING_COVER_SPRITE: &str = "OJN_COVER";
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
            role: PlacementRole::Single,
            id: entry.id.clone(),
            code: entry.code,
            sprite: entry.sprite.clone(),
            rect: unplaced_rect(),
        }),
    }
}

/// How one placement reaches the frame: painted once at its own
/// record, or as the expanded clone of a `SET`-group template member.
///
/// The exe instantiates `SET` children per parent control and writes
/// back parent-offset rects (`docs/ui-loader.md` step 8: the loader
/// matches a group by `code & 0xFF000`, recurses for its children, and
/// helper `0x5487B0` persists the adjusted rects). Staging expands the
/// same way: a clone's `rect` already carries its parent's origin, and
/// its record anchors stay template-local, so the body paints at
/// `rect + anchor` instead of at the anchors alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PlacementRole {
    /// Painted once: anchored bodies at their record anchors,
    /// anchor-free bodies at their rect origin (verified draw chain).
    #[default]
    Single,
    /// Expanded `SET` clone under one parent instance.
    Clone,
}

/// One `ControlList` entry with its `.bnd` layout rect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePlacement {
    /// Control id (`IDC_...`); a clone carries its parent's id so the
    /// scene can tell which instance owns it.
    pub id: String,
    /// Control code (top byte names the state).
    pub code: u32,
    /// Linked sprite body, when the entry names one.
    pub sprite: Option<String>,
    /// Native-canvas layout rect: positions the draw origin, never
    /// clips (verified draw path).
    pub rect: Rect,
    /// Whether this placement paints once or as a `SET` clone.
    pub role: PlacementRole,
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
/// pass. The `Note_BG` siblings overlap the lanes on some layers, so
/// behind painters clip the lane columns the caller passes (see
/// `paint_behind_placements`), and `Note_BG.ojs` rides its
/// exe-selected frame only (`playing_member_frame` — image 0, the
/// left strip; the right strip never paints on any Playing family).
/// Every name resolves in `Playing1.opi`. Couple-mode variants ride
/// unreferenced by the block, so only these five stage here — never
/// invented art.
pub const PLAYING_BEHIND_SPRITES: [&str; 5] = [
    "Playing_BG.ojs",
    "Note_BG.ojs",
    "3Key_Note_BG.ojs",
    "P2P3KeyNote_BG.ojs",
    "P2P7KeyNote_BG.ojs",
];

/// The BACK1 control code (`IDC_IMAGE_SPLAY_BACK1`, `0x1B500071`):
/// the Playing setup re-stages this control onto a mode-family
/// full-screen backdrop at runtime (the synthetic staging records
/// built on the stack through the factory `0x48B2D0`,
/// `docs/playing-backdrop.md`), so the shell synthesizes its
/// placement with the same code.
pub const PLAYING_BACKDROP_CODE: u32 = 0x1B500071;

/// Runtime-built BACK1 backdrop name for the room mode and chart
/// slot: the exe setup sprintf's `Playing_NormalBG%d.ojs` (solo),
/// `Playing_P2PBG%d.ojs` (team), or `Playing_TutorialBG.ojs`
/// (tutorial) with `%d = chart_index % 3`, a zero remainder taking 3
/// (file `0x4dc79f`–`0x4dc7bb`; the caller's arg semantic stays
/// OPEN — the walk feeds its chart slot, `docs/playing-backdrop.md`).
/// The solo/team families number 1..=3 / 1..=2 in the archive; the
/// shell has no tutorial arm.
#[must_use]
pub fn playing_backdrop_sprite(team_play: bool, chart_slot: u8) -> String {
    let variant = match chart_slot % 3 {
        0 => 3,
        remainder => remainder,
    };
    if team_play {
        format!("Playing_P2PBG{variant}.ojs")
    } else {
        format!("Playing_NormalBG{variant}.ojs")
    }
}

/// True for a runtime-built BACK1 backdrop body: the solo/team
/// numbered families plus the tutorial body. These names appear in
/// no ControlList — the exe builds them at runtime — so membership
/// is by shape, like `is_playing_behind_sprite`.
#[must_use]
pub fn is_playing_backdrop_sprite(sprite: &str) -> bool {
    sprite == "Playing_TutorialBG.ojs"
        || is_numbered_family(sprite, "Playing_NormalBG")
        || is_numbered_family(sprite, "Playing_P2PBG")
}

/// True for a `<family><digits>.ojs` body name: the runtime-built
/// BACK1 backdrops number their family members (`Playing_NormalBG1..3`,
/// `Playing_P2PBG1..2`), and no other archive body shares the shape.
fn is_numbered_family(sprite: &str, family: &str) -> bool {
    sprite.starts_with(family)
        && sprite.ends_with(".ojs")
        && sprite[family.len()..sprite.len() - 4]
            .bytes()
            .all(|byte| byte.is_ascii_digit())
}

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

/// Room mode the Playing art families select through: the Playing
/// applier (`0x4e7850`'s case table) loads only the family member the
/// joined room names, and the reference photo pins the solo 7-key set
/// (`docs/stage-bga.md`, `docs/playing-reference.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayingArtMode {
    /// Wire key mode (`0` 3-key, `1` 5-key, `2` 7-key;
    /// `o2jam_net::KeyMode`). Non-3-key rooms take the 7-key family
    /// member — the wider art — where the family splits by key mode.
    pub key_mode: u8,
    /// Team/versus room (`Live`); solo rooms stage the left/single
    /// member of every mirrored pair.
    pub team_play: bool,
}

/// True while a Playing sprite names the family member the room mode
/// selects; unselected members leave empty slots (the exe slot loader
/// loads nothing on a miss and an empty slot paints nothing).
/// Families (`ControlList_Playing.txt` entries + archive anchors;
/// solo set verified against the reference photo):
/// - note backgrounds: solo `Note_BG.ojs` (3-key rooms take
///   `3Key_Note_BG.ojs`), team `P2P{3,7}KeyNote_BG.ojs`; `Note_BG.ojs`
///   loads its exe-selected frame only (`playing_member_frame` —
///   image 0, the left strip; image 1 never paints on any family);
/// - life gauges: `Life_gauge_left_bar.ojs` always, the
///   `Life_gauge_back.ojs` shell team-only (the photo hides it);
/// - BP buffer bars: `BpPoint_Left_bar.ojs` solo (the photo's
///   far-left edge gauge), `BpPoint_Right_bar.ojs` team-only.
///
/// Every other sprite stages regardless.
#[must_use]
pub fn playing_member_selected(sprite: &str, mode: &PlayingArtMode) -> bool {
    let three_key = o2jam_net::KeyMode::from_u8(mode.key_mode)
        .is_ok_and(|key| key == o2jam_net::KeyMode::ThreeKeys);
    match sprite {
        "Note_BG.ojs" => !mode.team_play && !three_key,
        "3Key_Note_BG.ojs" => !mode.team_play && three_key,
        "P2P3KeyNote_BG.ojs" => mode.team_play && three_key,
        "P2P7KeyNote_BG.ojs" => mode.team_play && !three_key,
        "BpPoint_Left_bar.ojs" | "Life_gauge_left_bar.ojs" => !mode.team_play,
        "BpPoint_Right_bar.ojs" | "Life_gauge_back.ojs" => mode.team_play,
        _ => true,
    }
}

/// Frame the exe view paints of a note-background or BACK1 backdrop
/// body: `Some(0)` for `Note_BG.ojs`, `Some(1)` for the
/// runtime-built BACK1 backdrop, `None` for every other sprite.
///
/// `IDC_IMAGE_SPLAY_BACK2` (code `0x1B500072`, `Note_BG.ojs`) stores
/// two 213x484 raw strips — image 0 at anchor (20,0), image 1 at
/// (567,0), byte-identical pixels (Playing1.opi body walk, 412,416 B,
/// records at body +8/+28; `docs/notebg-cover.md`). The exe image
/// control paints one frame per view (`docs/ui-draw.md` steps 3–6;
/// setter `0x2f870` swaps images, file `0x2f891`–`0x2f8f7`), and a
/// frame change needs an exe writer for that control code — a
/// whole-image dword sweep finds none (`0x1B500072`: zero hits; the
/// only BACK-family literals are BACK1 `0x1B500071` at VA
/// `0x4dc822`/`0x4dcb8c`/`0x4dd2ec` and `0x1B5000D2` at
/// `0x4e7525`/`0x4e7577`). BACK2 therefore keeps its loader-default
/// image 0 — the left strip — on every Playing mode family; image 1
/// is authored but unselected, and the reference photo pins venue
/// deck over its band (`docs/playing-reference.md`).
///
/// The BACK1 backdrop bodies store two full-screen 800x600 frames;
/// the solo `Playing_NormalBG1..3.ojs` frames differ only inside
/// x 21..=230, y 0..=530 — the lane-panel band the staged
/// `Note_BG.ojs` strip covers — so the cut is invisible in the
/// settled scene; the P2P pair differs across the full panel
/// (x 21..=778). The exe setup drives the frame by mode arm
/// (`SetFrame 0x42f870`, frame 2 on the mode-0 arm at file
/// `0x4dc7f2` → image 1, frame 1 on the mode-2 arm at file
/// `0x4dc7dc` → image 0; which arm maps to which room mode stays
/// OPEN — `docs/playing-backdrop.md`), and the shell rides the
/// mode-0 arm's image 1. The unselected note-background family
/// members (`3Key_Note_BG.ojs`, the `P2P*Note_BG.ojs` bodies) are
/// single-frame full-screen bodies, so their cut is a no-op
/// (`docs/notebg-cover.md`).
#[must_use]
pub fn playing_member_frame(sprite: &str) -> Option<usize> {
    if sprite == "Note_BG.ojs" {
        return Some(0);
    }
    if is_playing_backdrop_sprite(sprite) {
        return Some(1);
    }
    None
}

/// [`load_scene_figures_for_stage`] through the room mode: family
/// members the room does not select (see [`playing_member_selected`])
/// never load, leaving their slots empty, and the selected
/// `Note_BG.ojs` member loads its exe-selected frame only (see
/// [`playing_member_frame`] — the view paints image 0, the left
/// strip; the right strip never covers the venue).
#[must_use]
pub fn load_scene_figures_for_room(
    archive: &[u8],
    names: &[String],
    arena_stage: Option<u16>,
    mode: &PlayingArtMode,
) -> HashMap<String, AvatarFigure> {
    let selected: Vec<String> = names
        .iter()
        .filter(|name| playing_member_selected(name, mode))
        .cloned()
        .collect();
    let mut figures = load_scene_figures_for_stage(archive, &selected, arena_stage);
    for (name, figure) in figures.iter_mut() {
        if let Some(last) = playing_member_frame(name) {
            *figure = figure.layers_through(last);
        }
    }
    figures
}

/// Anchor placement for one static backdrop entry: no `.bnd` rect
/// covers entries 0–212, so the sprite paints 1:1 at its own
/// record anchor (all-zero rect, never full-bleed).
fn place_backdrop_panel(entry: &ControlEntry, sprite: &str) -> ScenePlacement {
    ScenePlacement {
        role: PlacementRole::Single,
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

/// The member life-gauge body the eight peer-slot entries
/// reference: `Avatar_Info_LifeGauge.ojs` (46x8, `Playing1.opi`
/// slot 6). Its record anchor is the frame corner (0,0), clear of
/// the note lanes (they open at row 28).
pub const PLAYING_PEER_GAUGE_SPRITE: &str = "Avatar_Info_LifeGauge.ojs";

/// Anchor placement for the corner member life gauge (bnd-audit 16,
/// ticket #102): the first `Avatar_Info_LifeGauge.ojs` entry
/// (`IDC_IMAGE_SPLAY_LIFEGAUGE1`, entry 41 — before every behind and
/// backdrop entry) rides its own record anchor through the unplaced
/// rect, never an appended full-canvas placement over the 17-rect
/// bound window (ADR-0003: the painter consumes the staged rect,
/// never a canvas stretch). The sibling `LIFEGAUGE2..8` entries
/// reference the same body under runtime stacking, which the peer
/// bars keep as their RUNTIME model (geometry-audit
/// `NATIVE_PEER_BAR`), so one placement stages the body's static
/// anchor.
fn bind_playing_corner_gauge(entries: &[&ControlEntry]) -> Vec<ScenePlacement> {
    entries
        .iter()
        .copied()
        .take(PLAYING_SUBSET_FIRST_ENTRY)
        .filter(|entry| entry_takes_rect(entry))
        .find(|entry| entry.sprite.as_deref() == Some(PLAYING_PEER_GAUGE_SPRITE))
        .map(|entry| place_backdrop_panel(entry, PLAYING_PEER_GAUGE_SPRITE))
        .into_iter()
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
/// 1–17); the corner member gauge (entry 41) and the static backdrop
/// panels among entries 0–212 stage as anchor placements ahead of
/// them, and the behind-highway scene stages first of the file-order
/// art. Rects no entry claims are ignored (the
/// Playing tail past `VOLUME` rides unloaded).
#[must_use]
pub fn bind_scene_rects(block: &ControlBlock, bnd: &Bnd) -> Vec<ScenePlacement> {
    if is_stub_block(block) {
        return Vec::new();
    }
    if block.name == "STATE_PLAYING" {
        return bind_playing_subset(block, bnd);
    }
    let mut placed = bind_entries_by_ordinal(block, bnd);
    expand_set_clones(block, &mut placed);
    placed
}

/// `SET`-group match key of an entry: the exe's clone rule reads
/// `code & 0xFF000` against the group's `+0x04` word (`docs/ui-loader.md`
/// step 8), which is the second number on the `SET` line.
#[must_use]
pub fn set_group_key(code: u32) -> u32 {
    code & 0xFF_000
}

/// The `SET` group code a block line declares, if it declares one
/// (`SET 0x11 0x00012000` → `Some(0x12000)`; a parameter-less `SET`
/// run declares none).
#[must_use]
fn declared_set_code(params: &[u32]) -> Option<u32> {
    params.get(1).copied().filter(|code| *code != 0)
}

/// Expand every `SET` group under its parent instances (exe loader
/// step 8 + write-back helper `0x5487B0`): a group's members are
/// template-local — their `.bnd` rects sit inside one parent's box —
/// and each parent entry whose code carries the group key clones them
/// with the parent's origin added to the rect. Members that no parent
/// hosts stay single (a group without an instance paints nowhere in
/// the exe either, but dropping art on a rule miss is worse than
/// keeping it visible).
///
/// Verified against 7.55: `STATE_ROOM` parents `EXBUTTON1..6`
/// (`0x1301201B..20`, group `0x12000`) and `SET 0x11 0x00012000`
/// members `Room_Om.ojs (56,62)` / `Room_State.ojs (203,63)` /
/// `Room_Number.ojs (16,14)` land at panel `(26,82)` + rect in the
/// Wine capture `/home/arnim/scratch/ac-room.png` (correlation best
/// match `+0,+0` each); `STATE_WAITING` parents `EXTEND1..8`
/// (`0x16015022..29`, group `0x15000`) put `Wait_BossMark.ojs` at
/// `(37,55)+(1,1) = (38,56)` and `Wait_PCRoomMark.ojs` at
/// `(37,55)+(94,1) = (131,56)`, both within 1 px of
/// `/home/arnim/scratch/ad-wait.png`.
fn expand_set_clones(block: &ControlBlock, placements: &mut Vec<ScenePlacement>) {
    for set in &block.sets {
        let Some(code) = declared_set_code(&set.params) else {
            continue;
        };
        let member_codes: Vec<u32> = set
            .entries
            .iter()
            .filter(|entry| entry_takes_rect(entry) && entry.sprite.is_some())
            .map(|entry| entry.code)
            .collect();
        let parents: Vec<(String, Rect)> = placements
            .iter()
            .filter(|placed| set_group_key(placed.code) == code)
            .map(|placed| (placed.id.clone(), placed.rect))
            .collect();
        let members: Vec<ScenePlacement> = placements
            .iter()
            .filter(|placed| member_codes.contains(&placed.code))
            .cloned()
            .collect();
        if parents.is_empty() || members.is_empty() {
            continue;
        }
        placements.retain(|placed| !member_codes.contains(&placed.code));
        let mut clones = Vec::with_capacity(parents.len() * members.len());
        for (parent_id, parent_rect) in &parents {
            for member in &members {
                clones.push(ScenePlacement {
                    role: PlacementRole::Clone,
                    id: parent_id.clone(),
                    code: member.code,
                    sprite: member.sprite.clone(),
                    rect: Rect {
                        x: parent_rect.x.saturating_add(member.rect.x),
                        y: parent_rect.y.saturating_add(member.rect.y),
                        w: member.rect.w,
                        h: member.rect.h,
                    },
                });
            }
        }
        placements.extend(clones);
    }
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
/// anchor-placed, clipped clear of the lane columns at paint), then
/// the static backdrop panels, then the bound run (entries 213–229)
/// paired with the 17 rects by code ordinal (`EXIT` = ordinal 1 →
/// rect 0; the run is capped at the rect count, so the tail past
/// `VOLUME` rides unloaded). Backdrops paint first so the bound
/// gauges and buttons keep their top slot. Container-coded entries
/// stay out in every run; a window ordinal missing the table stages
/// the all-zero rect (anchor draw), like the bound path.
fn bind_playing_subset(block: &ControlBlock, bnd: &Bnd) -> Vec<ScenePlacement> {
    let entries = block.all_entries();
    let mut placed = bind_playing_corner_gauge(&entries);
    placed.extend(bind_playing_behind(&entries));
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
        role: PlacementRole::Single,
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

/// Decode the walked chart's OJN cover section as the loading-gate
/// song illustration, staged under [`LOADING_COVER_SPRITE`]. The JPEG
/// stream runs from the last level slice end to end of file with no
/// EOI marker on every shipped 7.55 chart (14/14 decode 800x600 per
/// `o2jam_file`'s `real_ojn_covers_extract_and_decode`); extraction
/// plus decode plus the RGB888→RGB555 fold all flow through the
/// verified `o2jam_file` readers. One opaque layer at anchor `(0,0)`:
/// the loading painter stretches it full-frame with no colour key
/// (the JPEG carries none — every pixel is art). `None` on any miss
/// — non-chart bytes, unknown magic, undecodable stream — and the
/// loading gate keeps its planet fallback then; nothing is invented.
#[must_use]
pub fn ojn_cover_figure(ojn_bytes: &[u8]) -> Option<AvatarFigure> {
    let stream = o2jam_file::ojn_cover_jpeg(ojn_bytes).ok()?;
    let decoded = o2jam_file::decode_ojn_cover(&stream).ok()?;
    let image = rgb888_to_rgb555(&decoded)?;
    let mut figure = AvatarFigure::default();
    figure.push_layer(FigureLayer {
        image,
        dx: 0,
        dy: 0,
        // Ignored by the opaque loading-cover paint; kept only so
        // the layer stays a well-formed figure member.
        colorkey: 0x0000,
    });
    Some(figure)
}

/// Fold one decoded OJN cover to RGB555 words (5-bit channels; the
/// frame blit composes in RGB555 words). `None` when the pixel
/// buffer does not tile the claimed dimensions.
#[must_use]
fn rgb888_to_rgb555(decoded: &o2jam_file::JpegRgbImage) -> Option<Rgb555Image> {
    let pixels = decoded.width.checked_mul(decoded.height)?.checked_mul(3)?;
    if decoded.pixels.len() != pixels {
        return None;
    }
    let words = decoded
        .pixels
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| {
            let [r, g, b] = *pixel;
            (u16::from(r) >> 3) << 10 | (u16::from(g) >> 3) << 5 | u16::from(b) >> 3
        })
        .collect();
    Some(Rgb555Image {
        w: decoded.width,
        h: decoded.height,
        words,
    })
}

/// Staged view position of one placement (`docs/ui-draw.md`): the
/// exe's image boxes stage view `(0, 0)` and the blit adds the
/// per-frame record anchor, so an anchor offset is the body's
/// authored absolute position — the reference photo pins the
/// disagreeing pair: `BpPoint_Left_bar.ojs` anchor `(2,266)` shows at
/// the far-left edge while its bound rect is `(200,3)`. A zero anchor
/// contributes no position, though, and then the bound rect is the
/// only authored placement data: its scaled origin stages the view
/// (the view-from-rect question `docs/ui-draw.md` leaves open — the
/// eight `Wait_Cover_Avatar.ojs` covers carry anchor `(0,0)` while
/// their eight `State_Waiting.bnd` rects name the seats, and the
/// `Room_*.ojs` lobby icons anchor at `(0,0)` inside their row
/// template). Unbound rects scale to the frame origin, so unbound art
/// still draws at its anchors alone.
fn placement_view_origin(
    figure: &AvatarFigure,
    placement: &ScenePlacement,
    frame: &Frame<'_>,
    shares_body: bool,
) -> (i32, i32) {
    // A `SET` clone's record anchors stay template-local, so its rect
    // positions the body even when the layers carry anchors (exe
    // loader writes back parent-offset rects, `docs/ui-loader.md`
    // step 8). Single placements keep the verified rule: anchored
    // bodies author their own absolute position and ignore the rect.
    if placement.role == PlacementRole::Clone || shares_body || !figure.has_anchor_offset() {
        let scaled = scale_rect_to_frame(&placement.rect, frame.w, frame.h);
        return (scaled.x, scaled.y);
    }
    (0, 0)
}

/// Paint every placed figure in file order (verified step 8: the
/// background lands first and later entries cover it). Each layer
/// blits 1:1 at its record anchor scaled into the frame, over the
/// staged view position of its placement
/// ([`placement_view_origin`]): anchored bodies draw at their
/// anchors, anchor-free bodies at their bound rect origins. Most
/// bound bodies duplicate their rect origins byte-exact
/// (`Normal_TimeBar.ojs` (489,556) == rect[2]), so anchored art lands
/// where its rect bounds it anyway. Placements without a loaded
/// figure are skipped, and live UI ink paints on top in the callers.
pub fn paint_scene_figures(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
) {
    paint_scene_figures_live(frame, placements, figures, |_| true);
}

/// [`paint_scene_figures`] behind a per-placement filter: the scene
/// decides which `SET` clones are live (populated rows and seats
/// paint, empty slots keep their bare plate). Single placements pass
/// any filter that accepts [`PlacementRole::Clone`] filtering only
/// when the caller opts in — see [`clone_instance_live`].
pub fn paint_scene_figures_live(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
    keep: impl Fn(&ScenePlacement) -> bool,
) {
    let shared = shared_sprite_names(placements);
    for placement in placements.iter().filter(|placement| keep(placement)) {
        let shares_body = placement
            .sprite
            .as_deref()
            .is_some_and(|sprite| shared.contains(sprite));
        paint_placed_figure(frame, placement, figures, shares_body);
    }
}

/// Sprite bodies referenced by more than one placement of a scene.
///
/// One record's anchors cannot be the absolute screen position for two
/// instances, so a shared body's anchors stay frame-internal and its
/// placements' rects position it. Verified on 7.55: `Wait_RoomNumber.ojs`
/// serves five top-bar digit wells (`CHNUM_10/01`, `RMNUM_100/010/001`,
/// rects `(100,19)…(193,19)`) with one anchor `(3,0)` — the exe paints
/// the digits in the wells, not at the frame corner — while single-use
/// bodies like `Playing_Exit.ojs` (anchor `(751,554)` == its rect)
/// author their own position.
#[must_use]
pub fn shared_sprite_names(placements: &[ScenePlacement]) -> std::collections::HashSet<&str> {
    let mut seen = std::collections::HashSet::new();
    let mut shared = std::collections::HashSet::new();
    for placement in placements {
        let Some(sprite) = placement.sprite.as_deref() else {
            continue;
        };
        if !seen.insert(sprite) {
            shared.insert(sprite);
        }
    }
    shared
}

/// Liveness of one staged placement for the `SET` clone model: a
/// clone paints only while its owner sits among the first `live`
/// entries of `instances` (the exe clones a row template per
/// populated row and leaves the rest as bare plates). Owners the
/// scene does not list are chrome (scroll tracks, chat lines) and
/// always paint; single placements always paint.
#[must_use]
pub fn clone_instance_live(placed: &ScenePlacement, instances: &[&str], live: usize) -> bool {
    if placed.role != PlacementRole::Clone {
        return true;
    }
    match instances.iter().position(|id| *id == placed.id) {
        Some(index) => index < live,
        None => true,
    }
}

/// The placement's loaded figure: every paint entry opens with
/// this guard (a missing sprite or figure paints nothing).
fn resolve_figure<'a>(
    placement: &ScenePlacement,
    figures: &'a HashMap<String, AvatarFigure>,
) -> Option<&'a AvatarFigure> {
    figures.get(placement.sprite.as_ref()?)
}

/// The placement's loaded figure and its layer `index`: the full
/// sprite -> figure -> layer guard chain as one question, asked by
/// the single-layer paint entries (a missing sprite, figure, or
/// layer paints nothing).
fn resolve_layer<'a>(
    placement: &ScenePlacement,
    figures: &'a HashMap<String, AvatarFigure>,
    index: usize,
) -> Option<(&'a AvatarFigure, &'a FigureLayer)> {
    let figure = resolve_figure(placement, figures)?;
    let layer = figure.layer(index)?;
    Some((figure, layer))
}

/// Paint one layer of a placed figure at its staged view position
/// (the same origin rule as [`paint_placed_figure`], clones included):
/// the hover pass repaints a single frame over the stacked rest paint,
/// so the frame index — not the whole stack — answers the pointer.
/// `clip` optionally narrows the repaint to a native rect (the planet
/// emblem plate reveal), scaled into the frame like every origin.
/// A missing sprite, figure, or layer paints nothing.
pub fn paint_placement_layer(
    frame: &mut Frame<'_>,
    placement: &ScenePlacement,
    figures: &HashMap<String, AvatarFigure>,
    layer_index: usize,
    shares_body: bool,
    clip: Option<Rect>,
) {
    let Some((figure, layer)) = resolve_layer(placement, figures, layer_index) else {
        return;
    };
    let (frame_w, frame_h) = (frame.w, frame.h);
    let clip = clip.map(|rect| scale_rect_to_frame(&rect, frame_w, frame_h));
    if placement.role == PlacementRole::Clone {
        let dest = scale_rect_to_frame(&placement.rect, frame_w, frame_h);
        blit_rgb555_clipped(
            frame,
            &layer.image.view(),
            dest.x + layer.dx,
            dest.y + layer.dy,
            Some(layer.colorkey),
            clip.as_ref(),
        );
        return;
    }
    let (view_x, view_y) = placement_view_origin(figure, placement, frame, shares_body);
    let (dx, dy) = anchor_in_frame(layer.dx, layer.dy, frame_w, frame_h);
    blit_rgb555_clipped(
        frame,
        &layer.image.view(),
        view_x + dx,
        view_y + dy,
        Some(layer.colorkey),
        clip.as_ref(),
    );
}

/// Paint one placed figure, if its sprite loaded: every layer blits
/// 1:1 at its record anchor scaled into the frame, over the staged
/// view position (see [`paint_scene_figures`] for the draw chain),
/// unclipped.
fn paint_placed_figure(
    frame: &mut Frame<'_>,
    placement: &ScenePlacement,
    figures: &HashMap<String, AvatarFigure>,
    shares_body: bool,
) {
    let Some(figure) = resolve_figure(placement, figures) else {
        return;
    };
    // A clone is one instance of a template control: an image control
    // paints one frame, so a clone contributes its frame-0 layer and the
    // scene's live-frame pass selects any other frame on top. Single
    // placements keep the verified all-layers stack.
    if placement.role == PlacementRole::Clone {
        let (view_x, view_y) = placement_view_origin(figure, placement, frame, shares_body);
        if let Some(layer) = figure.layer(0) {
            let (dx, dy) = anchor_in_frame(layer.dx, layer.dy, frame.w, frame.h);
            blit_rgb555(
                frame,
                &layer.image.view(),
                view_x + dx,
                view_y + dy,
                Some(layer.colorkey),
            );
        }
        return;
    }
    let (view_x, view_y) = placement_view_origin(figure, placement, frame, shares_body);
    figure.paint_scaled_anchors(frame, view_x, view_y, frame.w, frame.h);
}

/// True for a behind-highway placement: its sprite names the venue
/// or a note background, quoted or resolved (see
/// [`is_playing_behind_sprite`)).
pub(crate) fn placement_is_behind(placement: &ScenePlacement) -> bool {
    placement.sprite.as_deref().is_some_and(|sprite| {
        is_playing_behind_sprite(sprite) || is_playing_backdrop_sprite(sprite)
    })
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
    let shared = shared_sprite_names(placements);
    for placement in placements
        .iter()
        .filter(|placed| placement_is_behind(placed))
    {
        paint_behind_placement(frame, placement, figures, exclusions, &shared);
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
    let shared = shared_sprite_names(placements);
    for placement in placements
        .iter()
        .filter(|placed| !placement_is_behind(placed))
    {
        let shares_body = placement
            .sprite
            .as_deref()
            .is_some_and(|sprite| shared.contains(sprite));
        paint_placed_figure(frame, placement, figures, shares_body);
    }
}

/// Paint one behind-highway figure, if its sprite loaded: each layer
/// blits 1:1 at its record anchor (scaled into the frame — the same
/// verified draw chain as every painter, over the staged view
/// position), clipped to the fragments outside `exclusions`.
fn paint_behind_placement(
    frame: &mut Frame<'_>,
    placement: &ScenePlacement,
    figures: &HashMap<String, AvatarFigure>,
    exclusions: &[Rect],
    shared: &std::collections::HashSet<&str>,
) {
    let Some(figure) = resolve_figure(placement, figures) else {
        return;
    };
    let shares_body = shared.contains(placement.sprite.as_deref().expect("sprite resolved"));
    let (view_x, view_y) = placement_view_origin(figure, placement, frame, shares_body);
    paint_behind_layers(frame, figure, view_x, view_y, exclusions);
}

/// Paint every layer of one figure through its visible fragments:
/// the layer box (view origin plus frame-scaled anchor) minus the
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
    fn ojn_cover_rejects_non_chart_bytes() {
        assert!(ojn_cover_figure(b"garbage").is_none());
        assert!(ojn_cover_figure(&[]).is_none());
        assert!(ojn_cover_figure(b"OJM\0payload").is_none());
    }

    #[test]
    fn ojn_cover_stage_names_the_song_illustration() {
        // Install-guarded: the 7.55 `Music` folder carries the song
        // whose cover the loading capture shows (song 757). Skips
        // when `install-root` is absent.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let figure =
            ojn_cover_figure(&bytes).unwrap_or_else(|| panic!("{}: cover decodes", path.display()));
        assert_eq!(figure.len(), 1);
        let layer = figure.layer(0).unwrap();
        assert_eq!((layer.image.w, layer.image.h), (800, 600));
        assert_eq!((layer.dx, layer.dy), (0, 0));
    }

    /// Strided sprite-55 atlas body: one 20-byte record per entry,
    /// record `i` chained at data offset `i * w * h * 2`, and a blob of
    /// `records * stride` consecutive `w * h` RGB555 frames (the
    /// `Note_Combo4/5/6.ojt` layout; `parse_sprite55_strided` accepts
    /// only the stride that accounts exactly).
    fn strided55_body(
        records: &[(i16, i16, u16, u16)],
        stride: usize,
        frame_words: &[u16],
    ) -> Vec<u8> {
        assert_eq!(records.len() * stride, frame_words.len());
        let frame_bytes = frame_words.len() * 2 / (records.len() * stride);
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&(records.len() as u16).to_le_bytes());
        blob.extend_from_slice(&0x0000u16.to_le_bytes());
        for (index, (x, y, w, h)) in records.iter().enumerate() {
            blob.extend_from_slice(&x.to_le_bytes());
            blob.extend_from_slice(&y.to_le_bytes());
            blob.extend_from_slice(&w.to_le_bytes());
            blob.extend_from_slice(&h.to_le_bytes());
            blob.extend_from_slice(&((index * frame_bytes) as u32).to_le_bytes());
            blob.extend_from_slice(&(frame_bytes as u32).to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
        }
        for word in frame_words {
            blob.extend_from_slice(&word.to_le_bytes());
        }
        blob
    }

    /// Codec gap ①: the stage-resolved combo glow body
    /// (`_Note_Combo.ojt` + room stage 4 → the strided
    /// `Note_Combo4.ojt` atlas) routes through the archive loader
    /// instead of dropping (the plain layout accounting rejects it).
    #[test]
    fn combo_runtime_ref_routes_the_stage_atlas() {
        let frames: Vec<u16> = (0..8u16).map(|frame| 0x7C00 + frame).collect();
        let body = strided55_body(&[(0, 0, 1, 1); 2], 4, &frames);
        let archive = opa_blob(&[("Note_Combo4.ojt", &body)]);
        let figure = load_archive_sprite_for_stage(&archive, "_Note_Combo.ojt", Some(4)).unwrap();
        assert_eq!(figure.len(), 8);
    }

    /// Every strided layer carries the parser's own frame pixels
    /// (`frame_data`, record-major), its record anchor, and the atlas
    /// colour key — no invented pixels.
    #[test]
    fn strided_figure_decodes_parser_frames_at_record_anchors() {
        let frames: Vec<u16> = (0..8u16).map(|frame| 0x7C00 + frame).collect();
        let body = strided55_body(&[(0, 0, 1, 1), (3, 4, 1, 1)], 4, &frames);
        let atlas = strided_sprite55(&body).unwrap();
        assert_eq!(atlas.frames_per_image, 4);
        let figure = AvatarFigure::from_sprite55_strided(&atlas);
        assert_eq!(figure.len(), 8);
        for index in 0..8 {
            let (record, frame) = (index / 4, index % 4);
            let layer = figure.layer(index).unwrap();
            assert_eq!(layer.image.words, vec![0x7C00 + index as u16]);
            assert_eq!(layer.image.w, 1);
            assert_eq!(layer.image.h, 1);
            let (dx, dy) = if record == 0 { (0, 0) } else { (3, 4) };
            assert_eq!((layer.dx, layer.dy), (dx, dy));
            assert_eq!(layer.colorkey, 0x0000);
            assert_eq!(atlas.frame_data(&atlas.images[record], frame).len(), 2);
        }
    }

    /// The stride search accepts only the accounting-exact stride, so
    /// plain bodies and unknown bodies never route as strided.
    #[test]
    fn strided_search_accounts_exactly_or_not_at_all() {
        let plain = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        assert!(strided_sprite55(&plain).is_none());
        assert!(strided_sprite55(b"not a sprite").is_none());
        let frames: Vec<u16> = (0..8u16).map(|frame| 0x03E0 + frame).collect();
        let body = strided55_body(&[(0, 0, 1, 1); 2], 4, &frames);
        assert_eq!(strided_sprite55(&body).unwrap().frames_per_image, 4);
        // One byte short of the strided accounting: both layouts miss.
        let truncated: &[u8] = &body[..body.len() - 2];
        assert!(strided_sprite55(truncated).is_none());
        let archive = opa_blob(&[("Truncated.ojt", truncated)]);
        assert!(load_archive_sprite(&archive, "Truncated.ojt").is_none());
    }

    /// Prefixed sprite-55 body (the `.oji` row-group layout):
    /// header + one record per image, each image storing row groups
    /// `[u16 width][u16 x][u16 y]` + pixels plus a zero terminator
    /// word (the `SelectAlbum_Bar.oji` shape).
    fn prefixed_row_group(width: u16, x: u16, y: u16, words: &[u16]) -> Vec<u8> {
        assert_eq!(words.len(), width as usize);
        let mut group = Vec::new();
        group.extend_from_slice(&width.to_le_bytes());
        group.extend_from_slice(&x.to_le_bytes());
        group.extend_from_slice(&y.to_le_bytes());
        for word in words {
            group.extend_from_slice(&word.to_le_bytes());
        }
        group
    }

    fn prefixed_frame(row0: &[u16], row1: &[u16]) -> Vec<u8> {
        let mut data = prefixed_row_group(4, 0, 0, row0);
        data.extend_from_slice(&prefixed_row_group(2, 1, 1, row1));
        data.extend_from_slice(&0u16.to_le_bytes());
        data
    }

    fn prefixed55_body(images: &[(u16, u16, u16, u16, Vec<u8>)]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&(images.len() as u16).to_le_bytes());
        blob.extend_from_slice(&0x0000u16.to_le_bytes());
        let mut run = 0u32;
        for (x, y, w, h, data) in images {
            blob.extend_from_slice(&x.to_le_bytes());
            blob.extend_from_slice(&y.to_le_bytes());
            blob.extend_from_slice(&w.to_le_bytes());
            blob.extend_from_slice(&h.to_le_bytes());
            blob.extend_from_slice(&run.to_le_bytes());
            blob.extend_from_slice(&(data.len() as u32).to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
            run += data.len() as u32;
        }
        for (_, _, _, _, data) in images {
            blob.extend_from_slice(data);
        }
        blob
    }

    /// Codec gap ③: a prefixed `.oji` body routes through the archive
    /// loader into composed layers — the plain layout decodes none of
    /// its frames, and before the gap it dropped the body whole.
    /// Each layer pastes the row spans onto its `w×h` canvas; the
    /// trimmed-row gaps stay colour-key (never invented pixels).
    #[test]
    fn prefixed_oji_body_routes_to_composed_layers() {
        let body = prefixed55_body(&[
            (
                0,
                0,
                4,
                2,
                prefixed_frame(&[0x7C00, 0x7C01, 0x7C02, 0x7C03], &[0x7C04, 0x7C05]),
            ),
            (
                1,
                0,
                4,
                2,
                prefixed_frame(&[0x03E0, 0x03E1, 0x03E2, 0x03E3], &[0x03E4, 0x03E5]),
            ),
        ]);
        let archive = opa_blob(&[("SelectAlbum_Bar.oji", &body)]);
        let figure = load_archive_sprite(&archive, "SelectAlbum_Bar.oji").unwrap();
        assert_eq!(figure.len(), 2, "one composed layer per stored frame");
        let layer = figure.layer(0).unwrap();
        assert_eq!((layer.dx, layer.dy), (0, 0));
        assert_eq!(layer.colorkey, 0x0000);
        assert_eq!(
            layer.image.words,
            vec![0x7C00, 0x7C01, 0x7C02, 0x7C03, 0x0000, 0x7C04, 0x7C05, 0x0000]
        );
        let layer = figure.layer(1).unwrap();
        assert_eq!((layer.dx, layer.dy), (1, 0));
        assert_eq!(layer.image.words[0], 0x03E0);
    }

    /// Codec gap ⑤: a `.oja` effect animation routes through the
    /// archive loader into one layer per stored frame — each layer
    /// pastes its row-group pixel plane onto the colour-key-cleared
    /// canvas at the record anchor (stored word 0 is the transparent
    /// colour; `.oja` headers carry no colour key).
    #[test]
    fn oja_animation_routes_to_frame_layers() {
        // Frame 0: full 4x1 row of two words at anchor (0, 0); its
        // plane is one row group plus the zero terminator (16 B),
        // alpha one byte per covered pixel. Frame 1: a trimmed row at
        // anchor (2, 1). Frame 1's record `a` chains a0 + b0 + c0.
        let mut plane0 = 4u16.to_le_bytes().to_vec();
        plane0.extend_from_slice(&0u16.to_le_bytes());
        plane0.extend_from_slice(&0u16.to_le_bytes());
        plane0.extend_from_slice(&0x7C00u16.to_le_bytes());
        plane0.extend_from_slice(&0x03E0u16.to_le_bytes());
        plane0.extend_from_slice(&0x7C02u16.to_le_bytes());
        plane0.extend_from_slice(&0x7C03u16.to_le_bytes());
        plane0.extend_from_slice(&0u16.to_le_bytes());
        let mut plane1 = 2u16.to_le_bytes().to_vec();
        plane1.extend_from_slice(&1u16.to_le_bytes());
        plane1.extend_from_slice(&1u16.to_le_bytes());
        plane1.extend_from_slice(&0x001Fu16.to_le_bytes());
        plane1.extend_from_slice(&0x7C01u16.to_le_bytes());
        plane1.extend_from_slice(&0u16.to_le_bytes());
        let mut body = Vec::new();
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&0x555Au16.to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        for (frame, (a, b, c)) in [
            ((0i16, 0i16, 4u16, 3u16), (0u32, 16u32, 4u32)),
            ((2, 1, 4, 3), (20, 12, 2)),
        ] {
            body.extend_from_slice(&frame.0.to_le_bytes());
            body.extend_from_slice(&frame.1.to_le_bytes());
            body.extend_from_slice(&frame.2.to_le_bytes());
            body.extend_from_slice(&frame.3.to_le_bytes());
            body.extend_from_slice(&a.to_le_bytes());
            body.extend_from_slice(&b.to_le_bytes());
            body.extend_from_slice(&c.to_le_bytes());
        }
        body.extend_from_slice(&plane0);
        body.extend_from_slice(&[0x40, 0x80, 0xFF, 0x40]);
        body.extend_from_slice(&plane1);
        body.extend_from_slice(&[0xFF, 0xFF]);
        let archive = opa_blob(&[("Note_Click.oja", &body)]);
        let figure = load_archive_sprite(&archive, "Note_Click.oja").unwrap();
        assert_eq!(figure.len(), 2, "one layer per stored frame");
        let layer = figure.layer(0).unwrap();
        assert_eq!((layer.dx, layer.dy), (0, 0));
        assert_eq!(layer.colorkey, 0x0000);
        assert_eq!(
            layer.image.words,
            vec![
                0x7C00, 0x03E0, 0x7C02, 0x7C03, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
                0x0000, 0x0000
            ]
        );
        let layer = figure.layer(1).unwrap();
        assert_eq!((layer.dx, layer.dy), (2, 1));
        assert_eq!(layer.image.words[5], 0x001F);
        assert_eq!(layer.image.words[6], 0x7C01);
    }

    /// All three 7.55 `.oja` bodies load and render (skips without
    /// `install-root`): the tap and long-note click rings decode six
    /// shrinking centred frames each, the jam flash all sixteen
    /// stored frames — ten tabulated 300x300 flash frames plus the
    /// six recordless trailing frames at the inherited `(0, 0)`
    /// anchor. Their alpha/mask planes ride the parser for a future
    /// blended paint; the figures paste the pixel planes opaque.
    #[test]
    fn oja_bodies_render_from_the_archive() {
        let Some(archive) = install_archive("Playing1.opi") else {
            return;
        };
        for (name, frames) in [
            ("Note_Click.oja", 6),
            ("LongNote_Effect.oja", 6),
            ("Playing_Effect_Jam.oja", 16),
        ] {
            let figure = load_archive_sprite(&archive, name)
                .unwrap_or_else(|| panic!("{name} must route to a figure"));
            assert_eq!(figure.len(), frames, "{name} stored frames");
            let layer = figure.layer(0).unwrap();
            assert!(layer.image.w > 0 && layer.image.h > 0, "{name} frame size");
        }
        let jam = load_archive_sprite(&archive, "Playing_Effect_Jam.oja").unwrap();
        let tail = jam.layer(15).expect("recordless trailing frame");
        assert_eq!((tail.dx, tail.dy), (0, 0), "inherited body anchor");
        assert_eq!((tail.image.w, tail.image.h), (300, 300));
    }

    /// All ten 7.55 prefixed `.oji` bodies load and render (skips
    /// without `install-root`): one composed layer per stored frame —
    /// `SelectAlbum_Bar` 15, `SelectMusic_Bar` 10, the notice boards
    /// and loading rank view one each, the four `BG_Ani` background
    /// frames and the shadow, plus the three repeated-y bodies
    /// (gap ④). The full-row bodies ride the positive deltas
    /// codec-coverage pinned (+92..+902); `BG_Ani`/`Avatar_Shadow`
    /// walk the same row groups with narrower spans (transparent
    /// margins skipped, negative deltas); the repeated-y trio stores
    /// frame outlines only, sharing rows by repeating the `y` word.
    #[test]
    fn oji_prefixed_bodies_render_from_the_archives() {
        let Some(interface) = install_archive("Interface1.opi") else {
            return;
        };
        let Some(playing) = install_archive("Playing1.opi") else {
            return;
        };
        for (archive, name, frames) in [
            (&interface, "SelectAlbum_Bar.oji", 15),
            (&interface, "SelectMusic_Bar.oji", 10),
            (&interface, "Lanking_NoticeBoard.oji", 1),
            (&playing, "Lanking_NoticeBoard_Playing.oji", 1),
            (&playing, "Playing_Loding_Lankview.oji", 1),
            (&playing, "BG_Ani.oji", 4),
            (&playing, "Avatar_Shadow.oji", 1),
            (&playing, "Playing_ChatWindow.oji", 1),
            (&interface, "Myroom_MybagSelect.oji", 1),
            (&interface, "Shop_MybagSelect.oji", 1),
        ] {
            let figure = load_archive_sprite(archive, name)
                .unwrap_or_else(|| panic!("{name} must route to a figure"));
            assert_eq!(figure.len(), frames, "{name} stored frames");
        }
    }

    /// The three repeated-y `.oji` bodies (codec gap ④) load and
    /// render (skips without `install-root`): their stored bytes
    /// repeat the row-group `y` word to keep outline rows shared, and
    /// the strict in-order walk accepts exactly that — rows stay
    /// ordered, every row carried at least once, the walk ends on row
    /// `h − 1`.
    #[test]
    fn oji_repeated_y_bodies_render_from_the_archives() {
        let Some(interface) = install_archive("Interface1.opi") else {
            return;
        };
        let Some(playing) = install_archive("Playing1.opi") else {
            return;
        };
        for (archive, name, frames) in [
            (&playing, "Playing_ChatWindow.oji", 1),
            (&interface, "Myroom_MybagSelect.oji", 1),
            (&interface, "Shop_MybagSelect.oji", 1),
        ] {
            let figure = load_archive_sprite(archive, name)
                .unwrap_or_else(|| panic!("{name} must route to a figure"));
            assert_eq!(figure.len(), frames, "{name} stored frames");
        }
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
            role: PlacementRole::Single,
            id: "IDC_IMAGE_SPLAY_TIMEBAR".to_owned(),
            code: 0x1B50_0003,
            sprite: Some("Normal_TimeBar.ojs".to_owned()),
            rect,
        }
    }

    /// One red pixel at a native anchor plus a bound placement whose
    /// rect DISAGREES with the anchor (`BpPoint_Left_bar.ojs`: anchor
    /// `(2,266)` vs `State_Playing.bnd` rect[3] `(200,3)`), so the
    /// draw chain is observable: art lands on the record anchor alone
    /// (verified draw path: the blit adds the per-frame anchor to the
    /// view position; `.bnd` rects are input bounds, never read at
    /// draw), and the reference photo pins it — the BP gauge shows at
    /// the far-left edge (2,266), not mid-screen (200,3).
    #[test]
    fn bound_art_paints_at_its_record_anchor_not_its_rect() {
        let figure = anchored_red_figure(2, 266);
        let figures = HashMap::from([("BpPoint_Left_bar.ojs".to_owned(), figure)]);
        let placement = ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_IMAGE_SPLAY_BUFFER1".to_owned(),
            code: 0x1B5_0004,
            sprite: Some("BpPoint_Left_bar.ojs".to_owned()),
            rect: Rect {
                x: 200,
                y: 3,
                w: 28,
                h: 28,
            },
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_placed_figure(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placement,
            &figures,
            false,
        );
        let ink = o2jam_render::pack_xrgb(255, 0, 0);
        assert_eq!(buf[266 * 800 + 2], ink, "art lands on the record anchor");
        assert_ne!(buf[3 * 800 + 200], ink, "the rect never positions art");
    }

    #[test]
    fn duplicated_anchor_and_rect_agree_on_one_pixel() {
        // Verified duplication on 7.55: bound bodies carry absolute
        // record anchors equal to their rect origins
        // (`Normal_TimeBar.ojs` (489,556) == `State_Playing.bnd`
        // rect[2], `Playing_Exit.ojs` (751,554) == rect[0],
        // `Wait_TeamA.ojs` (652,65) == `State_Waiting.bnd` rect[3]),
        // so anchor-drawn art sits exactly where the rect bounds it.
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
            false,
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
            false,
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
            false,
        );
        assert_eq!(buf[102], o2jam_render::pack_xrgb(255, 0, 0));
        assert_eq!(buf[0], 0);
        assert_eq!(anchor_in_frame(204, 0, 400, 300), (102, 0));
        assert_eq!(anchor_in_frame(204, 558, 800, 600), (204, 558));
    }

    /// The `Wait_Cover_Avatar.ojs` shape: one 1x1 layer anchored
    /// `(0, 0)`, bound to a seat rect away from the origin. The rect
    /// stages the view position (ui-draw.md's open view-from-rect
    /// role); the anchor adds nothing — the art lands on the rect
    /// origin, never on the frame corner.
    #[test]
    fn zero_anchor_body_paints_at_its_bound_rect_origin() {
        let figure = anchored_red_figure(0, 0);
        let figures = HashMap::from([("Wait_Cover_Avatar.ojs".to_owned(), figure)]);
        let placement = ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_IMAGE_SWAIT_COVERAVATAR1".to_owned(),
            code: 0x165_003D,
            sprite: Some("Wait_Cover_Avatar.ojs".to_owned()),
            rect: Rect {
                x: 300,
                y: 200,
                w: 141,
                h: 175,
            },
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_placed_figure(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placement,
            &figures,
            false,
        );
        assert_eq!(
            buf[200 * 800 + 300],
            o2jam_render::pack_xrgb(255, 0, 0),
            "the bound rect stages the view position"
        );
        assert_ne!(
            buf[0],
            o2jam_render::pack_xrgb(255, 0, 0),
            "a zero anchor leaves the art off the frame corner"
        );
    }

    #[test]
    fn one_anchored_layer_keeps_the_whole_figure_on_its_anchors() {
        // Any anchor offset makes the whole body self-positioned: both
        // layers draw at their anchors from the frame origin, and the
        // bound rect never moves them (the verified anchor chain).
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x1F, 0x00], 1, 1).unwrap(),
            dx: 2,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let figures = HashMap::from([("Room_Mode.ojs".to_owned(), figure)]);
        let placement = ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_IMAGE_SROOM_MODE".to_owned(),
            code: 0x135_0028,
            sprite: Some("Room_Mode.ojs".to_owned()),
            rect: Rect {
                x: 500,
                y: 400,
                w: 44,
                h: 18,
            },
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_placed_figure(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placement,
            &figures,
            false,
        );
        assert_eq!(buf[0], o2jam_render::pack_xrgb(255, 0, 0));
        assert_eq!(buf[2], o2jam_render::pack_xrgb(0, 0, 255));
        assert_ne!(buf[400 * 800 + 500], o2jam_render::pack_xrgb(255, 0, 0));
    }

    /// The three room modes the photo and the wire name: solo 7-key
    /// (the reference photo), solo 3-key, team 7-key.
    #[test]
    fn playing_members_select_by_room_mode() {
        let solo7 = PlayingArtMode {
            key_mode: 2,
            team_play: false,
        };
        // Solo 7-key set, verified against the reference photo: the
        // 7-key note background, the left life gauge, the left BP
        // buffer — never the 3-key/P2P note backgrounds, the right
        // buffer, or the unused gauge back shell.
        assert!(playing_member_selected("Note_BG.ojs", &solo7));
        assert!(playing_member_selected("BpPoint_Left_bar.ojs", &solo7));
        assert!(playing_member_selected("Life_gauge_left_bar.ojs", &solo7));
        assert!(!playing_member_selected("3Key_Note_BG.ojs", &solo7));
        assert!(!playing_member_selected("P2P3KeyNote_BG.ojs", &solo7));
        assert!(!playing_member_selected("P2P7KeyNote_BG.ojs", &solo7));
        assert!(!playing_member_selected("BpPoint_Right_bar.ojs", &solo7));
        assert!(!playing_member_selected("Life_gauge_back.ojs", &solo7));
        // The stage body and every non-family sprite stage regardless.
        assert!(playing_member_selected("_Playing_BG.ojs", &solo7));
        assert!(playing_member_selected("Playing_Menu.ojs", &solo7));
        assert!(playing_member_selected("Normal_TimeBar.ojs", &solo7));
        // Team play flips the family to the P2P members.
        let team7 = PlayingArtMode {
            key_mode: 2,
            team_play: true,
        };
        assert!(playing_member_selected("P2P7KeyNote_BG.ojs", &team7));
        assert!(playing_member_selected("BpPoint_Right_bar.ojs", &team7));
        assert!(playing_member_selected("Life_gauge_back.ojs", &team7));
        assert!(!playing_member_selected("Note_BG.ojs", &team7));
        assert!(!playing_member_selected("BpPoint_Left_bar.ojs", &team7));
        // Solo 3-key rooms take the 3-key note background.
        let solo3 = PlayingArtMode {
            key_mode: 0,
            team_play: false,
        };
        assert!(playing_member_selected("3Key_Note_BG.ojs", &solo3));
        assert!(!playing_member_selected("Note_BG.ojs", &solo3));
    }

    /// The frame cut rides the family table: `Note_BG.ojs` loads its
    /// exe-selected image 0 only; every other sprite loads whole (the
    /// other family members are single-frame bodies).
    #[test]
    fn note_bg_frame_cut_selects_image_zero() {
        assert_eq!(playing_member_frame("Note_BG.ojs"), Some(0));
        assert_eq!(playing_member_frame("3Key_Note_BG.ojs"), None);
        assert_eq!(playing_member_frame("P2P7KeyNote_BG.ojs"), None);
        assert_eq!(playing_member_frame("Playing_BG.ojs"), None);
    }

    /// Loading through the room mode leaves the unselected members'
    /// slots empty (the exe slot-loader rule: a miss loads nothing and
    /// an empty slot paints nothing).
    #[test]
    fn room_mode_loads_leave_unselected_slots_empty() {
        let red = sprite55_blob(1, 0xFFFF, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[
            ("Note_BG.ojs", &red),
            ("P2P7KeyNote_BG.ojs", &red),
            ("Playing_BG.ojs", &red),
        ]);
        let solo7 = PlayingArtMode {
            key_mode: 2,
            team_play: false,
        };
        let names = [
            "_Playing_BG.ojs".to_owned(),
            "Note_BG.ojs".to_owned(),
            "P2P7KeyNote_BG.ojs".to_owned(),
        ];
        let figures = load_scene_figures_for_room(&archive, &names, None, &solo7);
        assert!(figures.contains_key("Note_BG.ojs"));
        assert!(figures.contains_key("_Playing_BG.ojs"));
        assert!(!figures.contains_key("P2P7KeyNote_BG.ojs"));
    }

    /// Two-frame body shaped like the real `Note_BG.ojs` (Playing1.opi
    /// body walk, `docs/notebg-cover.md`): two identical 213x484 raw
    /// RGB555 strips, image 0 anchored (20,0), image 1 anchored
    /// (567,0), colorkey `0x0000`.
    fn two_strip_note_bg() -> Vec<u8> {
        let (w, h) = (213u16, 484u16);
        let psize = u32::from(w) * u32::from(h) * 2;
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&2u16.to_le_bytes());
        blob.extend_from_slice(&0x0000u16.to_le_bytes());
        for (index, (ax, ay)) in [(20u16, 0u16), (567u16, 0u16)].into_iter().enumerate() {
            let index = u32::try_from(index).unwrap_or(0);
            blob.extend_from_slice(&ax.to_le_bytes());
            blob.extend_from_slice(&ay.to_le_bytes());
            blob.extend_from_slice(&w.to_le_bytes());
            blob.extend_from_slice(&h.to_le_bytes());
            blob.extend_from_slice(&(index * psize).to_le_bytes());
            blob.extend_from_slice(&psize.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
        }
        let red = [0x00u8, 0x7C];
        blob.extend(std::iter::repeat_n(red, psize as usize).flatten());
        blob
    }

    /// The parity residual (docs/parity/playing.md): our shot
    /// painted the `Note_BG.ojs` right strip at (570,458)–(774,478)
    /// over the venue deck. The exe paints BACK2 as a single-frame
    /// view — image 0, the left strip — because no exe writer selects
    /// image 1 (whole-image dword sweep: zero `0x1B500072` hits; the
    /// view draws one frame, `docs/ui-draw.md` steps 3–6), and the
    /// reference photo pins venue over that band. The room-mode load
    /// must cut the figure to the selected frame.
    #[test]
    fn note_bg_room_load_keeps_only_the_left_strip() {
        let archive = opa_blob(&[("Note_BG.ojs", &two_strip_note_bg())]);
        let solo7 = PlayingArtMode {
            key_mode: 2,
            team_play: false,
        };
        let figures =
            load_scene_figures_for_room(&archive, &["Note_BG.ojs".to_owned()], None, &solo7);
        assert_eq!(
            figures.get("Note_BG.ojs").map(AvatarFigure::len),
            Some(1),
            "the exe view paints image 0; image 1 stays unloaded"
        );
        let placed = vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_IMAGE_SPLAY_BACK2".to_owned(),
            code: 0x1B50_0072,
            sprite: Some("Note_BG.ojs".to_owned()),
            rect: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
        }];
        let mut buf = vec![0u32; 800 * 600];
        paint_behind_placements(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placed,
            &figures,
            &[],
        );
        let red_ink = o2jam_render::pack_xrgb(255, 0, 0);
        assert_eq!(buf[20], red_ink, "the left strip paints at (20,0)");
        assert_eq!(buf[567], 0, "the right strip never paints at (567,0)");
        assert_eq!(buf[458 * 800 + 570], 0, "the checked band stays venue");
        assert_eq!(buf[477 * 800 + 773], 0, "band bottom-right corner too");
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
            role: PlacementRole::Single,
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
                role: PlacementRole::Single,
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
                role: PlacementRole::Single,
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
    fn scene_figures_paint_zero_anchor_body_at_its_bound_rect() {
        // The verified draw chain, refined for zero anchors: the
        // record anchor (0,0) contributes no position, so the bound
        // rect stages the view — the body lands on the scaled rect
        // origin (400,0) → (40,0) on an 80-wide frame, not the frame
        // corner; ordinal pairing still binds the rect.
        let red = sprite55_blob(1, 0xFFFF, 1, 1, &[0x00, 0x7C]);
        let archive = opa_blob(&[("Room_List.ojs", &red)]);
        let figures = load_scene_figures(&archive, &["Room_List.ojs".to_owned()]);
        let list = o2jam_file::parse_control_list(&room_control_text()).unwrap();
        let block = scene_block(&list, ROOM_BLOCK).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(400, 0, 800, 600), (0, 0, 8, 8)])).unwrap();
        let placed = bind_scene_rects(block, &bnd);
        assert_eq!(
            (
                placed[0].rect.x,
                placed[0].rect.y,
                placed[0].rect.w,
                placed[0].rect.h
            ),
            (400, 0, 400, 600),
            "ordinal pairing still binds the rect"
        );
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
        assert_eq!(
            buf[40],
            o2jam_render::pack_xrgb(255, 0, 0),
            "the bound rect origin stages the view"
        );
        assert_ne!(
            buf[0],
            o2jam_render::pack_xrgb(255, 0, 0),
            "the zero anchor leaves the body off the frame corner"
        );
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
    fn bound_layers_paint_at_their_authored_positions() {
        // Verified draw math (draw-chain audit): each layer blits at
        // its own record anchor scaled into the frame; a body with no
        // anchor offset takes the bound rect origin as its view. The
        // anchor-free 2-wide list body (rect 10,20) paints both pixels
        // from the rect origin; the button pixel lands at its record
        // anchor (2,3), never steered by its rect (40,30).
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
        assert_eq!(buf[0], 0);
        assert_eq!(buf[3 * 800 + 2], blue_ink);
        assert_eq!(buf[30 * 800 + 40], 0);
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
    fn spriteless_channel_block_stages_no_placements() {
        // Structural parity: the real 7.55 `STATE_CHANNEL` shape (bare
        // `BOUND`, zero quoted sprites) stages nothing through the shared
        // anchor path — the scene is flat background plus live rows.
        let block = o2jam_file::parse_control_list(
            &[
                "NUMBER_OF_STATE 0x01",
                "STATE_CHANNEL 0x27",
                "{",
                "BOUND",
                "IDC_STATE_CHANNEL 0X12000000",
                "IDC_BACK_SCHANNEL_1 0X12100001",
                "IDC_BUTTON_SCHANNEL_BACK 0X12200002",
                "}",
                "",
            ]
            .join("\n"),
        )
        .unwrap()
        .blocks
        .pop()
        .unwrap();
        assert!(scene_sprite_names(&block).is_empty());
        assert!(anchor_placements(&block).is_empty());
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

    /// First opaque pixel of one layer plus its decoded color
    /// (colorkey-transparent pixels skipped). `None` when the layer is
    /// fully transparent.
    fn first_opaque_pixel(layer: &FigureLayer) -> Option<(i32, i32, u32)> {
        let at = (0..layer.image.w * layer.image.h)
            .find(|at| layer.image.words[*at] != layer.colorkey)?;
        let ink = layer.image.words[at];
        let (r, g, b) = o2jam_file::rgb555_to_rgb888(ink);
        Some((
            (at % layer.image.w) as i32,
            (at / layer.image.w) as i32,
            o2jam_render::pack_xrgb(r, g, b),
        ))
    }

    #[test]
    fn every_bound_body_paints_on_its_authored_position() {
        // Multi-entry proof of the shared placement rule on real 7.55
        // bytes (skips without `install-root`): every decodable
        // `.bnd`-claimed placement lands its first opaque pixel on its
        // authored position — the record anchor when the body carries
        // one (the verified draw chain: blit adds the anchor to the
        // view position), the bound rect origin when the body anchors
        // at (0,0) and the rect is its only placement data. Most bound
        // anchors duplicate their rect origins byte-exact
        // (docs/bnd-semantics.md; pairing `0x548640`, ev `3a869bb7…`),
        // and the reference photo pins the disagreeing pair
        // (`BpPoint_Left_bar.ojs` (2,266) vs rect[3] (200,3)).
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
                let Some(layer) = figure.layer(0) else {
                    continue;
                };
                let Some((px, py, ink)) = first_opaque_pixel(layer) else {
                    continue;
                };
                let anchor = (layer.dx, layer.dy);
                let anchored = figure.has_anchor_offset();
                let placement = ScenePlacement {
                    role: PlacementRole::Single,
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
                let mut buf = vec![0u32; 800 * 600];
                paint_placed_figure(
                    &mut Frame {
                        buf: &mut buf,
                        w: 800,
                        h: 600,
                    },
                    &placement,
                    &HashMap::from([(sprite.to_owned(), figure)]),
                    false,
                );
                let (ax, ay) = if anchored {
                    anchor_in_frame(anchor.0, anchor.1, 800, 600)
                } else {
                    (placement.rect.x, placement.rect.y)
                };
                let at = (ay + py) as usize * 800 + (ax + px) as usize;
                assert_eq!(
                    buf[at], ink,
                    "{} ({sprite}) must paint on its authored position",
                    placed.entry.id
                );
                proven += 1;
            }
        }
        assert!(
            proven >= 40,
            "expected the bound-run proof to cover 40+ entries, got {proven}"
        );
    }

    #[test]
    fn avatar_breathing_frame_clock_guard() {
        assert_eq!(avatar_breathing_frame(0), 0);
        let f250 = avatar_breathing_frame(250);
        assert_eq!(f250, 1);
        assert_ne!(f250, avatar_breathing_frame(0));
        assert_eq!(avatar_breathing_frame(12 * 250), 0);
        // determinism: same instant twice
        assert_eq!(avatar_breathing_frame(1000), avatar_breathing_frame(1000));
    }
}
