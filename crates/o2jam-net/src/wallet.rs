//! Wallet messages: cash points, free passes and web-coupon charges.
//!
//! Covers `StartPayment` 5015/5016, `SyncFreePass` 5025/5026 and
//! `SyncCashPoint` 5028/5029. Layouts mirror Memoryer's
//! `Source/Memoryer` field orders (clean-room C#); exe anchors stay on
//! each type, Memoryer-only shapes stay labeled.

use super::error::Error;
use super::frame::Frame;
use super::message::{decode_empty_frame, encode_empty, EmptyMessage, WireMessage};
use super::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};
use crate::macros::wire_enum;

// `SyncFreePass` 5025 likewise sends a bare header (send case VA
// `0x4f400c`, file `0xf400c`; the three virtual-send sites pass a
// placeholder zero: VAs `0x4f4184`/`0x4f41c6`/`0x4f51b4`).
empty_message!(SyncFreePass, 5025);

// `StartPayment` 5015 (web-coupon charge trigger) sends a bare
// header (send case VA `0x491a27`, file `0x91a27`).
empty_message!(StartPayment, 5015);
// `SyncCashPoint` 5028 (post-charge wallet refresh) sends a bare
// header (send case VA `0x491436`, file `0x91436`; virtual-send
// site VA `0x48ee81`, file `0x8ee81`).
empty_message!(SyncCashPoint, 5028);
// `StartPayment` 5016 response: dispatched (`cmpl $0x1398`, VA
// `0x490a09`, file `0x90a09`) to a bare UI notify that reads no
// body fields (VA `0x4910c1`, file `0x910c1`), so the body is
// modeled empty.
empty_message!(StartPaymentResponse, 5016);

// Named row positions keep aliases and page lookups independent of
// future row order.
const PAYMENT_ROW: usize = 0;
const GIFT_ROW: usize = 1;
const CASH_BILL_FIRST_ROW: usize = 2;
const CASH_BILL_LAST_ROW: usize = 6;
const JAMCARD_ROW: usize = 7;
const COUPON_ROW: usize = 8;

// The docs pin one shared range for the five cash-bill URLs, not five
// per-host offsets. Keeping that range named prevents a false
// per-string claim.
const CASH_BILL_URL_RANGE: &str = "0x2466d0-0x2467b8";

/// Publisher named by one pinned cash-bill URL literal. The exe's
/// condition for choosing a publisher at `0x45e010` is not recovered,
/// so the caller supplies the variant; no host selection is invented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingPublisher {
    /// `o2jam.monawa.com`.
    Monawa,
    /// `o2jam.gameangel.com`.
    GameAngel,
    /// `o2jam.gameon.nate.com`.
    GameOnNate,
    /// `o2jam.pdbox.co.kr`.
    Pdbox,
    /// `o2jam.nopp.co.kr`.
    Nopp,
}

impl BillingPublisher {
    /// All five pinned variants, in table order.
    pub const ALL: [Self; 5] = [
        Self::Monawa,
        Self::GameAngel,
        Self::GameOnNate,
        Self::Pdbox,
        Self::Nopp,
    ];

    const fn table_index(self) -> usize {
        match self {
            Self::Monawa => CASH_BILL_FIRST_ROW,
            Self::GameAngel => CASH_BILL_FIRST_ROW + 1,
            Self::GameOnNate => CASH_BILL_FIRST_ROW + 2,
            Self::Pdbox => CASH_BILL_FIRST_ROW + 3,
            Self::Nopp => CASH_BILL_LAST_ROW,
        }
    }

    /// Host literal from the pinned cash-bill table rows.
    #[must_use]
    pub const fn host(self) -> &'static str {
        BILLING_PAGE_TABLE[self.table_index()].host
    }

    /// Short publisher name used by the 7.55 URL inventory.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Monawa => "monawa",
            Self::GameAngel => "gameangel",
            Self::GameOnNate => "gameon.nate",
            Self::Pdbox => "pdbox",
            Self::Nopp => "nopp",
        }
    }

    /// Why the publisher choice is ours, not a recovered exe gate.
    #[must_use]
    pub const fn selector_provenance() -> &'static str {
        "OURS-BY-DESIGN: exe host-selection condition at 0x45e010 is OPEN"
    }
}

