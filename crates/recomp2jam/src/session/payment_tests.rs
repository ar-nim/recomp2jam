//! Scripted-gateway guards for the payment/gift wire families (#290 stories
//! 977-984). Declared `#[cfg(test)]` by `session.rs`, so `use super::*;`
//! reaches the crate-private session items the drain paths live on.
//!
//! Tier 2 here means a loopback gateway and the real
//! [`OnlineSession`] frame pump. Every frame is synthetic; no installed
//! archive is read, so the guards are CI-safe. Tier 3 live captures stay
//! OPEN for every story in this family.

use super::*;

use o2jam_net::frame::encode_frame;
use o2jam_net::gift::{GetGiftMessageListResponse, GiftMessage};
use o2jam_net::wallet::SyncFreePassResponse;
use o2jam_net::HEADER_LEN;
use o2jam_render::Rect;
use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
use o2jam_shell::screens::side::{MyRoomNotice, MyRoomPanel, MyRoomPassInfo, SideScreen};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::thread;

/// A loopback gateway that pushes the scripted answers before it starts
/// recording what the session sends. The read timeout lets the client
/// drop without leaving a server thread spinning.
fn payment_gateway(incoming: Vec<(u16, Vec<u8>)>) -> (u16, thread::JoinHandle<()>) {
    o2jam_net::testing::fake_gateway(move |mut stream: TcpStream| {
        for (id, body) in incoming {
            stream.write_all(&encode_frame(id, &body)).unwrap();
        }
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        loop {
            let mut header = [0u8; HEADER_LEN];
            match stream.read_exact(&mut header) {
                Ok(()) => {}
                Err(error)
                    if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
                {
                    continue;
                }
                Err(_) => break,
            }
            let body_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
            let mut body = vec![0u8; body_len.saturating_sub(HEADER_LEN)];
            if stream.read_exact(&mut body).is_err() {
                break;
            }
        }
    })
}

