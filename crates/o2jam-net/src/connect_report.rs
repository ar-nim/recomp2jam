//! Human-readable connect report: the shape the login flow returns and
//! the text renderer that presents it.
//!
//! Both live here so any consumer (CLI, tests, a future UI) can render
//! the same report without depending on the client binary. The seven
//! section helpers are private; [`format_connect_report`] is the one
//! entry point, and [`print_connect_report`] writes it to stdout.

use crate::{
    play::RoomArenaEcho, wallet::SyncCashPointResponse, AuthResult, ChannelLoginResponse,
    ChannelState, CharacterInfo, GameStatsUpdate, JoinRoomResponse, RoomListEntry,
};
use o2jam_game::{PeerBars, QueueEvent, RankDelivery};

/// What the login flow learned.
#[derive(Debug)]
pub struct ConnectReport {
    /// Authorize result code (always success — else this errors).
    pub auth_code: u32,
    /// Character info, when requested.
    pub character: Option<CharacterInfo>,
    /// Wallet balances, when requested with character info.
    pub wallet: Option<WalletReport>,
    /// Channel-entry `SyncCashPoint` 5029 answer (two words), read
    /// before the user list on the exe's room-stage ladder.
    pub cash_sync: Option<SyncCashPointResponse>,
    /// Worn figure layers stacked from `avatar.opa`, when requested.
    pub avatar_layers: usize,
    /// Channels the gateway listed.
    pub channels: Vec<ChannelState>,
    /// Channel login answer, when a channel was requested.
    pub channel_login: Option<ChannelLoginResponse>,
    /// Join answer, when a room was requested.
    pub join: Option<JoinRoomResponse>,
    /// Room rows, fetched only after a channel login (the room
    /// controller rejects channel outsiders).
    pub rooms: Vec<RoomListEntry>,
    /// Song-flow answer, when requested with a room join.
    pub song: Option<SongFlowReport>,
}

/// What the host song flow learned.
#[derive(Debug)]
pub struct SongFlowReport {
    /// Catalog rows in the `GetMusicList` 4031 answer.
    pub music_entries: usize,
    /// Gateway `FreeMusic` flag from the 4031 answer.
    pub free_music: bool,
    /// Stage word the gateway echoed for our arena select.
    pub arena_echo: RoomArenaEcho,
    /// Peer lifebar echo after our tick (`None` on quiet rooms).
    pub peer_bar: Option<GameStatsUpdate>,
    /// Mirror after applying the echo (untouched on quiet rooms).
    pub peers: PeerBars,
    /// Queue forward from the echo (`None` when quiet/unknown).
    pub peer_queue: Option<QueueEvent>,
    /// Rank deliveries from the echo (empty when quiet).
    pub peer_ranks: Vec<RankDelivery>,
    /// Broadcast ids skipped while awaiting answers (kept, not lost).
    pub skipped: Vec<u16>,
}

/// Wallet balances the login flow learned.
///
/// `gem`/`cash_point` come from the `SyncCashPoint` 5029 answer;
/// `point`/`o2_cash` come from the `SyncItemPurchase` 5001 prefix
/// words (the entry list after the prefix is not decoded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletReport {
    /// Gem balance (5029 first word).
    pub gem: i32,
    /// Cash points (5029 second word).
    pub cash_point: i32,
    /// Distribution points (5001 prefix second word).
    pub point: i32,
    /// Premium cash (5001 prefix third word).
    pub o2_cash: i32,
    /// Broadcast ids skipped while awaiting answers (kept, not lost).
    pub skipped: Vec<u16>,
}

/// Human-readable authorize/channel result code.
#[must_use]
pub fn name_result(code: u32) -> String {
    match AuthResult::from_code(code) {
        Some(named) => format!("{named:?} ({code})"),
        None => format!("unknown code {code}"),
    }
}

/// Render one login-flow report as text (authorize, character, avatar,
/// channels, rooms, login, join, song), one line per section.
///
/// Pure: [`print_connect_report`] writes exactly this string, so the
/// golden test below pins the CLI output.
#[must_use]
pub fn format_connect_report(report: &ConnectReport) -> String {
    let mut text = String::new();
    push_identity_section(&mut text, report);
    push_wallet_section(&mut text, report.wallet.as_ref());
    push_channels_section(&mut text, &report.channels);
    push_rooms_section(&mut text, &report.rooms);
    push_login_section(&mut text, report.channel_login.as_ref());
    push_join_section(&mut text, report.join.as_ref());
    push_song_section(&mut text, report.song.as_ref());
    text
}

/// Authorize, character, and avatar lines (each arm prints only when
/// the login flow actually returned that section).
fn push_identity_section(text: &mut String, report: &ConnectReport) {
    text.push_str(&format!("authorize: {}\n", name_result(report.auth_code)));
    if let Some(hero) = &report.character {
        text.push_str(&format!(
            "character: {} (Lv {}, {:?}, {} XP)\n",
            hero.nickname, hero.level, hero.gender, hero.experience
        ));
    }
    if report.avatar_layers > 0 {
        text.push_str(&format!("avatar: {} layers worn\n", report.avatar_layers));
    }
}

