//! `GetCharacterInfo` 2001 response (Memoryer `CharacterInfoResponse`).
//!
//! The 2000 request has no body (its handler takes no parameters), so
//! it travels as [`crate::GetCharacterInfo`]. The response is the last
//! Phase-1 layout: account stats, a 17-slot equipment table (one `i32`
//! item id per `Encore.Metadata.Items.ItemType`, `Instrument` 0 …
//! `Costume` 16), i32-prefixed inventory, i16-prefixed gift boxes,
//! and trailing cash/penalty fields.

use super::error::Error;
use super::frame::encode_frame;
use super::message::{Gender, WireMessage};
use super::reader::Reader;
use super::strings::{encode_null_terminated, MAX_STRING_LEN};

/// Equipment slots in wire order (`ItemType` 0–16).
pub const EQUIPMENT_SLOTS: usize = 17;

/// One gifted item (`GiftItemInfo` submessage).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GiftItemInfo {
    /// Gift id.
    pub gift_id: i32,
    /// Item id.
    pub item_id: i32,
    /// Gifter nickname.
    pub sender: String,
}

/// One gifted song (`GiftMusicInfo` submessage).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GiftMusicInfo {
    /// Gift id.
    pub gift_id: i32,
    /// Song id.
    pub music_id: i32,
    /// Gifter nickname.
    pub sender: String,
}

/// One consumable stack (`AttributiveItemInfo` submessage).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttributiveItemInfo {
    /// Item id.
    pub item_id: i32,
    /// Held count.
    pub item_count: i32,
}

/// Full `GetCharacterInfo` 2001 response.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterInfo {
    /// Account flagged invalid.
    pub invalid: bool,
    /// Display name.
    pub nickname: String,
    /// Account gender (one byte: 0/1/2).
    pub gender: Gender,
    /// Gem balance.
    pub gem: i32,
    /// Premium currency (distribution `Point`).
    pub point: i32,
    /// Global premium currency.
    pub o2_cash: i32,
    /// Level.
    pub level: i32,
    /// Battles fought.
    pub battles: i32,
    /// Battles won.
    pub wins: i32,
    /// Battles lost.
    pub losses: i32,
    /// Experience points.
    pub experience: i32,
    /// Bonus points.
    pub bonus_point: i32,
    /// Administrator account.
    pub is_administrator: bool,
    /// Equipped item ids by `ItemType` order.
    pub equipment: [i32; EQUIPMENT_SLOTS],
    /// Owned item ids.
    pub inventory: Vec<i32>,
    /// Unread gift messages.
    pub unread_gifts: i32,
    /// Gifted items waiting.
    pub item_gifts: Vec<GiftItemInfo>,
    /// Gifted songs waiting.
    pub music_gifts: Vec<GiftMusicInfo>,
    /// Song-shop cash.
    pub music_cash: i32,
    /// Item-shop cash.
    pub item_cash: i32,
    /// Cash points.
    pub cash_point: i32,
    /// Consumable stacks.
    pub attributive_items: Vec<AttributiveItemInfo>,
    /// Penalty count.
    pub penalty_count: i16,
    /// Penalty level.
    pub penalty_level: i16,
}

fn read_gift_item(reader: &mut Reader<'_>) -> Result<GiftItemInfo, Error> {
    Ok(GiftItemInfo {
        gift_id: reader.read_i32()?,
        item_id: reader.read_i32()?,
        sender: reader.read_string()?,
    })
}

fn read_gift_music(reader: &mut Reader<'_>) -> Result<GiftMusicInfo, Error> {
    Ok(GiftMusicInfo {
        gift_id: reader.read_i32()?,
        music_id: reader.read_i32()?,
        sender: reader.read_string()?,
    })
}

fn read_attributive(reader: &mut Reader<'_>) -> Result<AttributiveItemInfo, Error> {
    Ok(AttributiveItemInfo {
        item_id: reader.read_i32()?,
        item_count: reader.read_i32()?,
    })
}

fn push_i32(body: &mut Vec<u8>, value: i32) {
    body.extend_from_slice(&value.to_le_bytes());
}

fn push_i16(body: &mut Vec<u8>, value: i16) {
    body.extend_from_slice(&value.to_le_bytes());
}

