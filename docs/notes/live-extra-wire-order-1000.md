# #1000 — live-extra wire order (2026-11-25)

Target: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Tools recorded in `docs/pins/net-a.md`: Ghidra 12.2 / REA 6.10.1.

## Pinned

`0x541690` (file `0x141690`) consumes one wire byte into the reader object
at member displacement `+0x2131`, directly after the playing-state helper
`0x541670` (file `0x141670`) consumes its byte into `+0x2130`. Evidence:
`ev 6aeb.2ada.373c.103a.9ed5.0f6b.e571.d2e5.d7b5.8599.19f2.f5a5.bbcd.
c655.56b3.c245`. The resolved caller evidence is
`ev_6d732c7df583d4e6a5f8d63d055b7e2701cb6917059946f47ea3a81f9cf1be17`
and
`ev_2286141f913f9ff1c4e4b504a4f9da8f54770ac2eddb3b1930c94e8d12898875`.

Our `MemberInfo::decode_prefix` and `MemberInfo::encode_inner` already put
`playing_state` before `live_extra` (`crates/o2jam-net/src/relay/commands.rs`).
No production change was needed for #1000. The Tier 0 guard
`live_state_active_member_exact_bytes` now names each byte's pinned helper
and address.

## OPEN

The VALUE stored at `+0x2131` is not measured. Next evidence step from
`docs/pins/net-a.md`: find the code that consumes the reader object at
`+0x2131` (its data pointer lives past its own header at the advanced
cursor), not a load at a `+0x2131` displacement. No player-visible close
follows from this order-only guard.

## Contradiction in the pin file

The corrected addendum says the wire order is "playing-state, then
live-extra, then music-state" (`docs/pins/net-a.md:403`). That relative
claim conflicts with the same pin's `0x541480` / file `0x141480`
music-state row and with our codec, where `music_state` is the third seat
byte ahead of the trailing pair. Only the `playing-state` → `live-extra`
pair is needed for #1000 and it is settled. The music-state relation stays
OPEN; recheck the four resolved caller chains against the reader cursor
order with `ev_2286141f913f9ff1c4e4b504a4f9da8f54770ac2eddb3b1930c94e8d12898875`
before using it as evidence.