/// Wallet balance line plus the skipped-broadcast list (only when a
/// wallet was requested).
fn push_wallet_section(text: &mut String, wallet: Option<&WalletReport>) {
    if let Some(wallet) = wallet {
        text.push_str(&format!(
            "wallet: gem {} cash_point {} point {} o2_cash {}\n",
            wallet.gem, wallet.cash_point, wallet.point, wallet.o2_cash
        ));
        if !wallet.skipped.is_empty() {
            text.push_str(&format!("skipped broadcasts: {:?}\n", wallet.skipped));
        }
    }
}

/// Channel list line, one row per channel.
fn push_channels_section(text: &mut String, channels: &[ChannelState]) {
    text.push_str(&format!("channels: {}\n", channels.len()));
    for channel in channels {
        text.push_str(&format!(
            "  gateway {} channel {}: {}/{} {}\n",
            channel.gateway_id,
            channel.channel_id,
            channel.population,
            channel.capacity,
            if channel.active { "open" } else { "closed" },
        ));
    }
}

/// Open-room count plus up to 10 room rows (room number 0 means
/// closed, so it does not count).
fn push_rooms_section(text: &mut String, rooms: &[RoomListEntry]) {
    let open_rooms: Vec<_> = rooms.iter().filter(|room| room.number != 0).collect();
    text.push_str(&format!("rooms: {} open\n", open_rooms.len()));
    for room in open_rooms.iter().take(10) {
        text.push_str(&format!(
            "  #{} {:?} {}/{}{}\n",
            room.number,
            room.title,
            room.user_count,
            room.capacity,
            if room.has_password { " locked" } else { "" },
        ));
    }
}

/// Channel-login result line with the NORMAL-mode restriction word
/// (only when a channel login was requested).
fn push_login_section(text: &mut String, login: Option<&ChannelLoginResponse>) {
    if let Some(login) = login {
        let restricted = match login.restricted {
            Some(true) => "restricted",
            Some(false) => "unrestricted",
            None => "restriction unknown",
        };
        text.push_str(&format!(
            "channel: {} ({restricted})\n",
            name_result(login.result_code)
        ));
    }
}

/// Room-join answer line (snapshot seat count when the server echoed
/// the room, else the no-snapshot note).
fn push_join_section(text: &mut String, join: Option<&JoinRoomResponse>) {
    if let Some(join) = join {
        match &join.room {
            Some(room) => text.push_str(&format!(
                "room: {} seats, {} in ({})\n",
                room.slots.len(),
                room.user_count,
                room.title
            )),
            None => text.push_str("room: joined without snapshot\n"),
        }
    }
}

/// Song-flow lines: music list, arena echo, peer tick bar, queue
/// mirror, and skipped broadcasts (only when a song flow ran).
fn push_song_section(text: &mut String, song: Option<&SongFlowReport>) {
    if let Some(song) = song {
        text.push_str(&format!(
            "music: {} entries (free music: {})\n",
            song.music_entries,
            if song.free_music { "yes" } else { "no" },
        ));
        text.push_str(&format!("arena: {}\n", song.arena_echo.arena));
        match &song.peer_bar {
            Some(peer) => text.push_str(&format!(
                "tick: member {} {:?} = {}\n",
                peer.member_id,
                peer.kind(),
                peer.value,
            )),
            None => text.push_str("tick: no peer echo\n"),
        }
        match &song.peer_queue {
            Some(queue) => text.push_str(&format!(
                "mirror: slot {} channel {} queued, {} ranks, life[0]={}\n",
                queue.slot,
                queue.stat_kind,
                song.peer_ranks.len(),
                song.peers.life_levels()[0],
            )),
            None => text.push_str(&format!(
                "mirror: no queue forward ({} ranks)\n",
                song.peer_ranks.len()
            )),
        }
        if !song.skipped.is_empty() {
            text.push_str(&format!("skipped broadcasts: {:?}\n", song.skipped));
        }
    }
}