/// One billing URL pinned in the 7.55 exe. `file_offset` cites the URL
/// literal; the page-name strings also sit in the range recorded by
/// `docs/net-protocol.md` §2, files `0x2466d0`–`0x24a2d0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BillingPageRow {
    /// Page name the exe uses, without the URL path.
    pub page: &'static str,
    /// Host named by the URL literal.
    pub host: &'static str,
    /// Path named by the URL literal, without the leading slash.
    pub path: &'static str,
    /// Exe file offset, or a named shared range when the docs give no
    /// per-string offset. The five cash-bill rows use one such range.
    pub file_offset: &'static str,
}

/// One page-name-to-host table for the billing request-line builder.
/// Cash-bill hosts are pinned strings, but which one the exe chooses
/// stays OPEN: next evidence step is one local web-stub capture of
/// the host-selection call at `0x45e010` (`docs/web-url-archive-audit.md`
/// lines 48-53).
pub const BILLING_PAGE_TABLE: [BillingPageRow; 9] = [
    // `game1.o2jam.com` / `np/billing/payment/payment_input.aspx`, file `0x24b670`.
    BillingPageRow {
        page: "payment_input.aspx",
        host: "game1.o2jam.com",
        path: "np/billing/payment/payment_input.aspx",
        file_offset: "0x24b670",
    },
    // `game1.o2jam.com` / `np/billing/gift/gift_input.aspx`, file `0x24b638`.
    BillingPageRow {
        page: "gift_input.aspx",
        host: "game1.o2jam.com",
        path: "np/billing/gift/gift_input.aspx",
        file_offset: "0x24b638",
    },
    // Five cash-bill hosts, files `0x2466d0`–`0x2467b8`; the exe's choice is OPEN.
    BillingPageRow {
        page: "cash_bill",
        host: "o2jam.monawa.com",
        path: "client/cash_bill/index.phtml",
        file_offset: CASH_BILL_URL_RANGE,
    },
    BillingPageRow {
        page: "cash_bill",
        host: "o2jam.gameangel.com",
        path: "client/cash_bill/index.phtml",
        file_offset: CASH_BILL_URL_RANGE,
    },
    BillingPageRow {
        page: "cash_bill",
        host: "o2jam.gameon.nate.com",
        path: "client/cash_bill/index.phtml",
        file_offset: CASH_BILL_URL_RANGE,
    },
    BillingPageRow {
        page: "cash_bill",
        host: "o2jam.pdbox.co.kr",
        path: "client/cash_bill/index.phtml",
        file_offset: CASH_BILL_URL_RANGE,
    },
    BillingPageRow {
        page: "cash_bill",
        host: "o2jam.nopp.co.kr",
        path: "client/cash_bill/index.phtml",
        file_offset: CASH_BILL_URL_RANGE,
    },
    // `game1.o2jam.com` / `client/jamcard.asp`, file `0x24b87c`.
    BillingPageRow {
        page: "jamcard.asp",
        host: "game1.o2jam.com",
        path: "client/jamcard.asp",
        file_offset: "0x24b87c",
    },
    // `o2jam.nopp.co.kr` / `client/coupon2/coupon.phtml`, file `0x24b844`.
    BillingPageRow {
        page: "coupon.phtml",
        host: "o2jam.nopp.co.kr",
        path: "client/coupon2/coupon.phtml",
        file_offset: "0x24b844",
    },
];

/// Host of the payment, gift and jamcard URL rows (`0x24b670`,
/// `0x24b638`, `0x24b87c`). Derived from the table, never copied.
pub const BILLING_HOST: &str = BILLING_PAGE_TABLE[PAYMENT_ROW].host;
/// Host of the coupon URL row (`0x24b844`).
pub const COUPON_HOST: &str = BILLING_PAGE_TABLE[COUPON_ROW].host;
/// Path of the cash-bill URL rows (`0x2466d0`–`0x2467b8`).
pub const CASH_BILL_PATH: &str = BILLING_PAGE_TABLE[CASH_BILL_FIRST_ROW].path;
/// The five pinned cash-bill hosts, in table order.
pub const CASH_BILL_HOSTS: [&str; 5] = [
    BILLING_PAGE_TABLE[CASH_BILL_FIRST_ROW].host,
    BILLING_PAGE_TABLE[CASH_BILL_FIRST_ROW + 1].host,
    BILLING_PAGE_TABLE[CASH_BILL_FIRST_ROW + 2].host,
    BILLING_PAGE_TABLE[CASH_BILL_FIRST_ROW + 3].host,
    BILLING_PAGE_TABLE[CASH_BILL_LAST_ROW].host,
];

