# Embedded-IE remote URL archive audit — O2Jam 7.55

Date: 2026-11-25. This note lists the remote page URLs first, then records
where each URL goes, what the client uses it for, and what survives in web
archives. It does not claim a runtime access order: no client was booted for
this audit.

## Result

- The pinned exe contains **16 direct HTTP URL literals/templates** for
  embedded-IE surfaces. They use six hosts and 12 distinct path templates.
- One additional page-side name, `gameUser_list.asp`, is a URL suffix tested
  by the friend-list and music-shop browser controls. Its full path is OPEN.
- Only `client/coupon2/coupon.phtml` has a surviving exact Wayback capture in
  the completed exact-key checks: 22 captures, all HTTP 200 / `text/html`,
  from 2008-08-28 to 2012-02-21.
- The other 15 direct URLs returned no exact capture. Older sibling billing
  pages survive, but they are not the 7.55 ASPX paths.
- Four hosts did not resolve through the local system resolver. One returned
  a local sinkhole address. The remaining host returned HTTP 403 and an HTTPS
  certificate-name mismatch. Archives are therefore the only usable page
  source in this environment.

## Version pin and method

| Item | Value |
| --- | --- |
| Oracle | `OTwo.exe`, 5,029,888 B, SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` |
| Static tooling | REA 6.10.1, Ghidra 12.2, target `OTwo.exe` |
| Address rule | VA = file offset + `0x400000` |
| Web tooling | Pi Pocket `web_*` tools (`donsetch`) |
| Archive sources | Wayback exact timemap, Wayback availability API, direct `id_` snapshots, and the earlier local CDX/HTML captures under `~/scratch/o2jam-research/` |
| Limits | Wayback CDX returned a 503 bot challenge; wildcard coverage is incomplete. Common Crawl was unavailable. Arquivo.pt CDX was walled. Wayback screenshots failed with `ERR_CONNECTION_REFUSED`. |

Method: sweep initialized ASCII strings for HTTP URLs, join each URL to its
code references, and inspect the nine functions that call the IE host creator
`FUN_0057dac0`. Then query exact archive keys and read surviving bodies. This
is a static inventory plus archive research, not a live traffic capture.
Indirect or computed URLs outside these paths remain unknown.

## Direct URL inventory

These are the direct page navigations pinned in the exe. `<id>` is the decimal
user-index string built by the client. `<title>` is the armed Result-event
title value; its source remains OPEN.

```text
http://o2jam.monawa.com/client/cash_bill/index.phtml?USER_INDEX_ID=<id>
http://o2jam.gameangel.com/client/cash_bill/index.phtml?USER_INDEX_ID=<id>
http://o2jam.gameon.nate.com/client/cash_bill/index.phtml?USER_INDEX_ID=<id>
http://o2jam.pdbox.co.kr/client/cash_bill/index.phtml?USER_INDEX_ID=<id>
http://o2jam.nopp.co.kr/client/cash_bill/index.phtml?USER_INDEX_ID=<id>
http://o2jam.nopp.co.kr/client/event/2009/01/01_bingo/index.asp?USER_INDEX_ID=<id>
http://o2jam.nopp.co.kr/client/mtm/ingame_1n1qna01.phtml?USER_INDEX_ID=<id>
http://game1.o2jam.com/gamefind_new/gamefine_main.asp?USER_INDEX_ID=<id>&GiftOn=T|F
http://o2jam.nopp.co.kr/client/event/2009/06_moa/hot_time.php
http://o2jam.nopp.co.kr/client/event/2009/06_moa/ticket_add.php?USER_INDEX_ID=<id>&title=<title>
http://game1.o2jam.com/gamemusic/gamemusic.asp
http://game1.o2jam.com/np/billing/gift/gift_input.aspx?USER_INDEX_ID=<id>
http://game1.o2jam.com/np/billing/payment/payment_input.aspx?USER_INDEX_ID=<id>
http://o2jam.nopp.co.kr/client/coupon2/coupon.phtml?USER_INDEX_ID=<id>
http://game1.o2jam.com/client/jamcard.asp?USER_INDEX_ID=<id>
http://game1.o2jam.com/billing/imsi/buttons.asp
```

The `gamemusic.asp` literal has a leading `@` byte in the binary; the code
reference starts at the `h` of `http`, so the `@` is not part of the URL.

## Where each URL goes and what it does

| URL family | Host and path | Exe anchor | Client-visible or data role | Exact archive state |
| --- | --- | --- | --- | --- |
| `cash_bill/index.phtml` × 5 publisher hosts | Publisher host, `/client/cash_bill/` | Builder/host caller `0x45e010`; URL files `0x2466d0`–`0x2467b8` | Publisher-specific cash-charge landing page. The client appends `USER_INDEX_ID=<id>`. The page body and host-selection condition are not recovered here. | No exact capture found for any of the five URLs. |
| `event/2009/01/01_bingo/index.asp` | `o2jam.nopp.co.kr`, 2009 bingo event | `0x45ea60`; file `0x246888` | Bingo event browser surface. The client appends `USER_INDEX_ID=<id>`. Rules, rewards, forms, and page-to-client fields are OPEN. | No exact capture found. |
| `mtm/ingame_1n1qna01.phtml` | `o2jam.nopp.co.kr`, `/client/mtm/` | `0x45f190`; file `0x24695c` | In-game help/Q&A browser surface. The client appends `USER_INDEX_ID=<id>`. The actual questions and outbound links are OPEN. | No exact capture found. |
| `gamefind_new/gamefine_main.asp` | `game1.o2jam.com`, `/gamefind_new/` | Room host `0x495710`; file `0x248398` | Room bottom-left friend/user list in a `CWebFriendCtrl`, rect `(17,403)-(147,584)`. The client appends `USER_INDEX_ID=<id>` and `GiftOn=T/F`. The client injects `My_planet`, `My_channel`, and `CTuser_*` arrays. The page can return `SelectedUserNickName`, which stages a whisper target. | No exact capture found. |
| `event/2009/06_moa/hot_time.php` | `o2jam.nopp.co.kr`, 2009/06 MOA event | `0x4c1b90` and `0x4f5730`; file `0x24a284` | Hot-time event page. In three qualified solo Result boots, the zero-selector arm requested this URL and never requested `ticket_add.php`. The caller also appends the decimal user-index string; the exact separator/key in that assembled URL is OPEN. `docs/net-protocol.md` records a startup crash when this URL is unreachable. | No exact capture found. A later, different `/Event/Hottime/index.phtml` capture exists but is not equivalent evidence. |
| `event/2009/06_moa/ticket_add.php` | `o2jam.nopp.co.kr`, same event directory | Armed Result arm in `0x4c1b90`; file `0x24a2d0` | Event-ticket GET. The armed arm builds `USER_INDEX_ID=<id>&title=<title>`. It did not fire in the three qualified solo boots. The selector writer, title source, and two-member behavior remain OPEN. | No exact capture found. |
| `gamemusic/gamemusic.asp` | `game1.o2jam.com`, `/gamemusic/` | Host `0x4d2ff0`; URL file `0x24a9e8` | Web panel of the mixed music shop, rect `(4,0)-(384,541)`. The client can call page function `jsBuyMusic`. The page returns `mid` and `mflag`; `C`/`G` choose the cash/gem cart slot. | No exact capture found. |
| `np/billing/gift/gift_input.aspx` | `game1.o2jam.com`, `/np/billing/gift/` | URL builder `0x4f38b0`; host `0x4f3b30`; file `0x24b638` | Gift-flow entry page. The client appends `USER_INDEX_ID=<id>`. Older sibling ASP pages show recipient checking and a POST to step 2, but the exact ASPX body is not recovered. | No exact capture found. |
| `np/billing/payment/payment_input.aspx` | `game1.o2jam.com`, `/np/billing/payment/` | URL builder `0x4f38b0`; host `0x4f3b30`; file `0x24b670` | Payment-entry page. The client appends `USER_INDEX_ID=<id>`. Older sibling ASP captures are guard/error pages, not a recovered checkout form. | No exact capture found. |
| `client/coupon2/coupon.phtml` | `o2jam.nopp.co.kr`, `/client/coupon2/` | `0x4f4bd0`; file `0x24b844` | Coupon entry page. The client appends `USER_INDEX_ID=<id>`. An adjacent archived `coupon_in.phtml?myid=` page asks for coupon type/number and says one coupon per account. The exact landing-page script link is not exposed by the text extractor, so the join is INFERENCE. | 22 exact captures, all 200 / `text/html`, 20080828073314–20120221134220. The landing body is script-built; adjacent `coupon_in.phtml` survives. |
| `client/jamcard.asp` | `game1.o2jam.com`, `/client/` | `0x4f4bd0`; file `0x24b87c` | Jamcard page. The client appends `USER_INDEX_ID=<id>`. Archived sibling `popup/Layer_Jamcard/ajax_buy.php` and `ajax_id_check.php` both fail at `AES2DecryptString(NULL)` when fetched without parameters, which shows they expect encrypted input. Their exact purchase/check semantics remain OPEN. | No exact capture found. |
| `billing/imsi/buttons.asp` | `game1.o2jam.com`, `/billing/imsi/` | URL comparator `0x57f510`; file `0x24ded4` | Billing-buttons/return page. When the browser navigates to this URL, the client clears two host flags and forwards the cached URL as notify `0x12` / `0x1bbd`. This is the pinned billing-completion navigation seam. | No exact capture found. |

### Browser report-back, not extra URLs

The IE pages do not use a custom `o2jam://` protocol. They report through the
`EventBridge` COM object and browser navigation events:

