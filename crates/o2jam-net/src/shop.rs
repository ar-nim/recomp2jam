//! Shop messages: catalog sync, sell/equip and rename flows.
//!
//! Covers `SyncItemPurchase` 5000/5001 pieces, `SellItem` 5002/5003,
//! `EquipItem` 5004/5005 and `ChangeName`/`CheckName` 5030-5033. Layouts
//! mirror Memoryer's `Source/Memoryer` field orders (clean-room C#); exe
//! anchors stay on each type, Memoryer-only shapes stay labeled.

use super::error::Error;
use super::frame::Frame;
use super::message::{decode_empty_frame, encode_empty, EmptyMessage, WireMessage};
use super::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};

/// `EquipItem` 5004 request: wear `item_id` in equipment `slot`.
///
/// Body is two little-endian `i32` words, slot first. The send
/// builder stores the opcode as `u16`, stamps length `4`, then
/// appends both args as 4-byte words (VA `0x4a4ec8`, file
/// `0xa4ec8`); both C→S call sites pass slot then item (VAs
/// `0x4a4af4`/`0x4a4bcb`, files `0xa4af4`/`0xa4bcb`). The server
/// echoes the result as [`EquipItemResponse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipItem {
    /// Equipment slot index (`0..EQUIPMENT_SLOTS`;
    /// `crates/o2jam-net/src/character.rs`). The client derives it
    /// from the item record and range-checks `0 <= slot < 0x11`
    /// (VA `0x4a4baa`, file `0xa4baa`).
    pub slot: i32,
    /// Item id from `Itemdata_Korea.dat` (`1..=1541`); `0` is the
    /// empty-slot value (see `CharacterInfo::equipment`).
    pub item_id: i32,
}

/// Body length of [`EquipItem`]: two `i32` words.
const EQUIP_ITEM_BODY_LEN: usize = 8;

impl WireMessage for EquipItem {
    const COMMAND: u16 = 5004;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(EQUIP_ITEM_BODY_LEN);
        body.extend_from_slice(&self.slot.to_le_bytes());
        body.extend_from_slice(&self.item_id.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < EQUIP_ITEM_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: EQUIP_ITEM_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > EQUIP_ITEM_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - EQUIP_ITEM_BODY_LEN));
        }
        Ok(Self {
            slot: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            item_id: i32::from_le_bytes([body[4], body[5], body[6], body[7]]),
        })
    }
}

/// `EquipItem` 5005 response: result plus a four-word success tail.
///
/// Body is one little-endian `i32` result; when it is `0` four more
/// `i32` words follow (20 B total), otherwise the body ends (4 B).
/// The canonical S→C handler (VA `0x560cf0`, file `0x160cf0`; index 2
/// of the `0x138b`-based receive switch, stub VA `0x558853`, file
/// `0x158853`) reads the first word via the fixed-size reader
/// (`0x555d20`), returns at once unless it is `0`, then reads four
/// more words the same way and forwards the first and third tail
/// words with opcode `0x138d` and `0x12` into the UI event queue
/// (`0x478d50`, `ecx 0x762b44`): the queue arg at `0x10` carries the
/// first tail word and the arg at `0x14` carries the third tail word.
/// Result `1` and any other nonzero value exit with no further reads.
/// The shop-context `0x138d` consumer (VA `0x4a5fe8`, file `0xa5fe8`,
/// reached via `cmpl $0x138d` at VAs `0x4a5570`/`0x4a5586`) looks up the
/// `0x14` word in the shop list (`0x537b80` on `0x77ccbc`, VA `0x4a5ff8`)
/// and feeds its low word to the equip-apply function (`0x4a3c80`, VAs
/// `0x4a6089`/`0x4a60ea`; zero selects the remove path), while the `0x10`
/// word indexes the slot table (`0x64ca38`, VA `0x4a6008`) and is the
/// remove key (`0x4a3dc0`, VA `0x4a60b9`). The second and fourth tail
/// words are read to advance the packet cursor and then dropped:
/// wire-present but unconsumed, so modeled as reserved.
/// See [`EquipTail`] for the pinned word roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipTail {
    /// First tail word: slot selector forwarded as queue arg `0x10`.
    /// Indexes `0x64ca38` (VA `0x4a6008`); zero takes the default
    /// (`0x77ed38`) path, and the word is the remove key (`0x4a3dc0`).
    pub slot_select: i32,
    /// Second tail word: read via `0x555d20` then dropped.
    /// Wire-present but unconsumed; kept for exact framing.
    pub reserved_a: i32,
    /// Third tail word: shop entry/item id forwarded as queue arg `0x14`.
    /// Lookup key for `0x77ccbc` via `0x537b80` (VA `0x4a5ff8`); the low
    /// word feeds equip-apply (`0x4a3c80`), zero selects remove.
    pub entry_id: i32,
    /// Fourth tail word: read via `0x555d20` then dropped.
    /// Wire-present but unconsumed; kept for exact framing.
    pub reserved_b: i32,
}

