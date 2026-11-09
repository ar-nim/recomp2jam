//! `o2jam-text` — CP949 decode plus system-font text rasterization.
//!
//! Sprint S0.1: map 7.55 client text bytes to shaped pixels without
//! vendoring a font. Owner decision `e8d01` (repo memory): glyph
//! rasterization uses a modern cross-platform stack (`ab_glyph`
//! over the system OFL face), never exe GDI parity — the exe still
//! pins placement, sizes, and ink (`docs/gdi-text.md`).
//!
//! Evidence pins:
//!
//! - CP949 is the client text encoding: K claim C10
//!   (`docs/claims.md` §C) — the exe converts CP949 at codepage
//!   `0x3b5` (949) in the converter at VA `0x406360`
//!   (`MultiByteToWideChar` call at VA `0x40639a`).
//!   [`decode::decode_cp949`] mirrors it with the WHATWG `euc-kr`
//!   index, which covers the CP949 repertoire (same mapping as
//!   `o2jam-file::decode_control_text`).
//! - The system face is Noto Sans CJK KR (OFL) under
//!   `/usr/share/fonts/noto-cjk/NotoSansCJK-*.ttc`, parsed at its
//!   KR face index (1, verified against the TTC name table: face 0
//!   is JP, face 1 is KR). No font bytes live in this repo; when
//!   the file is absent [`font::system_font`] returns `None` and
//!   drawing degrades to the ASCII stand-in
//!   ([`font::ascii_stand_in`]) — no test in this crate fails for
//!   that reason, and nothing here reads `install-root/`.
//!
//! The crate is self-contained: no other crate depends on it yet,
//! so the [`font::Image`] canvas and the [`font::Mask`] raster are
//! the whole rendering surface.

pub mod decode;
pub mod font;

pub use decode::{decode_cp949, decode_cp949_lossy, DecodeError};
pub use font::{
    ascii_stand_in, draw_text, draw_text_cp949, measure_text, system_font, Image, Mask, RasterFont,
    KR_FACE_INDEX,
};
