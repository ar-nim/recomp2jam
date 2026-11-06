# Pill shop earn flow (7.55) — traced, no shop grant

Version pin: `AngelO2Jam.exe` == `OTwo.exe`, 5,029,888 B,
SHA-256 `83a04541…72c823b`. File offset = VA − `0x400000`.
Pill stock lives at game object `+0x398c` (sub-object `+0x1820`
field `+0x216c`); the streak lives at `+0x3990` (sub `+0x2170`).

## Verdict first

No shop or item-use path grants pills in-exe. The only verified
grant of `+0x398c` is the 15-consecutive-Cool streak adder inside
the rating-event dispatcher `FUN_0056e170` (file `0x16e170`,
dossier `ev_c168abac…`). Purchase handlers (`CItemManager`,
`CBillManager`, `CMyBag`, `CMyCart`, shop packets 5000–5041)
never write `+0x398c`: exhaustive displacement and caller scans
find no writer outside the streak path, init, and song-start
restore copies. No new model was added; the existing
`o2jam-game::{CoolStreak, PillStock}` already covers the only
verified grant. Do not implement a shop grant without a verified
site (rule: bytes stay read-only, no model without a site).

## Every writer of `+0x398c` / sub `+0x216c`

| Site | Role |
|---|---|
| Setter VA `0x540870` (file `0x140870`): `this+0x216c = arg` | All restores go through it |
| Adder VA `0x540b50` (file `0x140b50`): `this+0x216c += arg` | Grant (`+1`) and spend (`−1`) |
| Grant call VA `0x56e509` (file `0x16e509`), in `FUN_0056e170`, dossier `ev_c168abac…` | 15th consecutive Cool mints one pill: gated by stock `< 5` (compare file `0x16e4f8`), streak `> 14` (compare file `0x16e4d1`), mode byte `6` exempt (compare file `0x16e4e7`), then streak reset via `0x540890` (file `0x140890`) |
| Spend call VA `0x56e84d` (file `0x16e84d`), same dispatcher | Bad with stock `> 0` (compare file `0x16e827`, read file `0x16e81e`) spends one (`−1`); mode byte `6` exempts (compare file `0x16e83d`) |
| Streak-only increment VA `0x56e4c0` (file `0x16e4c0`) | Cool while stock `< 5` (compare file `0x16e4af`) bumps the streak via `0x540b80` (file `0x140b80`) with no pill write |
| Init zero file `0x13fca7` / `0x13fde2` | Song/member setup zeroes `+0x216c` alongside `+0x2170` |
| Bulk struct copy VA `0x4e95d3` (file `0xe95d3`) | Member-to-member field copy `0x216c(src) → 0x216c(dst)`, same family that copies `+0x214e`/`+0x2150`/`+0x2152`/`+0x2154`/`+0x2158` (score/jam/combo words) |
| Bulk member sync VA `0x4dfed0` (file `0xdfed0`), pill/streak calls file `0xdfeec`/`0xdff04` | Cross-object copy of `+0x398c`/`+0x3990`/`+0x3970`/`+0x396e` into a fresh member slot (stride `0x2190`), not a purchase |
| Song-start restores file `0x18e459`/`0x18e5ae`/`0x18e6a5`/`0x18e7f9`/`0x18e8ec`/`0x18ea41` | One function family (entry ~file `0x18e2d0`, branching on the word at VA `0x77ee84`) copies staged pill/streak/score values into game objects via the same setters; packet/stack-sourced, no shop opcode nearby |
| Miss-path streak reset VA `0x570859` (file `0x170859`) | Zeroes `+0x2170` only; no pill logic (verified absence) |

Direct-displacement scan of `.text` for `0x216c`/`0x398c` returns
only the rows above (plus reads and stack-slot false positives),
so the table is exhaustive. Caller scan of the setter/adder
(`0x540870`/`0x540b50`) returns only the callers above — none in
shop/packet-handler address ranges.

## Shop trace (negative)

- RTTI anchors (names only, no vtable xrefs in `.text`): `CItemManager`
  file `0x27eb40`, `CMyBag` file `0x27eb88`, `CMyCart` file `0x27eba0`,
  `CBillManager` file `0x27f9d8`; states `CStateShop`/`CStateMShop`
  per `docs/lifecycle.md` §12.
- Shop packets are client sends (`push $0x1388` = 5000 SyncItemPurchase
  at files `0x3289`/`0x3d9c3`/`0x3da5d`/…, `0x1392` = 5010, `0x138e` =
  5006 EnterShop, etc.) plus a server dispatch through the opcode
  jump tables at VA `0x558940`/`0x5589c4`/`0x5589f4`/`0x558aa0`
  (function entry ~file `0x158200`, shop branch `cmp $0x1389`
  at file `0x15833d`). No handler in that family calls the pill
  setter/adder, and no direct `0x216c` store sits in those ranges.
- The consumable catalog (`Itemdata_Korea.dat`, rings `11A080xx`)
  lists score/exp/jam bonuses, never pills; skill rings affect
  lanes/visibility, not stock.

## Model decision

Keep `CoolStreak` + `PillStock` (cap `MAX_PILLS = 5`) as the sole
earn/spend model. A shop-granted `PillStock::add` call exists as a
safe API (cap-enforced), but no caller is wired to any purchase
path — wiring one requires a verified write site first.

## Tests

- `cargo test -p o2jam-game` green (existing `cool_streak_*`,
  `pill_*`, `rated_*` suites pin the 15-Cool grant, cap 5,
  Bad-only spend, Miss-spends-nothing).
- `cargo fmt --check` clean (this change adds docs only; no
  `src/` edits, so no new lint surface).