impl EquipTail {
    /// Append the four words in wire order.
    fn encode_inner(&self, body: &mut Vec<u8>) {
        for word in [
            self.slot_select,
            self.reserved_a,
            self.entry_id,
            self.reserved_b,
        ] {
            body.extend_from_slice(&word.to_le_bytes());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipItemResponse {
    /// Equip verdict (`0` carries the success tail).
    pub result: i32,
    /// Four tail words; `Some` exactly when `result == 0`.
    pub tail: Option<EquipTail>,
}

/// Body length of a bare [`EquipItemResponse`] result.
const EQUIP_ITEM_RESPONSE_MIN_LEN: usize = 4;
/// Body length of an [`EquipItemResponse`] success (result + tail).
const EQUIP_ITEM_RESPONSE_FULL_LEN: usize = 20;

/// Read one little-endian `i32` at `body[offset..offset + 4]`.
fn read_word_at(body: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        body[offset],
        body[offset + 1],
        body[offset + 2],
        body[offset + 3],
    ])
}

impl WireMessage for EquipItemResponse {
    const COMMAND: u16 = 5005;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(EQUIP_ITEM_RESPONSE_FULL_LEN);
        body.extend_from_slice(&self.result.to_le_bytes());
        if let Some(tail) = &self.tail {
            tail.encode_inner(&mut body);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < EQUIP_ITEM_RESPONSE_MIN_LEN {
            return Err(Error::TruncatedMessage {
                need: EQUIP_ITEM_RESPONSE_MIN_LEN,
                have: body.len(),
            });
        }
        let result = read_word_at(body, 0);
        if result != 0 {
            if body.len() > EQUIP_ITEM_RESPONSE_MIN_LEN {
                return Err(Error::TrailingBytes(
                    body.len() - EQUIP_ITEM_RESPONSE_MIN_LEN,
                ));
            }
            return Ok(Self { result, tail: None });
        }
        if body.len() < EQUIP_ITEM_RESPONSE_FULL_LEN {
            return Err(Error::TruncatedMessage {
                need: EQUIP_ITEM_RESPONSE_FULL_LEN,
                have: body.len(),
            });
        }
        if body.len() > EQUIP_ITEM_RESPONSE_FULL_LEN {
            return Err(Error::TrailingBytes(
                body.len() - EQUIP_ITEM_RESPONSE_FULL_LEN,
            ));
        }
        Ok(Self {
            result,
            tail: Some(EquipTail {
                slot_select: read_word_at(body, 4),
                reserved_a: read_word_at(body, 8),
                entry_id: read_word_at(body, 12),
                reserved_b: read_word_at(body, 16),
            }),
        })
    }
}

// `SyncItemPurchase` 5000 (cart sync request) sends a bare header:
// the opcode-arg builder case for `0x1388` (VA `0x4f3fa8`, file
// `0xf3fa8`, dispatched on the id word at VA `0x4f3f7f`, file `0xf3f7f`)
// writes the opcode word plus a length word of 4 with no appends, and
// the common tail (VA `0x4f4051`, file `0xf4051`) transmits it unchanged.
empty_message!(SyncItemPurchase, 5000);

// `SyncMusicPurchase` 5010 (song-shop buy request) sends a bare header:
// the `0x138a`-based send builder (file `0xa4df0`) case at index 8
// (`0x138a` + 8 = `0x1392` = 5010) is the empty-send case — opcode
// word plus a length word of 4, no appends (`docs/shop-buy.md`).
// The name follows `docs/net-protocol.md` (Memoryer corroboration).
// 5011 reply shape (Memoryer `SyncMusicPurchaseResponse`, 2026-04):
// wallets (Gem, Point, O2Cash, MusicCash, ItemCash, CashPoint) plus an
// i16-counted MusicList of {MusicId u16, Day/Month/Year/Hour/Minute u8}
// expiry stamps, i16-counted ItemGiftBox and MusicGiftBox, and a trailing
// FreePassExtensionDays i32. Owned-state for the board derives from the
// MusicList ids (acquired songs) plus the FreeMusic flag from 4031.
empty_message!(SyncMusicPurchase, 5010);

/// `SyncMusicPurchase` 5011 response: wallets + acquired-music list +
/// gift boxes + free-pass extension days (reply shape pinned by #192).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicEntry {
    pub music_id: u16,
    pub day: u8,
    pub month: u8,
    pub year: u8,
    pub hour: u8,
    pub minute: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncMusicPurchaseResponse {
    pub gem: i32,
    pub point: i32,
    pub o2_cash: i32,
    pub music_cash: i32,
    pub item_cash: i32,
    pub cash_point: i32,
    pub music_list: Vec<MusicEntry>,
    /// Raw bytes of both i16-counted gift boxes (preserved for round-trip;
    /// inner element shape stays OPEN).
    pub gift_boxes_raw: Vec<u8>,
    pub free_pass_extension_days: i32,
}

impl super::message::WireMessage for SyncMusicPurchaseResponse {
    const COMMAND: u16 = 5011;
    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        for w in [
            self.gem,
            self.point,
            self.o2_cash,
            self.music_cash,
            self.item_cash,
            self.cash_point,
        ] {
            body.extend_from_slice(&w.to_le_bytes());
        }
        let n = self.music_list.len() as i16;
        body.extend_from_slice(&n.to_le_bytes());
        for m in &self.music_list {
            body.extend_from_slice(&m.music_id.to_le_bytes());
            body.extend_from_slice(&[m.day, m.month, m.year, m.hour, m.minute]);
        }
        body.extend_from_slice(&self.gift_boxes_raw);
        body.extend_from_slice(&self.free_pass_extension_days.to_le_bytes());
        body
    }
    fn decode_body(body: &[u8]) -> Result<Self, super::error::Error> {
        use super::error::Error;
        if body.len() < 24 + 2 + 4 {
            return Err(Error::TruncatedMessage {
                need: 30,
                have: body.len(),
            });
        }
        let mut o = 0;
        let rd = |b: &[u8], o: &mut usize| -> i32 {
            let v = i32::from_le_bytes([b[*o], b[*o + 1], b[*o + 2], b[*o + 3]]);
            *o += 4;
            v
        };
        let gem = rd(body, &mut o);
        let point = rd(body, &mut o);
        let o2_cash = rd(body, &mut o);
        let music_cash = rd(body, &mut o);
        let item_cash = rd(body, &mut o);
        let cash_point = rd(body, &mut o);
        // music list count i16
        let cnt = i16::from_le_bytes([body[o], body[o + 1]]) as usize;
        o += 2;
        let mut music_list = Vec::with_capacity(cnt);
        for _ in 0..cnt {
            if o + 7 > body.len() {
                return Err(Error::TruncatedMessage {
                    need: o + 7,
                    have: body.len(),
                });
            }
            let music_id = u16::from_le_bytes([body[o], body[o + 1]]);
            o += 2;
            let day = body[o];
            let month = body[o + 1];
            let year = body[o + 2];
            let hour = body[o + 3];
            let minute = body[o + 4];
            o += 5;
            music_list.push(MusicEntry {
                music_id,
                day,
                month,
                year,
                hour,
                minute,
            });
        }
        // gift boxes raw = everything except trailing 4 bytes
        if body.len() < o + 4 {
            return Err(Error::TruncatedMessage {
                need: o + 4,
                have: body.len(),
            });
        }
        let gift_boxes_raw = body[o..body.len() - 4].to_vec();
        let free_pass_extension_days = i32::from_le_bytes([
            body[body.len() - 4],
            body[body.len() - 3],
            body[body.len() - 2],
            body[body.len() - 1],
        ]);
        Ok(Self {
            gem,
            point,
            o2_cash,
            music_cash,
            item_cash,
            cash_point,
            music_list,
            gift_boxes_raw,
            free_pass_extension_days,
        })
    }
}

