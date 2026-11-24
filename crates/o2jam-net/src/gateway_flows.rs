//! Named scripted-gateway flow list (ticket #899, ADR-0014).
//!
//! Each flow is one request/response pair driven through a real
//! `GatewayConnection` against an in-test gateway. The list is the CI
//! bar: every named flow must send its request, take its answer, and
//! decode the answer with the codec the client uses.

#[cfg(test)]
mod gateway_flow_tests {
    use std::io::Write;
    use std::thread;

    use crate::command::Request;
    use crate::frame::{encode_frame, Frame};
    use crate::gift::GetGiftMessageListResponse;
    use crate::message::WireMessage;
    use crate::shop::SyncItemPurchaseReply;
    use crate::testing::{loopback_pair, read_frame};
    use crate::wallet::{StartPaymentResponse, SyncCashPointResponse, SyncFreePassResponse};
    use crate::{
        AuthResponse, ChannelList, ChannelLoginResponse, CharacterInfo, CreateResult,
        CreateRoomResponse, JoinRoomResponse, MusicScoreListAnswer, MusicScoreRow, RoomList,
        ScoreCompleted,
    };

    /// One request/response pair and the check on the answered frame.
    struct GatewayFlow {
        name: &'static str,
        request_id: u16,
        request_body: Vec<u8>,
        response: Frame,
        verify: Box<dyn Fn(&Frame) -> bool>,
    }

    impl GatewayFlow {
        /// Build a flow whose answer has a full codec.
        fn coded<T>(name: &'static str, request_id: u16, request_body: &[u8], answer: T) -> Self
        where
            T: WireMessage + PartialEq + 'static,
        {
            let response = Frame {
                id: T::COMMAND,
                payload: answer.encode_body(),
            };
            let verify =
                Box::new(move |frame: &Frame| T::decode_frame(frame).as_ref() == Ok(&answer));
            Self {
                name,
                request_id,
                request_body: request_body.to_vec(),
                response,
                verify,
            }
        }

        /// Build a flow whose answer is a raw body with a custom check.
        fn raw(
            name: &'static str,
            request_id: u16,
            request_body: &[u8],
            response_id: u16,
            response_body: Vec<u8>,
            verify: impl Fn(&Frame) -> bool + 'static,
        ) -> Self {
            Self {
                name,
                request_id,
                request_body: request_body.to_vec(),
                response: Frame {
                    id: response_id,
                    payload: response_body,
                },
                verify: Box::new(verify),
            }
        }
    }

    /// The named flow list published in `docs/net-protocol.md` §4f.
    fn named_flows() -> Vec<GatewayFlow> {
        let mut flows = login_flows();
        flows.extend(room_flows());
        flows.extend(play_flows());
        flows.extend(wallet_flows());
        flows
    }

    /// Planet and channel entry.
    fn login_flows() -> Vec<GatewayFlow> {
        vec![
            GatewayFlow::coded(
                "authorize_1000_1001",
                Request::Authorize.id(),
                &[],
                AuthResponse {
                    result_code: 0,
                    tail: Vec::new(),
                },
            ),
            GatewayFlow::coded(
                "channel_list_1002_1003",
                Request::GetChannelList.id(),
                &[],
                ChannelList { channels: vec![] },
            ),
            GatewayFlow::coded(
                "channel_login_1004_1005",
                Request::ChannelLogin.id(),
                &[0, 0, 0, 0],
                ChannelLoginResponse {
                    result_code: 0,
                    restricted: None,
                },
            ),
        ]
    }

    /// Channel board and waiting room.
    fn room_flows() -> Vec<GatewayFlow> {
        vec![
            GatewayFlow::coded(
                "character_info_2000_2001",
                Request::GetCharacterInfo.id(),
                &[],
                CharacterInfo::default(),
            ),
            GatewayFlow::coded(
                "room_list_2002_2003",
                Request::GetChannelInfo.id(),
                &[],
                RoomList { rooms: vec![] },
            ),
            GatewayFlow::coded(
                "create_room_2004_2006",
                Request::CreateRoom.id(),
                &[],
                CreateRoomResponse {
                    result: CreateResult::ChannelFull,
                    info: None,
                },
            ),
            GatewayFlow::coded(
                "join_waiting_3002_3003",
                Request::JoinWaiting.id(),
                &[0, 0, 0, 0, 0, 0],
                JoinRoomResponse {
                    result_code: 1,
                    room: None,
                },
            ),
        ]
    }

