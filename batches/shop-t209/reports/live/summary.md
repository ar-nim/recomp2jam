# shop-t209 batch summary (20261114-102337)

base `90970dd`; `rustc 1.99.1 (58cf8f9c3 2026-10-13) (Arch Linux rust 1:1.99.1-2)`; `Python 3.14.10`; data=present; live=1; red_proof=0

| row | verdict | checks (pass/fail/skip) | notes |
|---|---|---|---|
| item-shop-browse-buy | passed | see report.json | |
| music-shop-browse-buy | needs_review | see report.json | |
| myroom-enter-equip | needs_review | see report.json | |
| channel-closed-list | needs_review | see report.json | |

live (interop evidence, opt-in, not counted above): live-channel-list = passed

gate exit: 0 (failed=0)

artifacts: /home/arnim/scratch/shop-t209/20261114-102337 (PNGs local-only, never committed)

doubts for the senior (not silently resolved):
- final_hash does not discriminate MyRoom equip vs gift-claim end states
  (observable_state_hash covers shop/music-shop carts but no myroom
  equip/claim state); per-trace repeat determinism still holds and
  end states are pinned by guard legs, never by the hash alone.
- no references/manifest-schema.md or pipeline-layout.md exists; the
  manifest shape is documented in batches/shop-t209/README.md instead.
- live population is asserted == 0 (isolated-rig assumption); any
  second gateway session would turn live-channel-list FAIL.