/// First 16 wire bytes of a `SyncItemPurchase` 5001 response: the
/// four-word wallet prefix (`Gem`, `Point`, `O2Cash`, `CashPoint`).
///
/// Field order follows Memoryer's `SyncItemPurchaseResponse`
/// (orders 0-3: `Gem`, `Point`, `O2Cash`, `CashPoint`) and matches the
/// S→C handler (VA `0x560a40`, file `0x160a40`), which reads these words
/// first via the fixed-size reader (`0x555d20`): into `0x768b44`
/// (file `0x160a59`), `0x768b48` (file `0x160a68`), `0x768b4c`
/// (file `0x160a77`), `0x768b58` (file `0x160a86`). The first word is
/// the gem slot (applied via `0x540090` to `0x77cd00+0x2c`, the same
/// slot the `SyncCashPoint` gem word and the `SellItem` success word
/// update) and the fourth is the cash-point slot (applied via `0x540d00`
/// to `0x77cd00+0x2128`, the same slot the `SyncCashPoint` cash word
/// updates). The second word (`Point`) has no reader anywhere in `.text`
/// (write-only). Shop prices live on the entry (`ecx 0x77cce8`): the
/// `+0x14` getter (`0x53c010`, file `0x13b010`) is the cash price and
/// the `+0x18` getter (`0x53c030`, file `0x13b030`) is the gem price.
/// The third prefix word (`O2Cash`) is the cash-price baseline: the
/// entry `+0x14` price is affordable when it does not exceed `O2Cash`
/// (VA `0x499f26`, file `0x99f26`, sibling file `0xc841d`) or the
/// matching shop wallet (`0x768b50` at file `0x99f39`, `0x768b54` at
/// file `0x99f7d`; sibling files `0xc8430`/`0xc8474`), while the entry
/// `+0x18` price is checked against gem (`0x768b44` at file `0x99fae`,
/// sibling file `0xc84a5`). Balances display alongside gem at VAs
/// `0x4a221b`/`0x4a3544`/`0x4f45c4` (files `0xa221b`/`0xa3544`/`0xf45c4`);
/// entry prices render via the same decimal renderer at files `0xa2961`
/// (`+0x14` cash) and `0xa2995` (`+0x18` gem), duplicated at files
/// `0xc66be`/`0xc6693` for the sibling shop.
/// There is no full 5001 message type: the entry list after this prefix
/// is sized from the GLOBAL `0x77cce4`, so the body is not decodable
/// standalone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncItemPurchasePrefix {
    /// Gem balance (applied via `0x540090`; entry `+0x18` gem price
    /// is checked against it at file `0x99fae`).
    pub gem: i32,
    /// Distribution points: stored to `0x768b48`, never read in `.text`.
    pub point: i32,
    /// Premium cash: entry `+0x14` cash-price baseline (`0x99f26`)
    /// and displayed with gem.
    pub o2_cash: i32,
    /// Cash points (applied via `0x540d00`).
    pub cash_point: i32,
}

/// Wire length of [`SyncItemPurchasePrefix`]: four `i32` words.
const SYNC_ITEM_PURCHASE_PREFIX_LEN: usize = 16;

impl SyncItemPurchasePrefix {
    /// Parse the prefix from the front of `body`, returning value and rest.
    ///
    /// # Errors
    ///
    /// `Error::TruncatedMessage` when fewer than 16 bytes remain.
    pub fn decode_prefix(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < SYNC_ITEM_PURCHASE_PREFIX_LEN {
            return Err(Error::TruncatedMessage {
                need: SYNC_ITEM_PURCHASE_PREFIX_LEN,
                have: body.len(),
            });
        }
        Ok((
            Self {
                gem: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
                point: i32::from_le_bytes([body[4], body[5], body[6], body[7]]),
                o2_cash: i32::from_le_bytes([body[8], body[9], body[10], body[11]]),
                cash_point: i32::from_le_bytes([body[12], body[13], body[14], body[15]]),
            },
            &body[SYNC_ITEM_PURCHASE_PREFIX_LEN..],
        ))
    }

    /// Append the four words in wire order.
    pub fn encode_inner(&self, body: &mut Vec<u8>) {
        for word in [self.gem, self.point, self.o2_cash, self.cash_point] {
            body.extend_from_slice(&word.to_le_bytes());
        }
    }
}

/// Two `SyncItemPurchase` 5001 post-list wallet words (`MusicCash`,
/// `ItemCash`) read after the `0x77cce4`-sized entry list.
///
/// Field order follows Memoryer's `SyncItemPurchaseResponse` (orders 5-6).
/// The S→C handler reads them via the fixed-size reader (`0x555d20`)
/// into `0x768b50` (file `0x160b03`) and `0x768b54` (file `0x160b12`).
/// Both are the `O2Cash` alternative for the entry `+0x14` cash price
/// (`0x53c010`, file `0x13b010`): it is affordable when it does not
/// exceed `O2Cash` or the matching wallet (`0x768b50` at file `0x99f39`,
/// `0x768b54` at file `0x99f7d`; same pair duplicated for the sibling
/// shop at files `0xc841d`/`0xc8430` and `0xc8461`/`0xc8474`). Order 5 is
/// the music-shop wallet and order 6 the item-shop wallet (Memoryer
/// order, `t_o2jam_charcash` column order, `CharacterInfo` field order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncItemPurchaseWallets {
    /// Music-shop cash (`0x768b50`).
    pub music_cash: i32,
    /// Item-shop cash (`0x768b54`).
    pub item_cash: i32,
}

/// Wire length of [`SyncItemPurchaseWallets`]: two `i32` words.
const SYNC_ITEM_PURCHASE_WALLETS_LEN: usize = 8;

impl SyncItemPurchaseWallets {
    /// Parse the wallet pair from the front of `body`.
    ///
    /// # Errors
    ///
    /// `Error::TruncatedMessage` when fewer than 8 bytes remain.
    pub fn decode_prefix(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < SYNC_ITEM_PURCHASE_WALLETS_LEN {
            return Err(Error::TruncatedMessage {
                need: SYNC_ITEM_PURCHASE_WALLETS_LEN,
                have: body.len(),
            });
        }
        Ok((
            Self {
                music_cash: read_word_at(body, 0),
                item_cash: read_word_at(body, 4),
            },
            &body[SYNC_ITEM_PURCHASE_WALLETS_LEN..],
        ))
    }

    /// Append the pair in wire order.
    pub fn encode_inner(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.music_cash.to_le_bytes());
        body.extend_from_slice(&self.item_cash.to_le_bytes());
    }
}