/// Page name of the `STATE_PAYMENT` row, file `0x24b670`.
pub const PAYMENT_PAGE: &str = BILLING_PAGE_TABLE[PAYMENT_ROW].page;
/// Page name of the `STATE_PAYMENT2` gift row, file `0x24b638`.
pub const GIFT_PAGE: &str = BILLING_PAGE_TABLE[GIFT_ROW].page;
/// URL path of the payment row, file `0x24b670`.
pub const PAYMENT_URL_PATH: &str = BILLING_PAGE_TABLE[PAYMENT_ROW].path;
/// URL path of the gift row, file `0x24b638`.
/// OPEN contradiction: `docs/lifecycle.md` §12 omits the `np/` prefix,
/// while the file-`0x24b638` inventory names it. This constant follows
/// the offset-pinned row; next step is a byte read of that string and
/// its neighbours. `docs/net-protocol.md` §2 records both claims.
pub const GIFT_URL_PATH: &str = BILLING_PAGE_TABLE[GIFT_ROW].path;

/// One billing page whose URL the shell and the reproduction builder emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingWebPage {
    /// `STATE_PAYMENT` entry page.
    Payment,
    /// `STATE_PAYMENT2` gift input page.
    GiftInput,
    /// Cash-bill landing page for an explicitly supplied publisher.
    CashBill(BillingPublisher),
    /// Payment2 jamcard branch.
    JamCard,
    /// Payment2 coupon branch.
    Coupon,
}

impl BillingWebPage {
    const fn table_index(self) -> usize {
        match self {
            Self::Payment => PAYMENT_ROW,
            Self::GiftInput => GIFT_ROW,
            Self::CashBill(publisher) => publisher.table_index(),
            Self::JamCard => JAMCARD_ROW,
            Self::Coupon => COUPON_ROW,
        }
    }

    /// The pinned row this page's host and path come from.
    #[must_use]
    pub const fn row(self) -> &'static BillingPageRow {
        &BILLING_PAGE_TABLE[self.table_index()]
    }

    /// Host named by this page's pinned URL literal.
    #[must_use]
    pub const fn host(self) -> &'static str {
        billing_page(self).0
    }

    /// Path named by this page's pinned URL literal, without the slash.
    #[must_use]
    pub const fn path(self) -> &'static str {
        billing_page(self).1
    }

    /// The URL the client mounts for this page and session id.
    #[must_use]
    pub fn url(self, user_index_id: &str) -> String {
        format!(
            "http://{}/{}?USER_INDEX_ID={}",
            self.host(),
            self.path(),
            user_index_id
        )
    }
}

/// The one host/path pair a billing page reads from the pinned table.
#[must_use]
pub const fn billing_page(page: BillingWebPage) -> (&'static str, &'static str) {
    let row = page.row();
    (row.host, row.path)
}

/// Free-pass kinds (Memoryer `FreePassType`, `uint` bit flags;
/// the server sends one value per sync).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FreePassType {
    /// No pass.
    None = 0,
    /// Every song, no expiry.
    AllMusic = 1,
    /// Display-only slot.
    UnusedFreePlay1 = 2,
    /// New songs while the pass lasts.
    NewMusic = 4,
    /// Acquired songs while the pass lasts.
    FreePlay = 8,
    /// Acquired songs while the pass lasts.
    FreePlayAlt1 = 16,
    /// TOP 100 while the pass lasts.
    Top100 = 32,
    /// Every song while the pass lasts.
    PremiumTime = 64,
    /// Display-only slot.
    UnusedFreePlay2 = 128,
    /// TOP 100 (extended) while the pass lasts.
    Top100Extended = 256,
    /// Acquired songs while the pass lasts.
    FreePlayAlt2 = 512,
    /// Display-only slot.
    UnusedFreePlay3 = 1024,
    /// Acquired songs while the pass lasts.
    FreePlayAlt3 = 1056,
}