- `SelectedUserNickName` → whisper-target staging.
- `mid` + `mflag` (`C` or `G`) → native cart add with cash/gem price slot.
- Document-complete navigation → cached URL notify `0x12` / `0x1bbd`.
- Client-to-page hooks: `ReloadConnUserList`, `jsBuyMusic`, `My_planet`,
  `My_channel`, and `CTuser_*`.

## Page-side name and derived request keys

- `gameUser_list.asp` is a bare suffix at file `0x24df20`. Functions
  `0x57f690` (`CWebFriendCtrl` vtable) and `0x580d70` (`CWebMShopCtrl`
  vtable) test a URL string for this suffix. It is not a direct launch URL.
  The full path, host, and parameters are OPEN.
- Music-shop code at `0x4d3170` builds form/query fragments `mids=`, `&nid=`,
  `&hid=`, and `&wid=`. The exact HTTP method and target page for each fragment
  are not pinned in this audit.
- These names are not counted among the 16 direct URLs.

## Archive findings

### Coupon

Exact timemap source:
`https://web.archive.org/web/timemap/json?url=http%3A%2F%2Fo2jam.nopp.co.kr%2Fclient%2Fcoupon2%2Fcoupon.phtml%3F&fl=timestamp,original,statuscode,mimetype`

The 22 rows all record the same original URL without the trailing `?`, HTTP
200, and `text/html`. A 2009-era capture is `20090228070954`. The direct
snapshot is script-built and exposes no visible text through `web_fetch`.