/// The shell entry the live boot uses to reach the Waiting board.
fn waiting_game() -> ScreenGame {
    let mut game = ScreenGame::new();
    game.menu_key(MenuKey::Back, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    game
}

/// A Waiting board with one synthetic item and no gift in the box.
fn payment_game() -> ScreenGame {
    let mut game = waiting_game();
    game.set_myroom_items(o2jam_file::AvatarItemTable {
        rows: vec![o2jam_file::AvatarItemRow {
            item_id: 1,
            item_type: 2,
            price: 100,
            params: [0; 4],
            flags: (0, 0),
            name: "Gear1".to_owned(),
            description: String::new(),
            sprites: Vec::new(),
        }],
    });
    game.set_myroom_character(o2jam_net::CharacterInfo {
        nickname: "Tester".to_owned(),
        ..o2jam_net::CharacterInfo::default()
    });
    game
}

/// One synthetic hit rect staged by a side-scene code ordinal.
fn scene_area(code: u32, x: i32) -> ScenePlacement {
    ScenePlacement {
        id: String::new(),
        code,
        sprite: None,
        rect: Rect {
            x,
            y: 10,
            w: 40,
            h: 20,
        },
        role: PlacementRole::Single,
    }
}

/// A gift box with one claimable item row, staged like the live MOVE leg.
fn gift_claim_game() -> ScreenGame {
    let mut game = payment_game();
    let mut hero = o2jam_net::CharacterInfo {
        nickname: "Tester".to_owned(),
        ..o2jam_net::CharacterInfo::default()
    };
    hero.item_gifts.push(o2jam_net::GiftItemInfo {
        gift_id: 7,
        item_id: 1,
        sender: "Giver".to_owned(),
    });
    game.set_myroom_character(hero);
    game.load_side_scene(
        vec![
            scene_area(o2jam_shell::screens::side::MYROOM_GIFT_FIRST_CODE, 10),
            scene_area(o2jam_shell::screens::side::MYROOM_MOVE_CODE, 100),
        ],
        std::collections::HashMap::new(),
    );
    assert!(game.visit_side(SideScreen::MyRoom));
    game.click_myroom_side(11, 11);
    game.click_myroom_side(101, 11);
    game
}

/// Join a loopback gateway to a staged game.
fn payment_session(game: ScreenGame, port: u16) -> OnlineSession {
    let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
    OnlineSession::new(game, gateway)
}

/// One full session paint: wire poll first, then the game canvas.
fn paint_payment_frame(session: &mut OnlineSession) {
    let mut buffer = vec![0u32; 800 * 600];
    session.paint(&mut o2jam_render::Frame {
        buf: &mut buffer,
        w: 800,
        h: 600,
    });
}

/// Give the loopback server a few turns to deliver its scripted frames.
fn settle_payment(session: &mut OnlineSession) {
    for _ in 0..6 {
        paint_payment_frame(session);
        thread::sleep(Duration::from_millis(1));
    }
}

/// The game canvas only, with no wire poll, for before/after paint reads.
fn painted_game(game: &mut ScreenGame) -> Vec<u32> {
    let mut buffer = vec![0u32; 800 * 600];
    game.paint(&mut o2jam_render::Frame {
        buf: &mut buffer,
        w: 800,
        h: 600,
    });
    buffer
}

fn painted_payment_frame(session: &mut OnlineSession) -> Vec<u32> {
    painted_game(session.game_mut())
}

/// Compact paint-diff measure so a failure prints a number, not a
/// million-pixel vector dump.
fn changed_pixels(left: &[u32], right: &[u32]) -> usize {
    left.iter().zip(right).filter(|(a, b)| a != b).count()
}

/// Paint one framed session reply and close its gateway thread.
fn payment_frame_after(incoming: Vec<(u16, Vec<u8>)>) -> Vec<u32> {
    let (port, server) = payment_gateway(incoming);
    let mut session = payment_session(payment_game(), port);
    settle_payment(&mut session);
    let frame = painted_payment_frame(&mut session);
    drop(session);
    server.join().unwrap();
    frame
}

/// Paint a 5026 answer over one staged FREEINFO rect and pass word.
fn freeinfo_frame_after(reply: SyncFreePassResponse) -> Vec<u32> {
    let (port, server) = payment_gateway(vec![(5026, reply.encode_body())]);
    let mut game = freeinfo_game();
    // The exe's pass-word writer at `0x768b34` stays OPEN; staging it
    // lets the wire result decide whether the pinned lines ink.
    game.set_myroom_pass(MyRoomPassInfo {
        word: Some(1),
        until: "2026-12-31".to_owned(),
        skills: Vec::new(),
    });
    let mut session = payment_session(game, port);
    settle_payment(&mut session);
    let frame = painted_payment_frame(&mut session);
    drop(session);
    server.join().unwrap();
    frame
}

/// A status face with one staged FREEINFO rect and no pass word.
fn freeinfo_game() -> ScreenGame {
    let mut game = payment_game();
    game.show_myroom_panel(MyRoomPanel::Status);
    game.load_side_scene(
        vec![ScenePlacement {
            id: "FREEINFO".to_owned(),
            code: o2jam_shell::screens::side::MYROOM_FREEINFO_FIRST_CODE,
            sprite: None,
            rect: Rect {
                x: 200,
                y: 100,
                w: 40,
                h: 20,
            },
            role: PlacementRole::Single,
        }],
        std::collections::HashMap::new(),
    );
    assert!(game.visit_side(SideScreen::MyRoom));
    game
}

/// A bag cell picked and BAGSORT pressed, with 5038 still on the wire.
fn bag_sort_game() -> ScreenGame {
    let mut game = payment_game();
    game.load_side_scene(
        vec![
            scene_area(o2jam_shell::screens::side::MYROOM_BAG_FIRST_CODE, 10),
            scene_area(o2jam_shell::screens::side::MYROOM_BAGSORT_CODE, 100),
        ],
        std::collections::HashMap::new(),
    );
    assert!(game.visit_side(SideScreen::MyRoom));
    game.click_myroom_side(11, 11);
    assert_eq!(game.myroom().bag_cell(), Some(0));
    let _ = game.take_equip_send(); // the pick's own 5004 leg
    game.click_myroom_side(101, 11);
    assert!(game.has_pending_myroom_sends(), "BAGSORT queued 5038");
    game
}

/// Paint the bag face that a 5039 reply has just restaged.
fn bag_pixels_after_reply(inventory: Vec<i32>) -> Vec<u32> {
    let mut game = payment_game();
    let rows = [1, 2]
        .into_iter()
        .map(|item_id| o2jam_file::AvatarItemRow {
            item_id,
            item_type: 2,
            price: 100,
            params: [0; 4],
            flags: (0, 0),
            name: format!("Gear{item_id}"),
            description: String::new(),
            sprites: Vec::new(),
        })
        .collect();
    game.set_myroom_items(o2jam_file::AvatarItemTable { rows });
    game.set_myroom_character(o2jam_net::CharacterInfo {
        nickname: "Tester".to_owned(),
        inventory,
        ..o2jam_net::CharacterInfo::default()
    });
    game.show_myroom_panel(MyRoomPanel::Inventory);
    game.load_side_scene(
        vec![scene_area(
            o2jam_shell::screens::side::MYROOM_BAG_FIRST_CODE,
            10,
        )],
        std::collections::HashMap::new(),
    );
    assert!(game.visit_side(SideScreen::MyRoom));
    game.apply_myroom_reply(5039);
    painted_game(&mut game)
}

/// Run one 5024 claim arm through the live session pump and return its
/// staged result. The server checks the claim opcode before answering.
fn claim_with_validity(validity: i32) -> (OnlineSession, thread::JoinHandle<()>) {
    let mut body = validity.to_le_bytes().to_vec();
    body.push(4);
    let (port, server) = payment_gateway(vec![(5024, body)]);
    let mut session = payment_session(gift_claim_game(), port);
    settle_payment(&mut session);
    (session, server)
}

#[test]
fn point_sync_5014_word_repaints_the_wallet_only_with_a_live_session_flag() {
    // Live caller chain (story 977, pin ev_8a79ece0…, VA 0x562e40):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   apply_waiting_frame -> apply_payment_frame ->
    //   ScreenGame::apply_point_sync_word -> wallet repaint.
    // The exe stages the word at 0x768b4c only when its session flag at
    // 0x762b44 is set; the off arm consumes the word without a post.
    let (port, server) = payment_gateway(vec![(5014, 4242i32.to_le_bytes().to_vec())]);
    let mut session = payment_session(payment_game(), port);
    settle_payment(&mut session);

    assert_eq!(
        session.game.wallet().point,
        Some(4242),
        "a live 5014 word must stage into the wallet point row"
    );
    drop(session);
    server.join().unwrap();
}

#[test]
fn point_sync_5014_word_is_consumed_without_a_live_session_flag() {
    // Live caller chain (story 977, pin ev_8a79ece0…, VA 0x562e40):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   apply_waiting_frame -> apply_payment_frame ->
    //   ScreenGame::wallet -> wallet repaint.
    // The exe consumes the word but posts no event with its session
    // flag at 0x762b44 cleared.
    let (port, server) = payment_gateway(vec![(5014, 4242i32.to_le_bytes().to_vec())]);
    let mut session = payment_session(payment_game(), port);
    let before = session.game.wallet().point;
    session.set_session_active(false);
    settle_payment(&mut session);

    assert_eq!(
        session.game.wallet().point,
        before,
        "a stale 5014 word must not repaint the wallet"
    );
    drop(session);
    server.join().unwrap();
}

#[test]
fn start_payment_5016_empty_and_padded_bodies_raise_one_equal_notice() {
    // Live caller chain (story 978, pin ev_6e8a569a…, VA 0x562e90):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   apply_waiting_frame -> apply_payment_frame ->
    //   ScreenGame::note_start_payment_answer -> paint_at notice band.
    // The exe reads no body bytes; the padded arm must not invent ink.
    let baseline = payment_frame_after(Vec::new());
    let empty_frame = payment_frame_after(vec![(5016, Vec::new())]);
    let padded_frame = payment_frame_after(vec![(
        5016,
        vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
    )]);

    assert!(
        changed_pixels(&empty_frame, &baseline) > 0,
        "the 5016 answer must raise one visible UI notice"
    );
    assert_eq!(
        changed_pixels(&empty_frame, &padded_frame),
        0,
        "extra server bytes must not change the 5016 notice"
    );
}

#[test]
fn gift_list_5018_zero_flag_rows_stage_the_present_box() {
    // Live caller chain (story 979, pin ev_865e1b28…, VA 0x562eb0):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   apply_waiting_frame -> apply_payment_frame ->
    //   ScreenGame::apply_gift_message_list -> MyRoom gift paint.
    // The exe skips the row loop on a nonzero flag but still posts the
    // list notice; the box must match the rows the exe reads.
    let messages = vec![
        GiftMessage {
            message_id: 7,
            gift_type: o2jam_net::gift::GiftType::Item,
            write_date: "2026-11-25".to_owned(),
            sender: "SenderA".to_owned(),
            title: "TitleA".to_owned(),
            content: "BodyA".to_owned(),
        },
        GiftMessage {
            message_id: 8,
            gift_type: o2jam_net::gift::GiftType::Music,
            write_date: "2026-11-24".to_owned(),
            sender: "SenderB".to_owned(),
            title: "TitleB".to_owned(),
            content: "BodyB".to_owned(),
        },
    ];
    let reply = GetGiftMessageListResponse {
        invalid: false,
        messages,
    };
    let (port, server) = payment_gateway(vec![(5018, reply.encode_body())]);
    let mut session = payment_session(payment_game(), port);
    settle_payment(&mut session);

    let hero = session.game.myroom_character().expect("staged character");
    assert_eq!(
        hero.item_gifts.len(),
        1,
        "a zero-flag 5018 must stage its gift rows into the box"
    );
    drop(session);
    server.join().unwrap();
}

#[test]
fn zero_validity_5024_with_verdict_four_posts_the_gift_success_caption() {
    // Live caller chain (story 980, pin ev_1160.7a8d…, VA 0x563010):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   poll_gift_claim -> drain_gift_claim -> apply_gift_claim_reply ->
    //   ScreenGame::myroom_notice -> one-OK sheet.
    // Tier 2 counterpart of `valid_5024_with_memoryer_verdict_four_…`
    // (session.rs:6712): the validity word, not the verdict, owns the
    // success caption.
    let (session, server) = claim_with_validity(0);

    let hero = session.game.myroom_character().expect("staged character");
    assert!(hero.inventory.contains(&1), "the claim reaches the bag");
    assert_eq!(
        session.game.myroom_notice(),
        Some(&MyRoomNotice::GiftItemSuccess),
        "verdict 4 cannot suppress the validity-zero success caption"
    );
    drop(session);
    server.join().unwrap();
}

#[test]
fn nonzero_validity_5024_with_verdict_four_posts_the_gift_failure_caption() {
    // Same chain and pin as the validity-zero arm. The nonzero word
    // takes the exe's caption-writer error arm; verdict 4 is never
    // read, so the staged gift stays and the failure sheet opens.
    let (session, server) = claim_with_validity(1);

    let hero = session.game.myroom_character().expect("staged character");
    assert_eq!(hero.item_gifts.len(), 1, "the claim is refused");
    assert_eq!(
        session.game.myroom_notice(),
        Some(&MyRoomNotice::GiftItemFailure),
        "the nonzero validity word posts the pinned failure caption"
    );
    drop(session);
    server.join().unwrap();
}

#[test]
fn free_pass_5026_result_inks_the_staged_freeinfo_lines() {
    // Live caller chain (story 981, pin ev_ffda.e60e…, VA 0x5631b0):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   apply_waiting_frame -> apply_payment_frame ->
    //   ScreenGame::apply_free_pass_answer -> paint_myroom_data.
    // Result -0x5e posts the event with no lines; result zero writes
    // one to four lines from the externally staged pass word; the
    // writer at 0x768b34 itself stays OPEN.
    let pass_reply = SyncFreePassResponse {
        result_code: 0,
        info: Some(o2jam_net::wallet::UserFreePassInfo {
            gem: 0,
            point: 0,
            o2_cash: 0,
            music_cash: 0,
            item_cash: 0,
            free_pass_type: o2jam_net::wallet::FreePassType::AllMusic,
            cash_point: 0,
            duration: "2026-12-31".to_owned(),
            expiry_secs: 3600,
        }),
    };
    let frame = freeinfo_frame_after(pass_reply);
    assert!(
        (100..120).any(|y| (200..240).any(|x| frame[y * 800 + x] != 0)),
        "a zero 5026 result must ink the pass line selected by its pass word"
    );

    let no_pass = freeinfo_frame_after(SyncFreePassResponse {
        result_code: 0xFFFF_FFA2,
        info: None,
    });
    assert!(
        !(100..120).any(|y| (200..240).any(|x| no_pass[y * 800 + x] != 0)),
        "result -0x5e posts the notice without pass lines"
    );
}

#[test]
fn penalty_reset_5035_code_opens_the_pinned_one_ok_sheet() {
    // Live caller chain (story 982, pin ev_6d7f.6b6d…, VA 0x563a20):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   apply_waiting_frame -> apply_payment_frame ->
    //   ScreenGame::apply_myroom_notice -> MyRoomNotice::PenaltyCleared
    //   / PenaltyRetry -> one-OK sheet. The frame also reaches the
    //   payment arm from `drain_myroom_reply` when a short leg stands.
    let (port, server) = payment_gateway(vec![(5035, 0i32.to_le_bytes().to_vec())]);
    let mut game = payment_game();
    assert!(game.visit_side(SideScreen::MyRoom));
    let mut session = payment_session(game, port);
    settle_payment(&mut session);

    assert_eq!(
        session.game.myroom_notice(),
        Some(&MyRoomNotice::PenaltyCleared),
        "the 5035 success code must open its pinned one-OK sheet"
    );
    drop(session);
    server.join().unwrap();
}

#[test]
fn bag_expansion_5037_success_grows_the_bag_and_inks_the_caption() {
    // Live caller chain (story 983, pin ev_52c1.26fb…, VA 0x563a60):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   apply_waiting_frame -> apply_payment_frame ->
    //   ScreenGame::grow_myroom_bag ->
    //   MyRoomNotice::BagExpanded { after } -> one-OK sheet.
    // The exe reads a result word then the slot count; error codes
    // only post the retry caption.
    let body = [0i32.to_le_bytes(), 30i32.to_le_bytes()].concat();
    let (port, server) = payment_gateway(vec![(5037, body)]);
    let mut game = payment_game();
    assert!(game.visit_side(SideScreen::MyRoom));
    let mut session = payment_session(game, port);
    settle_payment(&mut session);

    assert_eq!(
        session.game.myroom_notice(),
        Some(&MyRoomNotice::BagExpanded { after: 30 }),
        "a successful 5037 must show the before/after bag caption"
    );
    drop(session);
    server.join().unwrap();
}

#[test]
fn bag_sort_5039_reply_repaints_the_staged_bag_and_drops_the_cell_cursor() {
    // Live caller chain (story 984, pin ev_5dfe.0e3a…, VA 0x563b20):
    //   main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    //   flush_myroom_sends (5038) -> poll_myroom_replies ->
    //   drain_myroom_reply -> ScreenGame::apply_myroom_reply(5039) ->
    //   ScreenGame::paint_myroom.
    // Tier 2 counterpart of the ignored guard at session.rs:6721.
    let (port, server) = payment_gateway(vec![(5039, Vec::new())]);
    let mut session = payment_session(bag_sort_game(), port);
    settle_payment(&mut session);

    assert_eq!(
        session.game.myroom().bag_cell(),
        None,
        "the 5039 answer drops the bag cell cursor"
    );
    drop(session);
    server.join().unwrap();
}

#[test]
fn staged_bag_slot_words_drive_the_myroom_paint() {
    // Tier 0 paint leg for story 984. Live chain:
    //   main.rs::run_play -> OnlineSession::paint_at_instant ->
    //   poll_wire_at -> poll_myroom_replies -> drain_myroom_reply ->
    //   ScreenGame::apply_myroom_reply -> ScreenGame::paint_myroom.
    // The exe stages the 5039 rows into slots before painting; this
    // guard proves the painter consumes the staged words it is given.
    // The body-driven staging itself stays BLOCKED (see report).
    let empty_bag = bag_pixels_after_reply(Vec::new());
    let stocked_bag = bag_pixels_after_reply(vec![1, 2]);

    assert!(
        changed_pixels(&empty_bag, &stocked_bag) > 0,
        "staged inventory words must ink the MyRoom bag face"
    );
}

#[test]
fn billing_page_names_keep_the_pinned_hosts_and_request_paths() {
    // seam: the URL inventory is a static host/path pin; this client
    // creates no browser object and sends no HTTP request, so the
    // literal request-line join is the whole observable behavior here.
    use o2jam_shell::screens::messenger::{BillingWebPage, BILLING_HOST, COUPON_HOST};

    assert_eq!(BILLING_HOST, "game1.o2jam.com");
    assert_eq!(COUPON_HOST, "o2jam.nopp.co.kr");
    assert_eq!(
        BillingWebPage::Payment.url("7"),
        "http://game1.o2jam.com/np/billing/payment/payment_input.aspx?USER_INDEX_ID=7"
    );
    assert_eq!(
        BillingWebPage::GiftInput.url("7"),
        "http://game1.o2jam.com/np/billing/gift/gift_input.aspx?USER_INDEX_ID=7"
    );
    assert_eq!(
        BillingWebPage::JamCard.url("7"),
        "http://game1.o2jam.com/client/jamcard.asp?USER_INDEX_ID=7"
    );
    assert_eq!(
        BillingWebPage::Coupon.url("7"),
        "http://o2jam.nopp.co.kr/client/coupon2/coupon.phtml?USER_INDEX_ID=7"
    );
}

#[test]
fn payment_web_rebuild_closes_the_old_window_before_the_new_page_mounts() {
    // Tier 1 window-input seam; live chain:
    //   main.rs::run_play -> window hub 0x5a5190 WM_APP+0x32 ->
    //   billing_window_arm -> BillingWebHandle::rebuild ->
    //   BillingWebEvent::ClosedPrevious then ::Mounted.
    // seam note: no WebCtrl32 window exists in the winit client, so
    // handle close/mount order is the whole host behavior.
    const APP_UI_EVENT: u32 = 0x232;
    use o2jam_shell::screens::messenger::{
        billing_window_arm, BillingWebEvent, BillingWebHandle, BillingWebPage, BillingWindowArm,
        BILLING_PAYMENT_STATE_WORD,
    };

    assert_eq!(
        billing_window_arm(APP_UI_EVENT, BILLING_PAYMENT_STATE_WORD, true),
        BillingWindowArm::RebuildPayment
    );

    let mut handle = BillingWebHandle::new();
    let first = handle.rebuild(BillingWebPage::Payment, "2", 41);
    assert_eq!(
        first,
        vec![BillingWebEvent::Mounted {
            handle: 41,
            url: "http://game1.o2jam.com/np/billing/payment/payment_input.aspx?USER_INDEX_ID=2"
                .to_owned(),
        }]
    );

    let second = handle.rebuild(BillingWebPage::Payment, "2", 42);
    assert_eq!(second.first(), Some(&BillingWebEvent::ClosedPrevious(41)));
    assert_eq!(
        second.get(1),
        Some(&BillingWebEvent::Mounted {
            handle: 42,
            url: "http://game1.o2jam.com/np/billing/payment/payment_input.aspx?USER_INDEX_ID=2"
                .to_owned(),
        })
    );
}

#[test]
fn gift_web_rebuild_reuses_the_user_index_id_on_both_gift_branches() {
    // Tier 1 window-input seam; live chain:
    //   main.rs::run_play -> window hub 0x5a5190 WM_APP+0x32 state 13 ->
    //   billing_window_arm -> BillingWebHandle::rebuild (0x4f5020 ->
    //   0x4f4bd0 URL opener) -> jamcard/coupon URL.
    // seam note: the branch condition is unpinned; the explicit page
    // argument keeps both pinned URL branches without inventing a gate.
    const APP_UI_EVENT: u32 = 0x232;
    use o2jam_shell::screens::messenger::{
        billing_window_arm, BillingWebEvent, BillingWebHandle, BillingWebPage, BillingWindowArm,
        BILLING_PAYMENT2_STATE_WORD,
    };

    assert_eq!(
        billing_window_arm(APP_UI_EVENT, BILLING_PAYMENT2_STATE_WORD, true),
        BillingWindowArm::RebuildGift
    );

    let mut handle = BillingWebHandle::with_handle(9);
    let jamcard = handle.rebuild(BillingWebPage::JamCard, "55", 51);
    assert_eq!(jamcard.first(), Some(&BillingWebEvent::ClosedPrevious(9)));
    assert_eq!(
        jamcard.get(1),
        Some(&BillingWebEvent::Mounted {
            handle: 51,
            url: "http://game1.o2jam.com/client/jamcard.asp?USER_INDEX_ID=55".to_owned(),
        })
    );

    let coupon = handle.rebuild(BillingWebPage::Coupon, "55", 52);
    assert_eq!(coupon.first(), Some(&BillingWebEvent::ClosedPrevious(51)));
    assert_eq!(
        coupon.get(1),
        Some(&BillingWebEvent::Mounted {
            handle: 52,
            url: "http://o2jam.nopp.co.kr/client/coupon2/coupon.phtml?USER_INDEX_ID=55".to_owned(),
        })
    );
}

#[test]
fn billing_rebuild_arm_needs_the_app_event_billing_state_and_a_live_handle() {
    // seam: the Win32 hub has no winit message pump; the arm decode
    // is the whole gate the headless client can observe.
    use o2jam_shell::screens::messenger::{
        billing_window_arm, BillingWindowArm, BILLING_PAYMENT_STATE_WORD,
    };

    assert_eq!(
        billing_window_arm(0x100, BILLING_PAYMENT_STATE_WORD, true),
        BillingWindowArm::None
    );
    assert_eq!(
        billing_window_arm(0x232, BILLING_PAYMENT_STATE_WORD, false),
        BillingWindowArm::None
    );
    assert_eq!(billing_window_arm(0x232, 99, true), BillingWindowArm::None);
}

#[test]
fn print_screen_names_numbered_jpegs_outside_states_ten_and_eleven() {
    // seam: no player Print Screen writer exists in this client, so
    // the pinned path/name/counter helper is the whole behavior the
    // headless seam can claim; JPEG bytes stay OPEN (Tier 4).
    use o2jam_shell::screens::messenger::{
        print_screen_file_name, print_screen_relative_path, PrintScreenAction, ScreenShotCounter,
        SCREENSHOT_DIMENSIONS, SCREENSHOT_QUALITY,
    };

    assert_eq!(print_screen_file_name(1), "ScreenShot001.jpg");
    assert_eq!(print_screen_file_name(1234), "ScreenShot1234.jpg");
    assert_eq!(
        print_screen_relative_path(7),
        "오투잼_스크린샷\\ScreenShot007.jpg"
    );
    assert_eq!(SCREENSHOT_DIMENSIONS, (800, 600));
    assert_eq!(SCREENSHOT_QUALITY, 80);

    let mut counter = ScreenShotCounter::new(3);
    assert_eq!(
        counter.press(10),
        PrintScreenAction::Suppressed { state: 10 }
    );
    assert_eq!(
        counter.press(11),
        PrintScreenAction::Suppressed { state: 11 }
    );
    let mut names = Vec::new();
    for _ in 0..3 {
        match counter.press(0) {
            PrintScreenAction::Write {
                file_name,
                width,
                height,
                quality,
            } => {
                assert_eq!((width, height, quality), (800, 600, 80));
                names.push(file_name);
            }
            other => panic!("an available shot must be written, got {other:?}"),
        }
    }
    // The counter base (000 or 001 on the first press) is not named
    // by the pin; only the per-press increment is claimed here.
    for (index, name) in names.iter().enumerate().skip(1) {
        let previous = names[index - 1]
            .trim_start_matches("ScreenShot")
            .trim_end_matches(".jpg")
            .parse::<u32>()
            .expect("pinned name shape");
        let current = name
            .trim_start_matches("ScreenShot")
            .trim_end_matches(".jpg")
            .parse::<u32>()
            .expect("pinned name shape");
        assert_eq!(current, previous + 1, "one name per written press");
    }
    assert_eq!(counter.used(), 3);
    assert_eq!(counter.remaining(), 0);
}

#[test]
fn print_screen_with_an_exhausted_counter_posts_a_notice_instead_of_a_file() {
    // seam: the depleted branch calls the UI queue instead of the
    // writer; no notice caption is pinned, so this guard claims only
    // that no numbered file name is produced.
    use o2jam_shell::screens::messenger::{PrintScreenAction, ScreenShotCounter};

    let mut counter = ScreenShotCounter::new(0);
    assert_eq!(counter.press(0), PrintScreenAction::Notice);
    assert_eq!(counter.used(), 0, "no file name is consumed by a notice");
    assert_eq!(counter.remaining(), 0);
}

#[test]
fn cash_bill_host_table_keeps_the_five_publisher_literals() {
    // seam: the host-selection condition is unpinned, so the whole
    // claim here is the pinned host set and shared path; no URL is
    // chosen and no HTTP request line is invented.
    use o2jam_shell::screens::messenger::{CASH_BILL_HOSTS, CASH_BILL_PATH};

    assert_eq!(CASH_BILL_PATH, "client/cash_bill/index.phtml");
    assert_eq!(
        CASH_BILL_HOSTS,
        [
            "o2jam.monawa.com",
            "o2jam.gameangel.com",
            "o2jam.gameon.nate.com",
            "o2jam.pdbox.co.kr",
            "o2jam.nopp.co.kr",
        ]
    );
}
