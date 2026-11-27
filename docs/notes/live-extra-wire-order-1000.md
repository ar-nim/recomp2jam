# #1000 — live-extra wire order (2026-11-25)

Target: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Tools recorded in `docs/pins/net-a.md`: Ghidra 12.2 / REA 6.10.1.

## Pinned

`0x541690` / file `0x141690` consumes one wire byte into `+0x2131` directly
after the playing-state read (`docs/pins/net-a.md:232-238`; evidence
`ev 6aeb.2ada.373c.103a.9ed5.0f6b.e571.d2e5.d7b5.8599.19f2.f5a5.bbcd.c655.56b3.c245`).
The paired playing-state helper is `0x541670` / file `0x141670`, which
consumes one byte into `+0x2130` (`docs/pins/net-a.md:223-229`; evidence
`ev 8194.4e12.51d7.4df1.a136.cb5c.c572.081c.524c.5d90.79f5.ae8c.6334.625c.a486.aded`).
The later correction states that these fields are reader objects, not flat
byte cells (`docs/pins/net-a.md:380-394`; reader evidence
`ev_249bdde1d6aabc01f74e3ddd4c12818dc897a34e5671e2cf5b253a29f488dee4`,
sibling-setter evidence
`ev_38c0105a671b50ca7ef7c789019e5a89765fff2722d8af8c640a1a21ecc1ad69`).

The resolved caller evidence is
`ev_6d732c7df583d4e6a5f8d63d055b7e2701cb6917059946f47ea3a81f9cf1be17`
(setter) and
`ev_2286141f913f9ff1c4e4b504a4f9da8f54770ac2eddb3b1930c94e8d12898875`
(callers), cited at `docs/pins/net-a.md:376-378`.

Our `MemberInfo::decode_prefix` and `MemberInfo::encode_inner` already put
`playing_state` before `live_extra` (`crates/o2jam-net/src/relay/commands.rs:1384-1385`,
`:1428-1429`). No production change was needed for #1000. The Tier 0 guard
`live_state_active_member_exact_bytes` now names each byte's pinned helper
and address.

## OPEN

The VALUE stored at `+0x2131` is not measured (`docs/pins/net-a.md:237`,
`:404-405`). Next evidence step from the correction: find the code that
consumes the reader object at `+0x2131` (its data pointer lives past its own
header at the advanced cursor), not a load at a `+0x2131` displacement
(`docs/pins/net-a.md:405-408`; evidence
`ev_249bdde1d6aabc01f74e3ddd4c12818dc897a34e5671e2cf5b253a29f488dee4` and
`ev_38c0105a671b50ca7ef7c789019e5a89765fff2722d8af8c640a1a21ecc1ad69`).
No player-visible close follows from this order-only guard.

## Order claim conflicts with our codec

The addendum's order claim at `docs/pins/net-a.md:404` says "playing-state,
then live-extra, then music-state". That relative claim conflicts with our
codec: `MemberInfo::encode_inner` writes `music_state` at
`crates/o2jam-net/src/relay/commands.rs:1417`, ahead of the equipment words
and well ahead of the trailing pair at `:1428-1429`.

The `0x541480` row (`docs/pins/net-a.md:160-167`) pins only the `+0x217c`
offset and makes no wire-order claim. It is not the opposing party to the
addendum's order claim.

Only the `playing-state` → `live-extra` pair is needed for #1000 and it is
settled at `docs/pins/net-a.md:235`. The music-state relation stays OPEN;
recheck the four resolved caller chains (`0x55cad0`, `0x55d420`, `0x559b50`,
`0x55d820`) against the reader cursor order with the full caller evidence
`ev_2286141f913f9ff1c4e4b504a4f9da8f54770ac2eddb3b1930c94e8d12898875`
(`docs/pins/net-a.md:370-378`) before using that relation as evidence.