Adjacent surviving pages:

- `http://o2jam.nopp.co.kr:80/client/coupon2/coupon_in.phtml?myid=` at
  `20080901140716`, HTTP 200: coupon registration form. Source:
  `https://web.archive.org/web/20080901140716id_/http://o2jam.nopp.co.kr:80/client/coupon2/coupon_in.phtml?myid=`
- `http://o2jam.nopp.co.kr:80/client/coupon2/coupon_in_proc.phtml` at
  `20080903090148`, HTTP 200: script-built processing page with no readable
  visible text.
- `http://o2jam.nopp.co.kr:80/popup/Layer_Jamcard/ajax_buy.php` at
  `20080930185916`, HTTP 200: archived response is a PHP exception caused by
  a null encrypted parameter.
- `http://o2jam.nopp.co.kr:80/popup/Layer_Jamcard/ajax_id_check.php` at
  `20080930185936`, HTTP 200: same null-parameter failure pattern.

The adjacent paths support a coupon-registration and Jamcard-check flow, but
they do not prove the exact script requests made by the 7.55 landing pages.

### Older gift and payment siblings

The local 2009-02-28 captures preserve these same-host pages:

- `billing/gift/gift_step_01.asp?` — recipient ID/nickname form. It checks the
  recipient through `memberNick_Chk.asp`, then POSTs to `gift_step_02.asp`.