impl FreePassType {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// Parse a wire word; unknown or combined flags are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside the known flag values.
    pub fn from_u32(value: u32) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::AllMusic),
            2 => Ok(Self::UnusedFreePlay1),
            4 => Ok(Self::NewMusic),
            8 => Ok(Self::FreePlay),
            16 => Ok(Self::FreePlayAlt1),
            32 => Ok(Self::Top100),
            64 => Ok(Self::PremiumTime),
            128 => Ok(Self::UnusedFreePlay2),
            256 => Ok(Self::Top100Extended),
            512 => Ok(Self::FreePlayAlt2),
            1024 => Ok(Self::UnusedFreePlay3),
            1056 => Ok(Self::FreePlayAlt3),
            _ => Err(Error::InvalidField {
                field: "free-pass type",
                reason: "unknown flag value",
            }),
        }
    }
}

wire_enum! {
    /// Free-pass sync verdicts (Memoryer `SyncResult`, `uint`).
    pub enum SyncFreePassResult: u32, resolve from_u32, encode as_u32, {
        /// Wallet + pass state follow (or nothing on a short body).
        Success = 0,
        /// Pass state unavailable.
        Unknown = 0xFFFF_FFA2,
    }
}

/// Wallet + pass snapshot inside [`SyncFreePassResponse`] 5026
/// (Memoryer's `UserFreePassInfo`; Memoryer-only — the 5026 path at
/// VA `0x4f41f4` (file `0xf41f4`) references neither body slot
/// (`0x10`/`0x14(%ebp)`); it forwards `0xc(%ebp)` to the pass applier
/// at VA `0x4f4300` (file `0xf4300`, reached via VA `0x4f424f`),
/// which itself reads no stack body bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFreePassInfo {
    /// Gem balance.
    pub gem: i32,
    /// Premium currency.
    pub point: i32,
    /// Global premium currency.
    pub o2_cash: i32,
    /// Song-shop cash.
    pub music_cash: i32,
    /// Item-shop cash.
    pub item_cash: i32,
    /// Active pass kind.
    pub free_pass_type: FreePassType,
    /// Cash points.
    pub cash_point: i32,
    /// Formatted pass duration (empty when no pass runs).
    pub duration: String,
    /// Pass seconds remaining (`SubscriptionTimeExpiryCodec`
    /// writes total seconds as `i32`).
    pub expiry_secs: i32,
}

/// Wire words before the duration string: 7 × `i32`/`u32`.
const FREE_PASS_INFO_WORDS_LEN: usize = 28;

impl UserFreePassInfo {
    /// Parse from the front of `body`, returning the value and rest.
    fn decode_prefix(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < FREE_PASS_INFO_WORDS_LEN {
            return Err(Error::TruncatedMessage {
                need: FREE_PASS_INFO_WORDS_LEN,
                have: body.len(),
            });
        }
        let word =
            |at: usize| i32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]]);
        let free_pass_type =
            FreePassType::from_u32(u32::from_le_bytes([body[20], body[21], body[22], body[23]]))?;
        let mut tail = &body[FREE_PASS_INFO_WORDS_LEN..];
        let (duration, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
        tail = &tail[used..];
        if tail.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: tail.len(),
            });
        }
        let expiry_secs = i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        Ok((
            Self {
                gem: word(0),
                point: word(4),
                o2_cash: word(8),
                music_cash: word(12),
                item_cash: word(16),
                free_pass_type,
                cash_point: word(24),
                duration,
                expiry_secs,
            },
            &tail[4..],
        ))
    }

    /// Full body for sending (tests and future use).
    fn encode_inner(&self, body: &mut Vec<u8>) {
        for word in [
            self.gem,
            self.point,
            self.o2_cash,
            self.music_cash,
            self.item_cash,
        ] {
            body.extend_from_slice(&word.to_le_bytes());
        }
        body.extend_from_slice(&self.free_pass_type.as_u32().to_le_bytes());
        body.extend_from_slice(&self.cash_point.to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.duration, MAX_STRING_LEN));
        body.extend_from_slice(&self.expiry_secs.to_le_bytes());
    }
}