fn push_gift_item(body: &mut Vec<u8>, gift: &GiftItemInfo) {
    push_i32(body, gift.gift_id);
    push_i32(body, gift.item_id);
    body.extend_from_slice(&encode_null_terminated(&gift.sender, MAX_STRING_LEN));
}

fn push_gift_music(body: &mut Vec<u8>, gift: &GiftMusicInfo) {
    push_i32(body, gift.gift_id);
    push_i32(body, gift.music_id);
    body.extend_from_slice(&encode_null_terminated(&gift.sender, MAX_STRING_LEN));
}

impl WireMessage for CharacterInfo {
    const COMMAND: u16 = 2001;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        push_i32(&mut body, i32::from(self.invalid));
        body.extend_from_slice(&encode_null_terminated(&self.nickname, MAX_STRING_LEN));
        body.push(self.gender.as_byte());
        for stat in [
            self.gem,
            self.point,
            self.o2_cash,
            self.level,
            self.battles,
            self.wins,
            self.losses,
            self.experience,
            self.bonus_point,
        ] {
            push_i32(&mut body, stat);
        }
        body.push(u8::from(self.is_administrator));
        for slot in self.equipment {
            push_i32(&mut body, slot);
        }
        push_i32(&mut body, self.inventory.len() as i32);
        for item in &self.inventory {
            push_i32(&mut body, *item);
        }
        push_i32(&mut body, self.unread_gifts);
        push_i16(&mut body, self.item_gifts.len() as i16);
        for gift in &self.item_gifts {
            push_gift_item(&mut body, gift);
        }
        push_i16(&mut body, self.music_gifts.len() as i16);
        for gift in &self.music_gifts {
            push_gift_music(&mut body, gift);
        }
        for cash in [self.music_cash, self.item_cash, self.cash_point] {
            push_i32(&mut body, cash);
        }
        push_i32(&mut body, self.attributive_items.len() as i32);
        for stack in &self.attributive_items {
            push_i32(&mut body, stack.item_id);
            push_i32(&mut body, stack.item_count);
        }
        push_i16(&mut body, self.penalty_count);
        push_i16(&mut body, self.penalty_level);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let invalid = reader.read_i32()? != 0;
        let nickname = reader.read_string()?;
        let gender = Gender::from_byte(reader.read_u8()?).ok_or(Error::InvalidField {
            field: "character gender",
            reason: "not 0, 1, or 2",
        })?;
        let mut stats = [0i32; 9];
        for stat in &mut stats {
            *stat = reader.read_i32()?;
        }
        let is_administrator = reader.read_flag("administrator flag")?;
        let mut equipment = [0i32; EQUIPMENT_SLOTS];
        for slot in &mut equipment {
            *slot = reader.read_i32()?;
        }
        let mut inventory = Vec::new();
        for _ in 0..reader.read_count("inventory", true)? {
            inventory.push(reader.read_i32()?);
        }
        let unread_gifts = reader.read_i32()?;
        let mut item_gifts = Vec::new();
        for _ in 0..reader.read_count("item gifts", false)? {
            item_gifts.push(read_gift_item(&mut reader)?);
        }
        let mut music_gifts = Vec::new();
        for _ in 0..reader.read_count("music gifts", false)? {
            music_gifts.push(read_gift_music(&mut reader)?);
        }
        let music_cash = reader.read_i32()?;
        let item_cash = reader.read_i32()?;
        let cash_point = reader.read_i32()?;
        let mut attributive_items = Vec::new();
        for _ in 0..reader.read_count("attributive items", true)? {
            attributive_items.push(read_attributive(&mut reader)?);
        }
        let penalty_count = reader.read_i16()?;
        let penalty_level = reader.read_i16()?;
        reader.finish()?;
        Ok(Self {
            invalid,
            nickname,
            gender,
            gem: stats[0],
            point: stats[1],
            o2_cash: stats[2],
            level: stats[3],
            battles: stats[4],
            wins: stats[5],
            losses: stats[6],
            experience: stats[7],
            bonus_point: stats[8],
            is_administrator,
            equipment,
            inventory,
            unread_gifts,
            item_gifts,
            music_gifts,
            music_cash,
            item_cash,
            cash_point,
            attributive_items,
            penalty_count,
            penalty_level,
        })
    }
}

/// Full frame bytes ready to send (test fake servers use this).
#[must_use]
pub fn encode_frame_for(info: &CharacterInfo) -> Vec<u8> {
    encode_frame(CharacterInfo::COMMAND, &info.encode_body())
}

