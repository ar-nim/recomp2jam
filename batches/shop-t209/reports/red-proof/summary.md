# shop-t209 batch summary (20261114-102833)

base `90970dd`; `rustc 1.99.1 (58cf8f9c3 2026-10-13) (Arch Linux rust 1:1.99.1-2)`; `Python 3.14.10`; data=present; live=0; red_proof=1

| row | verdict | checks (pass/fail/skip) | notes |
|---|---|---|---|
| item-shop-browse-buy | failed | see report.json | |
| music-shop-browse-buy | needs_review | see report.json | |
| myroom-enter-equip | needs_review | see report.json | |
| channel-closed-list | needs_review | see report.json | |

live (interop evidence, opt-in, not counted above): live-channel-list = skipped

gate exit: 1 (failed=1)

artifacts: /home/arnim/scratch/shop-t209/20261114-102833 (PNGs local-only, never committed)

doubts for the senior (not silently resolved):
- final_hash does not discriminate MyRoom equip vs gift-claim end states
  (observable_state_hash covers shop/music-shop carts but no myroom
  equip/claim state); per-trace repeat determinism still holds and
  end states are pinned by guard legs, never by the hash alone.
- no references/manifest-schema.md or pipeline-layout.md exists; the
  manifest shape is documented in batches/shop-t209/README.md instead.
- live population is asserted == 0 (isolated-rig assumption); any
  second gateway session would turn live-channel-list FAIL.

## red proof

break: `ScreenGame::take_shop_buy` drops the queued 5000 BUYITEM commit
(see out-dir red-proof/break.patch; full suite skipped by design)

row verdicts under the break: item=failed music=needs_review myroom=needs_review channel=needs_review
red proof: PASS (exactly item-shop-browse-buy failed, rest green)
recovery: PASS (both failing guards green after restore)