/// Print one login-flow report (authorize, character, avatar, channels,
/// rooms, login, join). Pure output, infallible.
pub fn print_connect_report(report: &ConnectReport) {
    print!("{}", format_connect_report(report));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connect_report as connect;
    use crate::{wallet::SyncCashPointResponse, Gender, JoinedRoom, RoomSlot};
    use o2jam_game::PeerBars;

    fn room_row(number: i32, title: &str, locked: bool) -> RoomListEntry {
        RoomListEntry {
            number,
            state_code: 1,
            title: title.to_owned(),
            has_password: locked,
            music_id: 0,
            difficulty: 0,
            key_mode: 5,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 3,
            min_level: 1,
            max_level: 99,
            skills: Vec::new(),
            premium: false,
            room_type: 0,
        }
    }

    fn full_report() -> connect::ConnectReport {
        let hero = CharacterInfo {
            nickname: "tester".to_owned(),
            gender: Gender::Male,
            level: 3,
            experience: 250,
            ..Default::default()
        };
        let mut peers = PeerBars::new();
        let queue = peers.apply_stats_update(0, 0, 1000).unwrap();
        let slots: Vec<RoomSlot> = (0..7u8)
            .map(|index| RoomSlot {
                index,
                state_code: 0,
                member: None,
            })
            .collect();
        connect::ConnectReport {
            auth_code: 0,
            character: Some(hero),
            cash_sync: Some(SyncCashPointResponse {
                gem: 5,
                cash_point: 6,
            }),
            wallet: Some(connect::WalletReport {
                gem: 5,
                cash_point: 6,
                point: 7,
                o2_cash: 10,
                skipped: Vec::new(),
            }),
            avatar_layers: 2,
            channels: vec![
                ChannelState {
                    gateway_id: 0,
                    channel_id: 1,
                    capacity: 100,
                    population: 12,
                    active: true,
                },
                ChannelState {
                    gateway_id: 0,
                    channel_id: 2,
                    capacity: 100,
                    population: 100,
                    active: false,
                },
            ],
            channel_login: Some(ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            }),
            join: Some(JoinRoomResponse {
                result_code: 0,
                room: Some(JoinedRoom {
                    member_id: 0,
                    team: 0,
                    title: "Lobby".to_owned(),
                    music_id: 0,
                    arena: 0,
                    key_mode: 5,
                    game_mode: 0,
                    difficulty: 0,
                    speed: 1,
                    has_super_manager: false,
                    user_count: 3,
                    slots,
                    skills: Vec::new(),
                    premium: false,
                    champion: None,
                    champion_streak: None,
                }),
            }),
            rooms: vec![room_row(0, "Empty", false), room_row(7, "Lobby", true)],
            song: Some(connect::SongFlowReport {
                music_entries: 4,
                free_music: true,
                arena_echo: RoomArenaEcho::new(3),
                peer_bar: Some(GameStatsUpdate {
                    member_id: 0,
                    kind_code: 0,
                    value: 1000,
                    ranks: [0; 8],
                }),
                peers,
                peer_queue: Some(queue),
                peer_ranks: vec![
                    RankDelivery {
                        position: 0,
                        rank: 5,
                        slot: Some(0),
                    },
                    RankDelivery {
                        position: 1,
                        rank: 6,
                        slot: None,
                    },
                ],
                skipped: vec![4001],
            }),
        }
    }

    #[test]
    fn full_report_renders_the_golden_text() {
        let expected = concat!(
            "authorize: Success (0)\n",
            "character: tester (Lv 3, Male, 250 XP)\n",
            "avatar: 2 layers worn\n",
            "wallet: gem 5 cash_point 6 point 7 o2_cash 10\n",
            "channels: 2\n",
            "  gateway 0 channel 1: 12/100 open\n",
            "  gateway 0 channel 2: 100/100 closed\n",
            "rooms: 1 open\n",
            "  #7 \"Lobby\" 3/8 locked\n",
            "channel: Success (0) (unrestricted)\n",
            "room: 7 seats, 3 in (Lobby)\n",
            "music: 4 entries (free music: yes)\n",
            "arena: 3\n",
            "tick: member 0 Some(Life) = 1000\n",
            "mirror: slot 0 channel 0 queued, 2 ranks, life[0]=1000\n",
            "skipped broadcasts: [4001]\n",
        );
        assert_eq!(format_connect_report(&full_report()), expected);
    }

    #[test]
    fn minimal_report_renders_the_golden_text() {
        let report = connect::ConnectReport {
            auth_code: 99,
            character: None,
            wallet: None,
            cash_sync: None,
            avatar_layers: 0,
            channels: Vec::new(),
            channel_login: None,
            join: None,
            rooms: Vec::new(),
            song: None,
        };
        let expected = "authorize: unknown code 99\nchannels: 0\nrooms: 0 open\n";
        assert_eq!(format_connect_report(&report), expected);
    }

    #[test]
    fn quiet_room_covers_the_fallback_lines() {
        let report = connect::ConnectReport {
            auth_code: 0,
            character: None,
            wallet: None,
            cash_sync: None,
            avatar_layers: 0,
            channels: Vec::new(),
            channel_login: Some(ChannelLoginResponse {
                result_code: 0,
                restricted: None,
            }),
            join: Some(JoinRoomResponse {
                result_code: 0,
                room: None,
            }),
            rooms: Vec::new(),
            song: Some(connect::SongFlowReport {
                music_entries: 0,
                free_music: false,
                arena_echo: RoomArenaEcho::new(0),
                peer_bar: None,
                peers: PeerBars::new(),
                peer_queue: None,
                peer_ranks: Vec::new(),
                skipped: Vec::new(),
            }),
        };
        let expected = concat!(
            "authorize: Success (0)\n",
            "channels: 0\n",
            "rooms: 0 open\n",
            "channel: Success (0) (restriction unknown)\n",
            "room: joined without snapshot\n",
            "music: 0 entries (free music: no)\n",
            "arena: 0\n",
            "tick: no peer echo\n",
            "mirror: no queue forward (0 ranks)\n",
        );
        assert_eq!(format_connect_report(&report), expected);
    }
}