/// One `SyncItemPurchase` 5001 restock pair: 8 wire bytes (one
/// `AttributiveItemInfo`: `AttributiveItemId` + `ItemCount`, Memoryer's
/// `SyncItemPurchaseResponse` order 7, `Int32`-prefixed list with count
/// M at file `0x160b8a`).
///
/// The list reads two words per entry (files `0x160bb6`/`0x160bc4`). The first word is the shop
/// entry id, validated by `0x537e90` on `0x77ccbc` (whose `0xff`
/// return skips the pair, file `0x160bf5`) and then resolved by
/// `0x538240` plus item-record lookups (`0x52dba0` on `0x77b540`,
/// with `[eax+0xa]`/`[eax+0xb]` feeding render `0x4a9b60` on
/// `0x7795fc`). The exe forwards only the LOW word of the count
/// (`movzx cx`, file `0x160bd8`); the high half is read to advance the
/// cursor then dropped (zero in practice: counts fit in `u16`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopRestockPair {
    /// Attributive item id (lookup key for `0x77ccbc` / `0x77b540`).
    pub attributive_item_id: i32,
    /// Stack count added into the shop entry by `0x538240`.
    pub item_count: i32,
}

/// Wire length of [`ShopRestockPair`]: two `i32` reads.
const SHOP_RESTOCK_PAIR_LEN: usize = 8;

impl ShopRestockPair {
    /// Parse one pair from the front of `body`, returning value and rest.
    ///
    /// # Errors
    ///
    /// `Error::TruncatedMessage` when fewer than 8 bytes remain.
    pub fn decode_prefix(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < SHOP_RESTOCK_PAIR_LEN {
            return Err(Error::TruncatedMessage {
                need: SHOP_RESTOCK_PAIR_LEN,
                have: body.len(),
            });
        }
        Ok((
            Self {
                attributive_item_id: read_word_at(body, 0),
                item_count: read_word_at(body, 4),
            },
            &body[SHOP_RESTOCK_PAIR_LEN..],
        ))
    }

    /// Append the pair in wire order.
    pub fn encode_inner(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.attributive_item_id.to_le_bytes());
        body.extend_from_slice(&self.item_count.to_le_bytes());
    }
}

/// Trailing word of a `SyncItemPurchase` 5001 response: free-pass
/// extension days (`FreePassExtensionDays`, Memoryer's
/// `SyncItemPurchaseResponse` order 8).
///
/// The S→C handler reads one `i32` (file `0x160c54`), then forwards it
/// with event id `0x1389` into the UI queue (`0x478d50`; pushes at files
/// `0x160c68`/`0x160c75`). The shop-context `0x1389` consumer (VA
/// `0x4f4144`, file `0xf4144`, dispatched at file `0xf410e`) branches on
/// it: nonzero stores it to shop state `[ecx+0x64]` and notifies `0x13a1`
/// (VA `0x4f417f`, file `0xf417f`; `0x13a1` = 5025 `SyncFreePass`); zero
/// takes the `0x4f4300` refresh path (VA `0x4f419e`, file `0xf419e`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncItemPurchaseTail {
    /// Free-pass extension days (0 = plain shop refresh).
    pub free_pass_days: i32,
}

/// Wire length of [`SyncItemPurchaseTail`]: one `i32` word.
const SYNC_ITEM_PURCHASE_TAIL_LEN: usize = 4;

impl SyncItemPurchaseTail {
    /// Parse the trailing word from the front of `body`.
    ///
    /// # Errors
    ///
    /// `Error::TruncatedMessage` when fewer than 4 bytes remain.
    pub fn decode_prefix(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < SYNC_ITEM_PURCHASE_TAIL_LEN {
            return Err(Error::TruncatedMessage {
                need: SYNC_ITEM_PURCHASE_TAIL_LEN,
                have: body.len(),
            });
        }
        Ok((
            Self {
                free_pass_days: read_word_at(body, 0),
            },
            &body[SYNC_ITEM_PURCHASE_TAIL_LEN..],
        ))
    }

    /// Append the word in wire order.
    pub fn encode_inner(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.free_pass_days.to_le_bytes());
    }
}

/// `SellItem` 5002 request: sell the selected shop entry back.
///
/// Body is two little-endian `i32` words: the validated selection
/// index and the entry's id word. The send case (VA `0x4a510c`,
/// file `0xa510c`; index 0 of the `0x138a`-based send switch, byte
/// table file `0xa54ac`) appends exactly those two words via the
/// fixed-size writer (`0x555d70`): first the `0x77ccc0` selection
/// cell (bounds-checked against the entry count `0x77cce4`, entry
/// fetched via `0x4a3250`), then the entry pair's first word. The
/// two virtual-send call sites pass ignored zeros (VAs `0x49cee2`/
/// `0x4a4c9e`, files `0x9cee2`/`0xa4c9e`); the values come from
/// shop UI state, not the call. Exact game meaning of the id word
/// stays OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellItem {
    /// Selection index snapshot (`0x77ccc0`).
    pub selection: i32,
    /// Entry id word (first word of the fetched entry pair).
    pub entry: i32,
}

/// Body length of [`SellItem`]: two `i32` words.
const SELL_ITEM_BODY_LEN: usize = 8;

impl WireMessage for SellItem {
    const COMMAND: u16 = 5002;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SELL_ITEM_BODY_LEN);
        body.extend_from_slice(&self.selection.to_le_bytes());
        body.extend_from_slice(&self.entry.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SELL_ITEM_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SELL_ITEM_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SELL_ITEM_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - SELL_ITEM_BODY_LEN));
        }
        Ok(Self {
            selection: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            entry: i32::from_le_bytes([body[4], body[5], body[6], body[7]]),
        })
    }
}

/// `SellItem` 5003 response: the server's sell verdict.
///
/// Body is one `i32` result plus an optional gem refund present exactly
/// when the result is zero (Memoryer's `SellItemResponse`: `Invalid`
/// bool + nullable `Gem`). The S→C handler (VA `0x560c80`, file
/// `0x160c80`; index 0 of the `0x138b`-based receive switch, byte table
/// file `0x158ae0`) reads the first word and returns unless it is `0`,
/// then reads the second word into `0x768b44` (file `0x160caa`) and
/// applies it via `0x540090` (writes `0x77cd00+0x2c`, the shared gem
/// slot); the shop-context case (VA `0x4a5c76`, file `0xa5c76`) treats
/// `0` as success and `0xffffffa0` as failure. Code meanings stay OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellItemResponse {
    /// Sell verdict (`0` takes the success path).
    pub result: i32,
    /// Gem refund; `Some` exactly when `result == 0`.
    pub gem: Option<i32>,
}