/// `SyncFreePass` 5026 response: `u32` result plus the optional
/// wallet snapshot (absent when the body ends after `result`, like
/// [`crate::message::ChannelLoginResponse`]'s trailing flag).
///
/// Dispatched by `cmpl $0x13a2` (VAs `0x4f40f2`/`0x4f40fe`, files
/// `0xf40f2`/`0xf40fe`); the info tail is Memoryer-only (see
/// [`UserFreePassInfo`]: the handler consumes no tail bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncFreePassResponse {
    /// Raw result code; see [`SyncFreePassResult::from_u32`].
    pub result_code: u32,
    /// Wallet snapshot; `None` on a 4-byte result-only body.
    pub info: Option<UserFreePassInfo>,
}

/// Body length of a result-only [`SyncFreePassResponse`].
const SYNC_FREE_PASS_RESPONSE_MIN_LEN: usize = 4;

impl WireMessage for SyncFreePassResponse {
    const COMMAND: u16 = 5026;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SYNC_FREE_PASS_RESPONSE_MIN_LEN);
        body.extend_from_slice(&self.result_code.to_le_bytes());
        if let Some(info) = &self.info {
            info.encode_inner(&mut body);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SYNC_FREE_PASS_RESPONSE_MIN_LEN {
            return Err(Error::TruncatedMessage {
                need: SYNC_FREE_PASS_RESPONSE_MIN_LEN,
                have: body.len(),
            });
        }
        let tail = &body[SYNC_FREE_PASS_RESPONSE_MIN_LEN..];
        let info = if tail.is_empty() {
            None
        } else {
            let (info, rest) = UserFreePassInfo::decode_prefix(tail)?;
            if !rest.is_empty() {
                return Err(Error::TrailingBytes(rest.len()));
            }
            Some(info)
        };
        Ok(Self {
            result_code: u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            info,
        })
    }
}

/// `SyncCashPoint` 5029 response: two-word wallet refresh.
///
/// Body is two little-endian `i32` words. VERIFIED on 7.55: the
/// `0x138b`-based receive switch routes `0x13a5` (index 26) through
/// byte-table file `0x158ae0` (value 9) to stub VA `0x5588d7` (file
/// `0x1588d7`), which calls the S→C handler (VA `0x563900`, file
/// `0x163900`). It reads both words via the fixed-size reader
/// (`0x555d20`) into `0x768b44` and `0x768b58`, then applies the first
/// via `0x540090` (writes `0x77cd00+0x2c`, the same slot the `SellItem`
/// success word and the 5001 prefix words update) and the second via
/// `0x540d00` (writes `0x77cd00+0x2128`, inside the wallet block copied at
/// VA `0x4e93d1`, file `0xe93d1`). Field names follow Memoryer's
/// `SyncCashPointResponse` order (`gem`, `cash_point`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncCashPointResponse {
    /// Gem balance.
    pub gem: i32,
    /// Cash points.
    pub cash_point: i32,
}

/// Body length of [`SyncCashPointResponse`]: two `i32` words.
const SYNC_CASH_POINT_RESPONSE_BODY_LEN: usize = 8;