- `billing/gift/memberNick_Chk.asp` — recipient eligibility result and return
  to step 1.
- `billing/gift/gift_step_02.asp` … `gift_step_04.asp` — tiny alert/referer
  guard pages in the captured state, not a complete gift transaction.
- `billing/payment/payment_step01.asp?`, `payment_step02.asp?`, and
  `payment_step03.asp?` — alert/referer guard pages.
- `billing/payment/mcash_error_CashType.asp` — invalid-product payment error.
- `billing/payment/payment_input_01.asp` — HTTP 302 at `20090228085245`;
  Wayback did not record the redirect target.

Primary snapshot:
`https://web.archive.org/web/20090228031613id_/http://game1.o2jam.com:80/billing/gift/gift_step_01.asp?`

Local provenance for the saved gift body:

| File | SHA-256 |
| --- | --- |
| `20090228031613_http___game1_o2jam_com_80_billing_gift_gift_step_01_asp_.html` | `248a89f2b4573e0e56723aacb1f7b1665e49e731895faa29ba6aee46f3507120` |
| `gift_step01.utf8.html` | `7cb47ea1a71b52fbc6cdfb753dcbeee4791ef1a06b1b6b41840f8b07df1d8cd4` |

These are 2009 sibling paths, not proof of the 7.55 `gift_input.aspx` or
`payment_input.aspx` bodies.

### Missing exact bodies

Exact timemap and availability checks returned no capture for the other 15
direct URLs: the five cash-bill hosts, bingo, Q&A, `gamefine_main.asp`,
`hot_time.php`, `ticket_add.php`, `gamemusic.asp`, gift input, payment input,
Jamcard, and billing buttons. A negative exact-key result is not proof that
the page never existed. Wildcard CDX coverage remains incomplete because of
the archive access limits above.

## Current destination check

Local resolver and HTTP checks on 2026-11-25:

| Host | Local result |
| --- | --- |
| `o2jam.monawa.com` | No A/AAAA answer from the system resolver |
| `o2jam.gameangel.com` | System resolver returned `0.0.0.0` / `::` (local sinkhole behavior, not an authoritative origin address) |
| `o2jam.gameon.nate.com` | No A/AAAA answer from the system resolver |
| `o2jam.pdbox.co.kr` | `121.78.175.221`; HTTP root returned 403 Forbidden; HTTPS certificate name did not match |
| `o2jam.nopp.co.kr` | No A/AAAA answer from the system resolver |
| `game1.o2jam.com` | No A/AAAA answer from the system resolver |

This is a local reachability check, not a global DNS census.

## Static evidence added by this audit

All IDs below come from the pinned 7.55 target through REA 6.10.1 / Ghidra
12.2. Earlier evidence in `docs/web-surfaces.md` remains valid.

