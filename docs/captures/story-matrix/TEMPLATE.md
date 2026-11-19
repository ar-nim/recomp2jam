# Story-matrix template — actor × action × observation

Use this template for every state spec in the story-coverage program (#381).
One matrix per spec lives at `docs/captures/story-matrix/<spec>.md`; the
missing stories go into the spec body as checklists, not only into the matrix.
The 7.55 executable is the behavior oracle. Memoryer is an interoperability
witness, never a substitute for an exe pin.

## 1. Actor set

Every row is raised by one actor; every column names one observer. Keep the
names fixed across specs so wave-2 lanes can be compared.

| Actor | Meaning | Typical stimulus source |
|---|---|---|
| `solo` | Solo user, no gateway session | `play --window` / offline boot |
| `master` | Room master (slot `+0x30 == 1`, local id) | room creator / handoff target |
| `peer` | Room peer (joined guest, non-master seat) | joiner / regular guest |
| `channel` | Channel observer in Room or channel board, not in the room | 07D2 poll / 2005 / 2007 events |
| `server` | Server-initiated frame with no local click | Memoryer event, 0FC4 member sync, 4026/4037 state |

A stimulus performed *by* a column actor is written `self:<actor>`; a
stimulus performed by another actor is written `other:<actor>`. Do not fold
two actors into one cell: "master sees peer join" and "peer sees master's
seat" are separate cells.

## 2. Action taxonomy per state

List actions from the spec's own checklist and the state's wire family. Split
one action per row; never combine create/join/leave or ready/start.

- **Presence:** enter state (create, join, solo), return, exit, idle.
- **Room lifecycle:** create, join, peer join, peer leave, last-player
  disband, master leave with peer remaining (role handoff).
- **Role controls:** MUSELECT, song-row pick, room echo, song/speed/difficulty,
  team/instrument, READY, START gate, START.
- **Conversation and art:** room chat, notice, download notice/acquire,
  progress/completion, avatar/pose, badge, dialogs.
- **Server frames:** member sync, member music state, room list events,
  DOWNLOADING badge (07E8), gate-close event, score/start handoff.

Every action row must name its state entry point: boot/create/join or a
later live event. An action that cannot be reached from the live caller chain
is labeled `NA-DESIGN` with the reason; it is not silently counted as passed.

## 3. Observation cell format

One cell = `<stimulus> → <observed outcome for that observer> → <evidence class>`.
The observed outcome is what the 7.55 exe actually did; it becomes the
story's expected-behavior text. Our client is conformed to that text later.
Each cell carries one status:

| Status | Meaning |
|---|---|
| `EVIDENCED` | At least one admissible evidence class names this exact cell. |
| `GAP` | No admissible evidence names it; next evidence step is mandatory. |
| `LIVE-FILLED` | Filled by a bounded simulation script in this program; cite the new checkpoint pack/wire log. |
| `OPEN-FIX` | Behavior is known but our client diverges and the fix is parked by the owner. |
| `NA-DESIGN` | Not a 7.55 behavior claim; name the design seam. |

Evidence classes, in descending authority:

1. `pack hash` — qualified capture with manifest + citation; name the file and
   its full sha256. Cite `--claim exe` for exe, `--claim none` for Rust.
2. `wire pin` — opcode/body with offset or frame hash and the live tap path.
3. `dossier id` — REA evidence id / VA / file offset from the pinned 7.55 binary
   (SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
4. `owner observation` — explicitly attributed human observation; keep the
   issue and date. It pins a symptom, not a mechanism.
5. `OPEN — <next evidence step>` — a concrete, runnable step (pack to capture,
   frame to read, proxy field to flip). Never leave a bare `OPEN`.

A cell can hold several classes, but a green Rust guard alone is never an
evidence class for exe behavior. Server state shared by both clients is not a
scenario delta; pair it with a wire log and a per-client observation.

## 4. Matrix shape

Rows are actions, columns are observing actors. Fill every intersection:

```markdown
| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| Enter Waiting (self:peer join) | NA — no solo session | `<join> → master's seat grid stays plain → dossier ...` | `<join> → peer sees master seated → pack hash ...` | `<join> → room row count unchanged → wire pin ...` | `<0BBB/2003> → seat list includes joiner → wire pin ...` |
```

Read each cell as a story candidate. EVIDENCED cells may already be in the
spec checklist (add the row reference in the matrix, do not duplicate the
story). GAP and OPEN-FIX cells become checklist rows; each row names its
evidence class and, for gaps, the next evidence step.

## 5. Scenario script format (simulation)

Populate the matrix by **simulation**, not one boot per cell. One scenario
script drives a bounded two-client session through an ordered action chain
(create → join → ready → chat → gate-toggle → song-change → leave →
disband); one boot pair, plus a channel-observer socket, can cover dozens of
cells in about ten minutes. Every action step names the acting actor, and
every checkpoint captures each observer's screen **and** wire, filling one
cell per observer column.

A script has a stable ID and these fields. A Markdown table or JSON with the
same fields is acceptable:

| Field | Requirement |
|---|---|
| `scenario_id`, `spec`, `state`, `trace_id` | Stable across the session and both manifests. |
| `boot_pair` | Two tagged actors: e.g. `M = uid 1 / master`, `P = uid 2 / peer`; optional `O = uid 3 / channel observer socket`. |
| `containment` | Private display per client, owned null sink, `--no-sound`, no default-sink stream, recorded server pid/epoch. |
| `steps` | Ordered `seq, action, actor, entry_state, checkpoint_id`; one action per step, actors tagged in every step. |
| `checkpoint` | For each step: `<row> × <observer>` cell ids, per-client frame hash, wire tap id/body, observer poll line. |
| `outcome` | The exe-observed sentence for each cell, or `OPEN — <next evidence step>`. |

Expected-outcome sentence pattern (fill from the exe observation, never from
our client's behavior):

> When `<actor>` `<action>` in `<state>`, `<observer>`'s `<observable>` is
> `<observed outcome>` (pack `<full sha256>`, wire `<dir:id:body>`, dossier
> `<id>`).

Example shape: "When the peer readies, the master's READY pill flips orange
(pack `<hash>`, wire `0FC4`);" the actual color and hash must come from that
checkpoint. If the exe observation is missing, keep the cell `GAP` with the
next capture step; never invent an expected outcome.

Simulation discipline:

1. Poll `ss -tnp | grep 15010` and `ss -ltnp | grep 15010`; one lane only,
   never restart or signal the shared server.
2. Boot the tagged pair sequentially, then walk the scripted chain. Do not
   stop between cells; a checkpoint is a screen+wire sample, not a new boot.
3. At each checkpoint capture every actor's frame, the bidirectional wire
   tap, and the channel observer's 2003 poll. Record the trace id and any
   popup, focus, or timeout deviation as run metadata.
4. Transcribe each observed outcome into the matrix cell, then into the spec
   story as its expected-behavior text. A later conformance run tests our
   client against those stories; a mismatch becomes a bug, never a story edit.
   Only new exe evidence can change a story.

## 6. Gap-to-story hand-off

1. Inventory all existing evidence *before* marking gaps. Include negative
   packs and stale probes in the inventory with their verdicts.
2. Rank gaps by player visibility × reachability, then write one scenario
   script that traverses the whole chain. Record a boot ledger (display,
   tagged actors, sink, scene, wire tap, checkpoints, result). One live lane
   at a time on the shared Memoryer.
3. Preflight `ss -tnp | grep 15010` and `ss -ltnp | grep 15010`; announce and
   never restart/signal the shared server. Full containment: private Xvfb,
   per-run null sink, `route=sink`, `on_default_sink=0`, `preboot_breach=0`.
4. Write the new stories into the spec body under `## Multi-actor stories`,
   each as `- [ ] As <actor>, I want <exe-observed outcome>, so <consequence>. — evidence: <class>; verify: <step>`.
   The exe-observed outcome is the expected behavior; later client conformance
   failures are bugs, not story edits. Add one tracker comment naming what was
   added and why; read the body back.
5. Report counts: cells, EVIDENCED, GAP, LIVE-FILLED, OPEN-FIX, NA-DESIGN.
   A story is not closed by a matrix cell; closing still needs the live-path
   caller chain and the project gates.

## 7. Integrity rules

- No copyrighted bytes enter the repo; keep captures under local scratch and
  commit only hashes and textual findings.
- Describe exe behavior in new words with offsets and evidence ids; never paste
  disassembly or decompiled code.
- Conflicting sources are recorded as contradictions (exe wins as oracle), not
  silently reconciled. A server-only behavior is not called exe behavior.
- Document every failed boot or checkpoint as a failed attempt with its
  reason; a failed attempt never fills a cell.
- A story is the exe's observed outcome, not our client's current output.
  When conformance differs, file or update a bug; do not rewrite the story
  to match the implementation.