impl WireMessage for SyncCashPointResponse {
    const COMMAND: u16 = 5029;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SYNC_CASH_POINT_RESPONSE_BODY_LEN);
        body.extend_from_slice(&self.gem.to_le_bytes());
        body.extend_from_slice(&self.cash_point.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SYNC_CASH_POINT_RESPONSE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SYNC_CASH_POINT_RESPONSE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SYNC_CASH_POINT_RESPONSE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - SYNC_CASH_POINT_RESPONSE_BODY_LEN,
            ));
        }
        Ok(Self {
            gem: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            cash_point: i32::from_le_bytes([body[4], body[5], body[6], body[7]]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::decode_frame;

    #[test]
    fn free_pass_type_names_flag_values() {
        assert_eq!(FreePassType::from_u32(0), Ok(FreePassType::None));
        assert_eq!(FreePassType::from_u32(1), Ok(FreePassType::AllMusic));
        assert_eq!(FreePassType::from_u32(64), Ok(FreePassType::PremiumTime));
        assert_eq!(FreePassType::from_u32(1056), Ok(FreePassType::FreePlayAlt3));
        assert_eq!(
            FreePassType::from_u32(3),
            Err(Error::InvalidField {
                field: "free-pass type",
                reason: "unknown flag value"
            })
        );
        assert_eq!(FreePassType::Top100.as_u32(), 32);
    }

    #[test]
    fn sync_free_pass_result_names_codes() {
        assert_eq!(
            SyncFreePassResult::from_u32(0),
            Some(SyncFreePassResult::Success)
        );
        assert_eq!(
            SyncFreePassResult::from_u32(0xFFFF_FFA2),
            Some(SyncFreePassResult::Unknown)
        );
        assert_eq!(SyncFreePassResult::from_u32(7), None);
    }

    #[test]
    fn sync_free_pass_result_codes_round_trip() {
        for result in [SyncFreePassResult::Success, SyncFreePassResult::Unknown] {
            assert_eq!(SyncFreePassResult::from_u32(result.as_u32()), Some(result));
        }
        assert_eq!(SyncFreePassResult::from_u32(1), None);
    }

    #[test]
    fn sync_free_pass_response_decodes_result_only() {
        let response = SyncFreePassResponse::decode_body(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(response.result_code, 0);
        assert_eq!(response.info, None);
        assert_eq!(
            SyncFreePassResult::from_u32(response.result_code),
            Some(SyncFreePassResult::Success)
        );
    }

    #[test]
    fn sync_free_pass_response_encodes_result_only_frame() {
        let response = SyncFreePassResponse {
            result_code: 0,
            info: None,
        };
        assert_eq!(
            response.encode(),
            vec![0x08, 0x00, 0xA2, 0x13, 0x00, 0x00, 0x00, 0x00]
        );
    }

    fn free_pass_info() -> UserFreePassInfo {
        UserFreePassInfo {
            gem: 1,
            point: 2,
            o2_cash: 3,
            music_cash: 4,
            item_cash: 5,
            free_pass_type: FreePassType::PremiumTime,
            cash_point: 6,
            duration: String::new(),
            expiry_secs: 3600,
        }
    }

    #[test]
    fn sync_free_pass_response_round_trips_full_info() {
        let response = SyncFreePassResponse {
            result_code: 0,
            info: Some(free_pass_info()),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5026);
        assert_eq!(SyncFreePassResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn sync_free_pass_response_rejects_short_info() {
        let body = vec![0x00, 0x00, 0x00, 0x00, 0x01];
        assert_eq!(
            SyncFreePassResponse::decode_body(&body),
            Err(Error::TruncatedMessage { need: 28, have: 1 })
        );
    }

    #[test]
    fn sync_free_pass_response_rejects_trailing_bytes() {
        let mut body = SyncFreePassResponse {
            result_code: 0,
            info: Some(free_pass_info()),
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            SyncFreePassResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn sync_cash_point_response_encodes_two_words() {
        let response = SyncCashPointResponse {
            gem: 5,
            cash_point: 6,
        };
        assert_eq!(
            response.encode(),
            vec![0x0C, 0x00, 0xA5, 0x13, 0x05, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn live_5029_body_decodes_gem_626_and_zero_cash_point() {
        // Live Memoryer body `7202000000000000` from
        // `uid2-server.jsonl` (`96bb62ce…25009b`) and the tapped BACK
        // wire `94b2ea8f…a2508`: the exe paints `G 626 / W 0`.
        let response =
            SyncCashPointResponse::decode_body(&[0x72, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                .unwrap();
        assert_eq!(
            response,
            SyncCashPointResponse {
                gem: 626,
                cash_point: 0,
            }
        );
    }

    #[test]
    fn sync_cash_point_response_round_trips_through_frame() {
        let response = SyncCashPointResponse {
            gem: -1,
            cash_point: 0,
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5029);
        assert_eq!(SyncCashPointResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn sync_cash_point_response_rejects_short_body() {
        assert_eq!(
            SyncCashPointResponse::decode_body(&[0x00; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn sync_cash_point_response_rejects_trailing_bytes() {
        let mut body = SyncCashPointResponse {
            gem: 5,
            cash_point: 6,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            SyncCashPointResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    mod billing_request_line_tests {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        use super::super::{BillingPublisher, BillingWebPage};
        use crate::testing::fake_gateway;

        /// Reproduction-only request line. The exe's header bytes are
        /// not captured, so this stays inside the test module until a
        /// local web-stub capture pins them.
        fn billing_request_line(page: BillingWebPage, user_index_id: &str) -> String {
            let row = page.row();
            format!(
                "GET /{}?USER_INDEX_ID={} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                row.path, user_index_id, row.host
            )
        }

        /// Send one built request line to a scripted loopback gateway and
        /// return its one-line answer.
        fn replay_request_line(request_line: &str) -> String {
            let expected_line = request_line.to_owned();
            let (port, server) = fake_gateway(move |mut stream| {
                let mut received = [0u8; 1024];
                let used = stream.read(&mut received).unwrap();
                assert!(used > 0, "gateway must receive a request line");
                assert_eq!(
                    String::from_utf8(received[..used].to_vec()).unwrap(),
                    expected_line,
                    "gateway must receive the builder's exact request line"
                );
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .unwrap();
            });

            let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
            client.write_all(request_line.as_bytes()).unwrap();
            let mut answer = String::new();
            client.read_to_string(&mut answer).unwrap();
            server.join().unwrap();
            answer
        }

        fn host_and_target(request_line: &str) -> (&str, &str) {
            let mut lines = request_line.lines();
            let target = lines.next().unwrap().split_whitespace().nth(1).unwrap();
            let host = lines.find_map(|line| line.strip_prefix("Host: ")).unwrap();
            (host, target)
        }

        #[test]
        fn billing_request_lines_name_the_pinned_host_page_and_session_join_on_the_scripted_gateway(
        ) {
            // Seam: this client emits no HTTP request line by owner pin
            // (`docs/pins/payment.md`, #427 live half). The built line is
            // the reproduction seam; no live caller exists to trace.
            // Cash-bill's publisher gate at 0x45e010 stays OPEN. Next
            // evidence step: one local web-stub capture of the host choice;
            // this guard asserts only the five hosts the docs pin.
            const PAYMENT_HOST: &str = "game1.o2jam.com";
            const GIFT_HOST: &str = "game1.o2jam.com";
            const CASH_BILL_HOSTS: [&str; 5] = [
                "o2jam.monawa.com",
                "o2jam.gameangel.com",
                "o2jam.gameon.nate.com",
                "o2jam.pdbox.co.kr",
                "o2jam.nopp.co.kr",
            ];

            let cases = [
                (
                    "payment_input.aspx",
                    Some(PAYMENT_HOST),
                    BillingWebPage::Payment,
                ),
                (
                    "gift_input.aspx",
                    Some(GIFT_HOST),
                    BillingWebPage::GiftInput,
                ),
                (
                    "cash_bill",
                    None,
                    BillingWebPage::CashBill(BillingPublisher::Nopp),
                ),
            ];

            for (page, pinned_host, name) in cases {
                let request_line = billing_request_line(name, "7");
                let answer = replay_request_line(&request_line);
                assert_eq!(answer, "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");

                let (host, target) = host_and_target(&request_line);
                assert!(
                    target.contains(page),
                    "{page}: request target {target} must name the pinned page"
                );
                assert!(
                    target.ends_with("?USER_INDEX_ID=7") || target.contains("&USER_INDEX_ID=7"),
                    "{page}: request target {target} must join session 7"
                );
                match pinned_host {
                    Some(expected) => assert_eq!(
                        host, expected,
                        "{page}: gateway Host header expected {expected}, got {host}"
                    ),
                    None => assert!(
                        CASH_BILL_HOSTS.contains(&host),
                        "{page}: gateway Host header {host} must be one of the five pinned hosts {CASH_BILL_HOSTS:?}"
                    ),
                }
            }
        }
    }
}