/// Body length of a bare [`SellItemResponse`] result.
const SELL_ITEM_RESPONSE_MIN_LEN: usize = 4;
/// Body length of a [`SellItemResponse`] success (result + word).
const SELL_ITEM_RESPONSE_FULL_LEN: usize = 8;

impl WireMessage for SellItemResponse {
    const COMMAND: u16 = 5003;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SELL_ITEM_RESPONSE_FULL_LEN);
        body.extend_from_slice(&self.result.to_le_bytes());
        if let Some(gem) = self.gem {
            body.extend_from_slice(&gem.to_le_bytes());
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SELL_ITEM_RESPONSE_MIN_LEN {
            return Err(Error::TruncatedMessage {
                need: SELL_ITEM_RESPONSE_MIN_LEN,
                have: body.len(),
            });
        }
        let result = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        if result != 0 {
            if body.len() > SELL_ITEM_RESPONSE_MIN_LEN {
                return Err(Error::TrailingBytes(
                    body.len() - SELL_ITEM_RESPONSE_MIN_LEN,
                ));
            }
            return Ok(Self { result, gem: None });
        }
        if body.len() < SELL_ITEM_RESPONSE_FULL_LEN {
            return Err(Error::TruncatedMessage {
                need: SELL_ITEM_RESPONSE_FULL_LEN,
                have: body.len(),
            });
        }
        if body.len() > SELL_ITEM_RESPONSE_FULL_LEN {
            return Err(Error::TrailingBytes(
                body.len() - SELL_ITEM_RESPONSE_FULL_LEN,
            ));
        }
        Ok(Self {
            result,
            gem: Some(read_word_at(body, 4)),
        })
    }
}

/// `ChangeName` 5030 request: spend a rename coupon on `name`.
///
/// Body is a null-terminated name plus two `i32` words. The send
/// case (VA `0x4a520f`, file `0xa520f`; index 28 of the
/// `0x138a`-based send switch, byte table file `0xa54ac`)
/// appends the name from the rename buffer (object `+0xb8`) via
/// the string writer (`0x555e30` = `strlen + 1` bytes), then the
/// selection-cell snapshot (`0x77ccc0`) and one `0x537b80` lookup
/// word via the fixed-size writer (`0x555d70`). Live call sites
/// pass ignored zeros (VAs `0x49ce55`/`0x4a4cba`, files `0x9ce55`/
/// `0xa4cba`); the values come from shop UI state. The words'
/// game meaning stays OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeName {
    /// New nickname (null-terminated on the wire).
    pub name: String,
    /// Selection-cell snapshot (`0x77ccc0`).
    pub selection: i32,
    /// Lookup word for the rename entry.
    pub entry: i32,
}

/// Words after the name in a [`ChangeName`] body: two `i32`.
const CHANGE_NAME_WORDS_LEN: usize = 8;

impl WireMessage for ChangeName {
    const COMMAND: u16 = 5030;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = encode_null_terminated(&self.name, MAX_STRING_LEN);
        body.extend_from_slice(&self.selection.to_le_bytes());
        body.extend_from_slice(&self.entry.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (name, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
        let tail = &body[used..];
        if tail.len() < CHANGE_NAME_WORDS_LEN {
            return Err(Error::TruncatedMessage {
                need: CHANGE_NAME_WORDS_LEN,
                have: tail.len(),
            });
        }
        if tail.len() > CHANGE_NAME_WORDS_LEN {
            return Err(Error::TrailingBytes(tail.len() - CHANGE_NAME_WORDS_LEN));
        }
        Ok(Self {
            name,
            selection: i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]),
            entry: i32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]),
        })
    }
}

/// `ChangeName` 5031 response: the rename verdict.
///
/// Body is one little-endian `i32`. The S→C handler (VA `0x563950`,
/// file `0x163950`; index 28 of the `0x138b`-based receive switch,
/// byte table file `0x158ae0`) reads a single word: `0` runs the
/// rename applier (`0x540bf0`), anything else skips it. Code
/// meanings stay OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeNameResponse {
    /// Rename verdict (`0` = applied).
    pub result_code: i32,
}

/// Body length of [`ChangeNameResponse`]: one `i32` word.
const CHANGE_NAME_RESPONSE_BODY_LEN: usize = 4;

impl WireMessage for ChangeNameResponse {
    const COMMAND: u16 = 5031;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(CHANGE_NAME_RESPONSE_BODY_LEN);
        body.extend_from_slice(&self.result_code.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < CHANGE_NAME_RESPONSE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: CHANGE_NAME_RESPONSE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > CHANGE_NAME_RESPONSE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - CHANGE_NAME_RESPONSE_BODY_LEN,
            ));
        }
        Ok(Self {
            result_code: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
        })
    }
}

/// `CheckName` 5032 request: look up `name` before renaming.
///
/// Body is one null-terminated nickname. The send case (VA
/// `0x45c3af`, file `0x5c3af`) writes the header then appends the
/// rename buffer (object `+0x288`) via the string writer
/// (`0x555e30`); the call site (VA `0x45c200`, file `0x5c200`)
/// passes ignored zeros, like the sell/name-change call sites —
/// the value comes from UI state. A length gate precedes it: when
/// the buffer's `strlen` already matches the expected count the
/// client shows a notice instead of sending (VA `0x45c1cb`, file
/// `0x5c1cb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckName {
    /// Nickname to check.
    pub name: String,
}

impl WireMessage for CheckName {
    const COMMAND: u16 = 5032;

    fn encode_body(&self) -> Vec<u8> {
        encode_null_terminated(&self.name, MAX_STRING_LEN)
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (name, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
        if used != body.len() {
            return Err(Error::TrailingBytes(body.len() - used));
        }
        Ok(Self { name })
    }
}

/// `CheckName` 5033 response: nickname-availability answer.
///
/// Body is one `i32` verdict plus a null-terminated string. The
/// S→C handler (VA `0x5639a0`, file `0x1639a0`; index 30 of the
/// `0x138b`-based receive switch) reads the word, then the string
/// via the string reader (`0x555ea0` = `strlen + 1` bytes), and
/// merges the word with `0xff000000`. The string's role
/// (checked-name echo vs notice) stays OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckNameResponse {
    /// Availability verdict (merged with `0xff000000` in-exe).
    pub result_code: i32,
    /// Trailing string (null-terminated on the wire).
    pub name: String,
}