#[cfg(test)]
mod character_info_tests {
    use super::*;
    use crate::frame::{decode_frame, Frame};

    fn minimal_body() -> Vec<u8> {
        let mut body = vec![0x00, 0x00, 0x00, 0x00];
        body.extend_from_slice(b"ab\0");
        body.push(0x01);
        body.extend_from_slice(&[0x00; 36]);
        body.push(0x00);
        body.extend_from_slice(&[0x00; EQUIPMENT_SLOTS * 4]);
        body.extend_from_slice(&[0x00; 4]);
        body.extend_from_slice(&[0x00; 4]);
        body.extend_from_slice(&[0x00; 2]);
        body.extend_from_slice(&[0x00; 2]);
        body.extend_from_slice(&[0x00; 12]);
        body.extend_from_slice(&[0x00; 4]);
        body.extend_from_slice(&[0x00; 4]);
        body
    }

    #[test]
    fn decodes_minimal_character() {
        let info = CharacterInfo::decode_body(&minimal_body()).unwrap();
        assert!(!info.invalid);
        assert_eq!(info.nickname, "ab");
        assert_eq!(info.gender, Gender::Male);
        assert_eq!(info.level, 0);
        assert!(!info.is_administrator);
        assert_eq!(info.equipment, [0; EQUIPMENT_SLOTS]);
        assert!(info.inventory.is_empty());
        assert!(info.item_gifts.is_empty());
        assert!(info.music_gifts.is_empty());
        assert!(info.attributive_items.is_empty());
        assert_eq!(info.penalty_count, 0);
    }

    #[test]
    fn round_trips_full_character() {
        let mut equipment = [0i32; EQUIPMENT_SLOTS];
        equipment[0] = 101;
        equipment[16] = 117;
        let info = CharacterInfo {
            invalid: false,
            nickname: "tester".to_owned(),
            gender: Gender::Female,
            gem: 5,
            point: 6,
            o2_cash: 7,
            level: 10,
            battles: 3,
            wins: 2,
            losses: 1,
            experience: 900,
            bonus_point: 4,
            is_administrator: true,
            equipment,
            inventory: vec![11, 22],
            unread_gifts: 1,
            item_gifts: vec![GiftItemInfo {
                gift_id: 1,
                item_id: 2,
                sender: "giver".to_owned(),
            }],
            music_gifts: vec![GiftMusicInfo {
                gift_id: 3,
                music_id: 4,
                sender: "dj".to_owned(),
            }],
            music_cash: 8,
            item_cash: 9,
            cash_point: 10,
            attributive_items: vec![AttributiveItemInfo {
                item_id: 12,
                item_count: 13,
            }],
            penalty_count: 0,
            penalty_level: 0,
        };
        let frame = decode_frame(&encode_frame_for(&info)).unwrap();
        assert_eq!(frame.id, 2001);
        assert_eq!(
            Frame {
                id: 2001,
                payload: info.encode_body(),
            },
            frame
        );
        assert_eq!(CharacterInfo::decode_frame(&frame), Ok(info));
    }

    #[test]
    fn rejects_negative_inventory_count() {
        let mut body = minimal_body();
        let at = 4 + 3 + 1 + 36 + 1 + EQUIPMENT_SLOTS * 4;
        body[at..at + 4].copy_from_slice(&(-1i32).to_le_bytes());
        assert_eq!(
            CharacterInfo::decode_body(&body),
            Err(Error::InvalidField {
                field: "inventory",
                reason: "negative count"
            })
        );
    }

    #[test]
    fn rejects_bad_admin_byte() {
        let mut body = minimal_body();
        let at = 4 + 3 + 1 + 36;
        body[at] = 0x02;
        assert_eq!(
            CharacterInfo::decode_body(&body),
            Err(Error::InvalidField {
                field: "administrator flag",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn rejects_unknown_gender_byte() {
        let mut body = minimal_body();
        body[4 + 3] = 0x09;
        assert_eq!(
            CharacterInfo::decode_body(&body),
            Err(Error::InvalidField {
                field: "character gender",
                reason: "not 0, 1, or 2"
            })
        );
    }

    #[test]
    fn rejects_short_body() {
        assert_eq!(
            CharacterInfo::decode_body(&[0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }
}
