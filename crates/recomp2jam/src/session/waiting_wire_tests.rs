//! Scripted-gateway guards for the Waiting wire families (#291 wire stories).
//! Declared `#[cfg(test)]` by `session.rs`, so `use super::*;` reaches the
//! crate-private session items the drain paths live on.

use super::session_tests::{joined_snapshot, joined_waiting_session, paint_until};

/// #994: the 3010 room-removal event picks one of the two pinned
/// captions (`0x4bb700`; CP949 strings at `0x64da60`/`0x64da88`).
mod removal_notice {
    use std::io::Write;

    use o2jam_net::frame::encode_frame;

    use super::*;

    /// One scripted 3010 cause through the real session drain. The
    /// caption is read from the same shell accessor the painter uses.
    fn caption_for_cause(cause: u8) -> Option<String> {
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            stream.write_all(&encode_frame(3010, &[cause])).unwrap();
        });
        paint_until(&mut session, |session| {
            session.game.waiting_notice().is_some()
        });
        let caption = session.game.waiting_notice().map(str::to_owned);
        server.join().unwrap();
        caption
    }

    /// Tier 2 scripted-gateway guard. Live caller chain (once the
    /// missing handler lands): `main.rs::run_play` ->
    /// `OnlineSession::paint` -> `poll_waiting_chat` ->
    /// `drain_waiting_chat` -> the 3010 caption arm ->
    /// `ScreenGame::waiting_notice`.
    ///
    /// BLOCKED: opcode 3010 has no handler in this branch, so both
    /// causes leave `waiting_notice` empty. Ignored until the senior
    /// re-slices the shell/session caption path; run with `--ignored`
    /// to see the red.
    #[test]
    #[ignore = "#994: shell/session 3010 removal-caption arm outside this lane's files"]
    fn removal_cause_selects_a_distinct_caption_from_the_gateway() {
        let no_song_caption = caption_for_cause(0);
        let plain_caption = caption_for_cause(1);
        assert_ne!(
            no_song_caption, plain_caption,
            "the two 3010 causes must reach different removal captions"
        );
    }
}