impl WireMessage for CheckNameResponse {
    const COMMAND: u16 = 5033;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4);
        body.extend_from_slice(&self.result_code.to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.name, MAX_STRING_LEN));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        let result_code = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let (name, used) = decode_null_terminated(&body[4..], MAX_STRING_LEN)?;
        if 4 + used != body.len() {
            return Err(Error::TrailingBytes(body.len() - 4 - used));
        }
        Ok(Self { result_code, name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::decode_frame;
    use crate::message::SyncCashPoint;

    #[test]
    fn equip_item_encodes_slot_then_item() {
        let request = EquipItem {
            slot: 3,
            item_id: 5,
        };
        assert_eq!(
            request.encode(),
            vec![0x0C, 0x00, 0x8C, 0x13, 0x03, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn equip_item_round_trips_through_frame() {
        let request = EquipItem {
            slot: 11,
            item_id: 23001,
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5004);
        assert_eq!(EquipItem::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn equip_item_rejects_short_body() {
        assert_eq!(
            EquipItem::decode_body(&[0x01; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn equip_item_rejects_trailing_bytes() {
        let mut body = EquipItem {
            slot: 0,
            item_id: 1,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(EquipItem::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn equip_item_rejects_wrong_command() {
        let frame = decode_frame(
            &EquipItemResponse {
                result: 5,
                tail: None,
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            EquipItem::decode_frame(&frame),
            Err(Error::UnknownCommand(5005))
        );
    }

    #[test]
    fn equip_item_response_success_carries_four_word_tail() {
        let response = EquipItemResponse {
            result: 0,
            tail: Some(EquipTail {
                slot_select: 1,
                reserved_a: 2,
                entry_id: 3,
                reserved_b: 4,
            }),
        };
        assert_eq!(
            response.encode(),
            vec![
                0x18, 0x00, 0x8D, 0x13, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00,
                0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00
            ]
        );
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5005);
        assert_eq!(EquipItemResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn equip_item_response_names_forwarded_tail_words() {
        let response = EquipItemResponse {
            result: 0,
            tail: Some(EquipTail {
                slot_select: 11,
                reserved_a: 22,
                entry_id: 33,
                reserved_b: 44,
            }),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        let back = EquipItemResponse::decode_frame(&frame).unwrap();
        let tail = back.tail.unwrap();
        assert_eq!(tail.slot_select, 11);
        assert_eq!(tail.entry_id, 33);
        assert_eq!(tail.reserved_a, 22);
        assert_eq!(tail.reserved_b, 44);
    }

    #[test]
    fn equip_item_response_error_decodes_result_only() {
        let response = EquipItemResponse::decode_body(&[0x01, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(response.result, 1);
        assert_eq!(response.tail, None);
    }

    #[test]
    fn equip_item_response_rejects_tail_on_error() {
        let body = vec![0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00];
        assert_eq!(
            EquipItemResponse::decode_body(&body),
            Err(Error::TrailingBytes(4))
        );
    }

    #[test]
    fn equip_item_response_rejects_trailing_success() {
        let mut body = EquipItemResponse {
            result: 0,
            tail: Some(EquipTail {
                slot_select: 1,
                reserved_a: 2,
                entry_id: 3,
                reserved_b: 4,
            }),
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            EquipItemResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn equip_item_response_rejects_wrong_command() {
        let frame = decode_frame(
            &EquipItem {
                slot: 0,
                item_id: 1,
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            EquipItemResponse::decode_frame(&frame),
            Err(Error::UnknownCommand(5004))
        );
    }

    #[test]
    fn equip_item_response_rejects_short_success() {
        assert_eq!(
            EquipItemResponse::decode_body(&[0x00, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
        assert_eq!(
            EquipItemResponse::decode_body(&[0x00, 0x00, 0x00, 0x00, 0x01]),
            Err(Error::TruncatedMessage { need: 20, have: 5 })
        );
    }

    fn purchase_prefix() -> SyncItemPurchasePrefix {
        SyncItemPurchasePrefix {
            gem: 5,
            point: 0,
            o2_cash: 10,
            cash_point: 6,
        }
    }

    #[test]
    fn sync_item_purchase_prefix_decodes_first_sixteen_bytes() {
        let mut body = Vec::new();
        purchase_prefix().encode_inner(&mut body);
        body.push(0xFF);
        let (prefix, rest) = SyncItemPurchasePrefix::decode_prefix(&body).unwrap();
        assert_eq!(prefix, purchase_prefix());
        assert_eq!(rest, &[0xFF]);
    }

    #[test]
    fn sync_item_purchase_prefix_names_wallet_roles() {
        let mut body = Vec::new();
        purchase_prefix().encode_inner(&mut body);
        let (prefix, rest) = SyncItemPurchasePrefix::decode_prefix(&body).unwrap();
        assert!(rest.is_empty());
        assert_eq!(prefix.gem, 5);
        assert_eq!(prefix.point, 0);
        assert_eq!(prefix.o2_cash, 10);
        assert_eq!(prefix.cash_point, 6);
    }

    #[test]
    fn sync_item_purchase_prefix_rejects_short_prefix() {
        assert_eq!(
            SyncItemPurchasePrefix::decode_prefix(&[0x01; 15]),
            Err(Error::TruncatedMessage { need: 16, have: 15 })
        );
    }

    #[test]
    fn sync_item_purchase_prefix_encodes_named_wallets() {
        let mut body = Vec::new();
        SyncItemPurchasePrefix {
            gem: 1,
            point: 2,
            o2_cash: 3,
            cash_point: 4,
        }
        .encode_inner(&mut body);
        assert_eq!(body, vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
    }

    #[test]
    fn sync_item_purchase_wallets_decode_music_and_item_cash() {
        let body = vec![11, 0, 0, 0, 12, 0, 0, 0, 0xFF];
        let (wallets, rest) = SyncItemPurchaseWallets::decode_prefix(&body).unwrap();
        assert_eq!(wallets.music_cash, 11);
        assert_eq!(wallets.item_cash, 12);
        assert_eq!(rest, &[0xFF]);
    }

    #[test]
    fn sync_item_purchase_wallets_round_trip() {
        let wallets = SyncItemPurchaseWallets {
            music_cash: -2,
            item_cash: 7,
        };
        let mut body = Vec::new();
        wallets.encode_inner(&mut body);
        let (back, rest) = SyncItemPurchaseWallets::decode_prefix(&body).unwrap();
        assert!(rest.is_empty());
        assert_eq!(back, wallets);
    }

    #[test]
    fn sync_item_purchase_wallets_reject_short_body() {
        assert_eq!(
            SyncItemPurchaseWallets::decode_prefix(&[0x01; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn sync_item_purchase_tail_decodes_free_pass_days() {
        let body = vec![5, 0, 0, 0, 0xFF];
        let (tail, rest) = SyncItemPurchaseTail::decode_prefix(&body).unwrap();
        assert_eq!(tail.free_pass_days, 5);
        assert_eq!(rest, &[0xFF]);
    }

    #[test]
    fn sync_item_purchase_tail_round_trips_zero_refresh() {
        let tail = SyncItemPurchaseTail { free_pass_days: 0 };
        let mut body = Vec::new();
        tail.encode_inner(&mut body);
        assert_eq!(body, vec![0, 0, 0, 0]);
        let (back, rest) = SyncItemPurchaseTail::decode_prefix(&body).unwrap();
        assert!(rest.is_empty());
        assert_eq!(back, tail);
    }

    #[test]
    fn sync_item_purchase_tail_rejects_short_body() {
        assert_eq!(
            SyncItemPurchaseTail::decode_prefix(&[0x01; 3]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    fn restock_pair() -> ShopRestockPair {
        ShopRestockPair {
            attributive_item_id: 23001,
            item_count: 3,
        }
    }

    #[test]
    fn shop_restock_pair_decodes_attributive_item_and_count() {
        let body = vec![0xD9, 0x59, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00];
        let (pair, rest) = ShopRestockPair::decode_prefix(&body).unwrap();
        assert!(rest.is_empty());
        assert_eq!(pair.attributive_item_id, 23001);
        assert_eq!(pair.item_count, 3);
    }

    #[test]
    fn shop_restock_pair_round_trips_pair_list() {
        let mut body = Vec::new();
        restock_pair().encode_inner(&mut body);
        ShopRestockPair {
            attributive_item_id: -1,
            item_count: 9,
        }
        .encode_inner(&mut body);
        let (first, rest) = ShopRestockPair::decode_prefix(&body).unwrap();
        assert_eq!(first, restock_pair());
        let (second, rest) = ShopRestockPair::decode_prefix(rest).unwrap();
        assert!(rest.is_empty());
        assert_eq!(second.item_count, 9);
    }

    #[test]
    fn shop_restock_pair_rejects_short_body() {
        assert_eq!(
            ShopRestockPair::decode_prefix(&[0x01; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn sync_item_purchase_encodes_bare_frame() {
        assert_eq!(SyncItemPurchase.encode(), vec![0x04, 0x00, 0x88, 0x13]);
    }

    #[test]
    fn sync_item_purchase_round_trips_through_frame() {
        let frame = decode_frame(&SyncItemPurchase.encode()).unwrap();
        assert_eq!(frame.id, 5000);
        assert_eq!(SyncItemPurchase::decode_frame(&frame), Ok(SyncItemPurchase));
    }

    #[test]
    fn sync_item_purchase_rejects_trailing_bytes() {
        let frame = decode_frame(&[0x05, 0x00, 0x88, 0x13, 0x00]).unwrap();
        assert_eq!(
            SyncItemPurchase::decode_frame(&frame),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn sync_item_purchase_rejects_wrong_command() {
        let frame = decode_frame(&SyncCashPoint.encode()).unwrap();
        assert_eq!(
            SyncItemPurchase::decode_frame(&frame),
            Err(Error::UnknownCommand(5028))
        );
    }

    #[test]
    fn sync_music_purchase_encodes_bare_frame() {
        assert_eq!(SyncMusicPurchase.encode(), vec![0x04, 0x00, 0x92, 0x13]);
    }

    #[test]
    fn sync_music_purchase_round_trips_through_frame() {
        let frame = decode_frame(&SyncMusicPurchase.encode()).unwrap();
        assert_eq!(frame.id, 5010);
        assert_eq!(
            SyncMusicPurchase::decode_frame(&frame),
            Ok(SyncMusicPurchase)
        );
    }

    #[test]
    fn sync_music_purchase_rejects_trailing_bytes() {
        let frame = decode_frame(&[0x05, 0x00, 0x92, 0x13, 0x00]).unwrap();
        assert_eq!(
            SyncMusicPurchase::decode_frame(&frame),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn sync_music_purchase_rejects_wrong_command() {
        let frame = decode_frame(&SyncItemPurchase.encode()).unwrap();
        assert_eq!(
            SyncMusicPurchase::decode_frame(&frame),
            Err(Error::UnknownCommand(5000))
        );
    }

    #[test]
    fn sell_item_encodes_two_words() {
        let request = SellItem {
            selection: 3,
            entry: 23001,
        };
        assert_eq!(
            request.encode(),
            vec![0x0C, 0x00, 0x8A, 0x13, 0x03, 0x00, 0x00, 0x00, 0xD9, 0x59, 0x00, 0x00]
        );
    }

    #[test]
    fn sell_item_round_trips_through_frame() {
        let request = SellItem {
            selection: 0,
            entry: -1,
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5002);
        assert_eq!(SellItem::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn sell_item_rejects_short_body() {
        assert_eq!(
            SellItem::decode_body(&[0x01; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn sell_item_rejects_trailing_bytes() {
        let mut body = SellItem {
            selection: 0,
            entry: 1,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(SellItem::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn sell_item_rejects_wrong_command() {
        let frame = decode_frame(
            &SellItemResponse {
                result: 0,
                gem: None,
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            SellItem::decode_frame(&frame),
            Err(Error::UnknownCommand(5003))
        );
    }

    #[test]
    fn sell_item_response_decodes_result_only() {
        let response = SellItemResponse::decode_body(&[0x05, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(response.result, 5);
        assert_eq!(response.gem, None);
    }

    #[test]
    fn sell_item_response_success_carries_gem_word() {
        let response = SellItemResponse {
            result: 0,
            gem: Some(7),
        };
        assert_eq!(
            response.encode(),
            vec![0x0C, 0x00, 0x8B, 0x13, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00]
        );
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5003);
        assert_eq!(SellItemResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn sell_item_response_rejects_gem_word_on_error() {
        let body = vec![0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00];
        assert_eq!(
            SellItemResponse::decode_body(&body),
            Err(Error::TrailingBytes(4))
        );
    }

    #[test]
    fn sell_item_response_rejects_short_success() {
        assert_eq!(
            SellItemResponse::decode_body(&[0x00, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
        assert_eq!(
            SellItemResponse::decode_body(&[0x00, 0x00, 0x00, 0x00, 0x01]),
            Err(Error::TruncatedMessage { need: 8, have: 5 })
        );
    }

    #[test]
    fn change_name_encodes_string_plus_two_words() {
        let request = ChangeName {
            name: "ab".to_owned(),
            selection: 1,
            entry: 2,
        };
        assert_eq!(
            request.encode(),
            vec![
                0x0F, 0x00, 0xA6, 0x13, 0x61, 0x62, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00
            ]
        );
    }

    #[test]
    fn change_name_round_trips_through_frame() {
        let request = ChangeName {
            name: "newname".to_owned(),
            selection: -1,
            entry: 0,
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5030);
        assert_eq!(ChangeName::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn change_name_rejects_short_tail() {
        let mut body = vec![0x61, 0x00];
        body.extend_from_slice(&[0x01; 7]);
        assert_eq!(
            ChangeName::decode_body(&body),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn change_name_rejects_trailing_bytes() {
        let mut body = ChangeName {
            name: "a".to_owned(),
            selection: 0,
            entry: 0,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(ChangeName::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn change_name_rejects_wrong_command() {
        let frame = decode_frame(&ChangeNameResponse { result_code: 0 }.encode()).unwrap();
        assert_eq!(
            ChangeName::decode_frame(&frame),
            Err(Error::UnknownCommand(5031))
        );
    }

    #[test]
    fn change_name_response_encodes_single_word() {
        let response = ChangeNameResponse { result_code: 0 };
        assert_eq!(
            response.encode(),
            vec![0x08, 0x00, 0xA7, 0x13, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn change_name_response_round_trips_through_frame() {
        let response = ChangeNameResponse { result_code: -96 };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5031);
        assert_eq!(ChangeNameResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn change_name_response_rejects_short_body() {
        assert_eq!(
            ChangeNameResponse::decode_body(&[0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }

    #[test]
    fn change_name_response_rejects_trailing_bytes() {
        let mut body = ChangeNameResponse { result_code: 0 }.encode_body();
        body.push(0xFF);
        assert_eq!(
            ChangeNameResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn check_name_response_encodes_word_plus_string() {
        let response = CheckNameResponse {
            result_code: 0,
            name: "ab".to_owned(),
        };
        assert_eq!(
            response.encode(),
            vec![0x0B, 0x00, 0xA9, 0x13, 0x00, 0x00, 0x00, 0x00, 0x61, 0x62, 0x00]
        );
    }

    #[test]
    fn check_name_response_round_trips_through_frame() {
        let response = CheckNameResponse {
            result_code: -1,
            name: "taken".to_owned(),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5033);
        assert_eq!(CheckNameResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn check_name_response_rejects_short_body() {
        assert_eq!(
            CheckNameResponse::decode_body(&[0x00, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn check_name_response_rejects_trailing_bytes() {
        let mut body = CheckNameResponse {
            result_code: 0,
            name: "a".to_owned(),
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            CheckNameResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn check_name_encodes_terminated_name() {
        let request = CheckName {
            name: "ab".to_owned(),
        };
        assert_eq!(
            request.encode(),
            vec![0x07, 0x00, 0xA8, 0x13, 0x61, 0x62, 0x00]
        );
    }

    #[test]
    fn check_name_round_trips_through_frame() {
        let request = CheckName {
            name: "newname".to_owned(),
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5032);
        assert_eq!(CheckName::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn check_name_rejects_trailing_bytes() {
        let mut body = CheckName {
            name: "a".to_owned(),
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(CheckName::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn check_name_rejects_wrong_command() {
        let frame = decode_frame(
            &CheckNameResponse {
                result_code: 0,
                name: "a".to_owned(),
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            CheckName::decode_frame(&frame),
            Err(Error::UnknownCommand(5033))
        );
    }

    fn purchased_two_songs() -> SyncMusicPurchaseResponse {
        SyncMusicPurchaseResponse {
            gem: 1000,
            point: 250,
            o2_cash: 5000,
            music_cash: 120,
            item_cash: 340,
            cash_point: 56,
            music_list: vec![
                MusicEntry {
                    music_id: 101,
                    day: 14,
                    month: 3,
                    year: 25,
                    hour: 18,
                    minute: 45,
                },
                MusicEntry {
                    music_id: 202,
                    day: 1,
                    month: 12,
                    year: 26,
                    hour: 9,
                    minute: 5,
                },
            ],
            // Item-gift count 1 + opaque payload, music-gift count 0;
            // inner element shape stays OPEN.
            gift_boxes_raw: vec![0x01, 0x00, 0x09, 0x07, 0x00, 0x00],
            free_pass_extension_days: 7,
        }
    }

    #[test]
    fn sync_music_purchase_5011_response_round_trips_two_entries_and_gifts() {
        let response = purchased_two_songs();
        let body = response.encode_body();
        assert_eq!(SyncMusicPurchaseResponse::decode_body(&body), Ok(response));
    }

    #[test]
    fn sync_music_purchase_5011_response_decodes_pinned_wire_bytes() {
        let body = vec![
            0xE8, 0x03, 0x00, 0x00, // gem 1000
            0xFA, 0x00, 0x00, 0x00, // point 250
            0x88, 0x13, 0x00, 0x00, // o2_cash 5000
            0x78, 0x00, 0x00, 0x00, // music_cash 120
            0x54, 0x01, 0x00, 0x00, // item_cash 340
            0x38, 0x00, 0x00, 0x00, // cash_point 56
            0x02, 0x00, // music count 2
            0x65, 0x00, 0x0E, 0x03, 0x19, 0x12, 0x2D, // id 101 + expiry
            0xCA, 0x00, 0x01, 0x0C, 0x1A, 0x09, 0x05, // id 202 + expiry
            0x01, 0x00, 0x09, 0x07, 0x00, 0x00, // gift boxes raw
            0x07, 0x00, 0x00, 0x00, // free-pass days 7
        ];
        assert_eq!(
            SyncMusicPurchaseResponse::decode_body(&body),
            Ok(purchased_two_songs())
        );
    }
}