| Function | Role | Evidence ID |
| --- | --- | --- |
| `0x45e010` | Five cash-bill URL variants | `ev_c0173d8cb413f497635a27dc29ebbfd418623f969ebc0e5efbca5d9dc30e49cd` |
| `0x45ea60` | Bingo URL and `USER_INDEX_ID=` | `ev_17e88c82bac3356d6695bccb5ac7ca212b1bc82e5d710f9a6f8de627e08484da` |
| `0x45f190` | Q&A URL and `USER_INDEX_ID=` | `ev_e46507c8658f220f07821384780afc9eae7b6d825c533b9a91bf693cea4ff623` |
| `0x495710` | Room URL plus `GiftOn=T/F` | `ev_7f5a49b8f0ac9228964420acf703239e095c333fa18402be12eb05171f54e862` |
| `0x4c1b90` | Hot-time/ticket Result gate and `&title=` | `ev_64c08939b14dde0875dc89c4575816037088b50a8a9b6bcbadf9a0db546e1ef6` |
| `0x4d2ff0` | Music-shop host URL | `ev_dc36869d7cd23e1ee11aa9ddcaadacc044b7ebaab0774f6a0e632b7bf13e6a9e` |
| `0x4d3170` | Music-shop `mids/nid/hid/wid` fragments | `ev_f168e60ea11189e70d572607e63578ce8a3145ff495a11d96563d7ca66eca495` |
| `0x4f38b0` | Gift/payment URL builders | `ev_449b4bd537704545c8c9aa75f4e7d964e68ab1af2a7a8a4e3edc4b14c212c91f` |
| `0x4f4bd0` | Coupon/Jamcard URLs | `ev_0f4801dce99fa1415203fcc0515c7c0b5d8acf142540c536d74a31b67093a618` |
| `0x4f5730` | Second hot-time host caller | `ev_9a5aae5f337c6348cc8730891bc3cacebc75b88d60bd9537717044d88f4d5eb3` |
| `0x57f510` | Billing-buttons completion URL | `ev_f0d036a1d170bbeb00b36f6826f4f28af004c4909d593f39ed514943d6529d67` |
| `0x57f690` | `CWebFriendCtrl` suffix test for `gameUser_list.asp` | `ev_5619ed7681737baf6f19c9b024fbdb1d1579f28f5341f386532d70c7cad69a93` |
| `0x580d70` | `CWebMShopCtrl` suffix test for `gameUser_list.asp` | `ev_aca023b6d588e4952a5da7cbcca37ea799422607b68338dd89e581267658a` |

## Contradictions and OPEN items

1. **Startup role of `gamefine_main.asp`.** `docs/lifecycle.md` calls this a
   startup game-find page. `docs/web-surfaces.md` pins the exe caller chain to
   the Room friend pane and refutes browser-hosted channel/planet selection.
   The Room use is supported. A separate startup use of the same URL remains
   OPEN; the contradiction is preserved, not silently resolved.
2. **Archive completeness.** Fifteen exact direct bodies are missing, and
   wildcard CDX coverage is incomplete. Re-run from a network that can use CDX
   without the bot challenge.
3. **Gift/payment ASPX bodies.** The 2009 ASP siblings show the older flow, not
   the exact 7.55 ASPX documents. Recover an exe-time page capture or a
   server-side archive of `/np/billing/`.
4. **Hot-time assembled suffix.** The literal has no query separator. The
   caller appends the decimal user-index string, but the exact separator/key
   is not established. The armed `ticket_add.php` title source is also OPEN.
5. **Music-shop request contract.** `mids`, `nid`, `hid`, and `wid` are pinned
   fragments. Their HTTP method, target, and field semantics need a request
   capture or page body.
6. **`gameUser_list.asp`.** The suffix test is pinned; full URL and parameters
   are OPEN.
7. **Runtime access order and multiplayer behavior.** This audit did not boot
   the client. The three prior solo Result boots are recorded in
   `docs/captures/journey-audit/294-result-story-matrix.md`; two-member Result
   GET behavior remains OPEN.

## Excluded non-IE HTTP endpoints

These are real client URLs but are not embedded-IE page navigations, so they
are outside the 16-item list:

- Score/replay upload: `http://upload.nopp.co.kr:18000/O2JamUpload/FileUpload.aspx`.
- Patch/version files and publisher download hosts used by the launcher.
- Microsoft symbol-server path embedded for debug metadata.
- FTP music hosts and crash-dump FTP endpoints.