    /// Ranking lookup and the end-of-song tally.
    fn play_flows() -> Vec<GatewayFlow> {
        vec![
            GatewayFlow::coded(
                "music_score_list_2030_2031",
                Request::GetMusicScoreList.id(),
                &[],
                MusicScoreListAnswer {
                    result: 0,
                    rows: vec![MusicScoreRow {
                        music_id: 757,
                        difficulty_ranks: [7, 5, 3, 1],
                    }],
                },
            ),
            GatewayFlow::raw(
                "submit_score_4016_4018",
                Request::SubmitScore.id(),
                &[],
                ScoreCompleted::COMMAND,
                live_score_completed_body(),
                |frame| {
                    ScoreCompleted::decode_frame(frame).is_ok_and(|answer| {
                        answer.scores.len() == 1 && answer.scores[0].score == 123_456
                    })
                },
            ),
        ]
    }

    /// Shop, gifts, payment, and wallet refreshes.
    fn wallet_flows() -> Vec<GatewayFlow> {
        vec![
            GatewayFlow::raw(
                "shop_buy_5000_5001",
                5000,
                &[],
                5001,
                live_purchase_reply_body(),
                |frame| {
                    SyncItemPurchaseReply::decode_live_152(&frame.payload)
                        .is_ok_and(|reply| reply.prefix.gem == 626)
                },
            ),
            GatewayFlow::coded(
                "gift_list_5017_5018",
                5017,
                &[],
                GetGiftMessageListResponse {
                    invalid: false,
                    messages: vec![],
                },
            ),
            GatewayFlow::raw("payment_5015_5016", 5015, &[], 5016, Vec::new(), |frame| {
                StartPaymentResponse::decode_frame(frame).is_ok()
            }),
            GatewayFlow::coded(
                "cash_refresh_5028_5029",
                5028,
                &[],
                SyncCashPointResponse {
                    gem: 626,
                    cash_point: 0,
                },
            ),
            GatewayFlow::coded(
                "free_pass_5025_5026",
                5025,
                &[],
                SyncFreePassResponse {
                    result_code: 0,
                    info: None,
                },
            ),
        ]
    }

    /// The 50-byte solo 4018 body decoded by `score_completed_decodes_live_capture`.
    fn live_score_completed_body() -> Vec<u8> {
        vec![
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x14, 0x00,
            0x05, 0x00, 0x03, 0x00, 0x64, 0x00, 0x09, 0x00, 0x40, 0xe2, 0x01, 0x00, 0xd5, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x77, 0x07, 0x00, 0x00, 0x01, 0x95, 0x27, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x32, 0x00, 0x00, 0x00,
        ]
    }

    /// The 152-byte 5001 body observed on both uid-1 and uid-2 legs
    /// (`docs/net-protocol.md` §4c; `SyncItemPurchaseReply` type doc).
    fn live_purchase_reply_body() -> Vec<u8> {
        let mut body = vec![0u8; crate::shop::SYNC_ITEM_PURCHASE_LIVE_REPLY_LEN];
        body[..4].copy_from_slice(&626i32.to_le_bytes());
        body
    }

    /// Drive one flow: assert the request on the gateway side, then
    /// decode the answer on the client side.
    fn drive_flow(flow: GatewayFlow) {
        let name = flow.name.to_owned();
        let request_id = flow.request_id;
        let expected_request = Frame {
            id: request_id,
            payload: flow.request_body.clone(),
        };
        let answer = encode_frame(flow.response.id, &flow.response.payload);
        let (mut client, mut server) = loopback_pair();
        let gateway_side = thread::spawn(move || {
            let request = read_frame(&mut server);
            assert_eq!(request, expected_request, "request of flow {name}");
            server.write_all(&answer).unwrap();
        });

        let frame = client
            .exchange(request_id, &flow.request_body, flow.response.id)
            .unwrap_or_else(|error| panic!("flow {} failed: {error}", flow.name));
        gateway_side.join().unwrap();
        assert!(
            (flow.verify)(&frame),
            "answer of flow {} is wrong",
            flow.name
        );
    }

    #[test]
    fn every_named_flow_round_trips_through_the_scripted_gateway() {
        // Live chain (tier 2): `GatewayConnection::exchange` is the same
        // transport `connect::connect_and_login_with_transport` and
        // `OnlineSession` use; this guard pins codec + framing on that
        // shared seam. Window, archives, and the real server stay out.
        let flows = named_flows();
        assert_eq!(flows.len(), 14);
        for flow in flows {
            drive_flow(flow);
        }
    }
}
