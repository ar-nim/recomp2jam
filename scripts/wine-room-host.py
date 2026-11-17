#!/usr/bin/env python3
"""Host a Memoryer room so the Wine client has something to join (#240).

The 7.55 client cannot create a room under Wine (its CREATE ROOM OK never
fires: `docs/wine-boot.md`, Attempts 10-12), and a room list without a
room paints no row, so the Wine leg's room/waiting/loading scenes need a
host outside the exe. This script is that host: it authorises as one
Memoryer account, creates `--title`, sets the room music, waits for the
guest's `0BBC` join event, then sends `4010` StartGame plus its own
`4012` ConfirmMusicLoaded, and holds the session with `6001` pings.

Framing and bodies are the documented wire shapes
(`docs/net-protocol.md`, `[u16 len][u16 id][body]`, null-terminated
strings); the opcode sequence is the one the CLI song flow sends
(`crates/recomp2jam/src/connect.rs` `run_song_flow`) restricted to what
the host needs to bring a guest to the loading screen.

The account token is read read-only from the Memoryer database
(`mode=ro`); nothing is written to the server or the DB.

Two modes: host a room for the guest, or `--wire-tap` a pass-through
TCP relay that logs the exe's game-TCP frames into the run out-dir. The
relay logs every frame in both directions; S2C 2005 is the room-created
event that opens the client's row-paint window.

Usage:
  scripts/wine-room-host.py --out-dir DIR [options]
  scripts/wine-room-host.py --wire-tap --out-dir DIR --wire-log PATH \\
      --listen-port 15012 --upstream-port 15010

Options:
  --out-dir DIR        directory for host.log (required)
  --user-index-id N    host account (default 3 = t207probe; the rig's
                       guest is account 1 = tester)
  --db PATH            Memoryer database (default ~/memoryer-local/O2JAM.db)
  --title NAME         room title (default WINEJOIN; max 22 bytes)
  --music-id ID        room music (default 757)
  --start-delay S      seconds to wait after the guest joins before 4010
                       (default 20: the drive screenshots the waiting room
                       and presses READY inside this window)
  --cmd-file PATH      command file the drive writes (default
                       OUT_DIR/host.cmd; tokens `recreate` and `start`).
                       `recreate` rebuilds the room while the guest sits on
                       the room stage, which repaints the list from fresh
                       2005 data and keeps the ~40 s no-response modal away
                       (docs/wine-boot.md, Attempt 12).
  --hold S             seconds to hold the session (default 300)
  --stop-file PATH     touching this file ends the hold early

Log lines are the run's witness for host-side events:
  host room created number=N
  host guest JOINED
  host start sent 4010+4012
  host opcode 4014 ...   (every non-ping frame, with its body hex)
"""

from __future__ import annotations

import argparse
import socket
import sqlite3
import struct
import sys
import threading
import time
from pathlib import Path

GATEWAY = ("127.0.0.1", 15010)
CLIENT_ID = "726PNA-B1YW-28ZG9M-I5SC-Q0OPM6"
PING = 6001
ROOM_CREATED = 2005
WIRE_BODY_PREVIEW = 128

AUTHORIZE = 1000
AUTHORIZE_ANSWER = 1001
GET_CHANNEL_LIST = 1002
CHANNEL_LIST = 1003
CHANNEL_LOGIN = 1004
CHANNEL_LOGIN_ANSWER = 1005
CREATE_ROOM = 2004
CREATE_ROOM_ANSWER = 2006
SET_ROOM_MUSIC = 4000
SET_ROOM_ARENA = 4002
SYNC_MEMBER_MUSIC_STATE = 4036
SET_DOWNLOAD_PROGRESS = 4038
START_GAME = 4010
CONFIRM_MUSIC_LOADED = 4012
JOIN_WAITING_EVENT = 0x0BBC
ROOM_SPEED = 1


def frame(opcode: int, body: bytes) -> bytes:
    return struct.pack("<HH", 4 + len(body), opcode) + body


def encode_string(text: str, cap: int = 65535) -> bytes:
    raw = text.encode("utf-8")[: cap - 1]
    return raw + b"\x00"


def encode_endpoint(ip: str, port: int) -> bytes:
    return bytes(int(part) for part in ip.split(".")) + struct.pack("<H", port)


def split_wire_frames(buffer: bytes) -> tuple[list[bytes], bytes]:
    """Return whole length-prefixed frames and the bytes still incomplete."""
    frames = []
    offset = 0
    while len(buffer) - offset >= 4:
        (length,) = struct.unpack_from("<H", buffer, offset)
        if length < 4 or len(buffer) - offset < length:
            break
        frames.append(buffer[offset:offset + length])
        offset += length
    return frames, buffer[offset:]


class WireLog:
    """Appends one line per relayed game frame, without changing a byte.

    The exe reaches Memoryer through this tap in a wired run. S2C 2005
    is the room-created event that repaints the client's room list, so
    each such line carries an evidence ID for the row-paint window.
    """

    def __init__(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        self.handle = open(path, "w", buffering=1, encoding="utf-8")

    def frame(self, direction: str, data: bytes) -> None:
        opcode, body = struct.unpack_from("<H", data, 2)[0], data[4:]
        detail = ""
        if direction == "S2C" and opcode == ROOM_CREATED:
            detail = f" ROOM_CREATED room={body[:4].hex()}"
        self.handle.write(
            f"{time.strftime('%H:%M:%S')} {direction} opcode={opcode} "
            f"({opcode:04X}) len={len(body)} hex={body[:WIRE_BODY_PREVIEW].hex()}"
            f"{detail}\n"
        )

    def connection(self, message: str) -> None:
        self.handle.write(f"{time.strftime('%H:%M:%S')} {message}\n")


class Log:
    """One timestamped line per event, echoed to stdout and host.log."""

    def __init__(self, path: Path) -> None:
        self.handle = open(path, "w", buffering=1, encoding="utf-8")

    def write(self, message: str) -> None:
        line = f"{time.strftime('%H:%M:%S')} {message}"
        print(line, flush=True)
        self.handle.write(line + "\n")


class Frames:
    """Length-prefixed frame reader that keeps unparsed bytes."""

    def __init__(self, sock: socket.socket) -> None:
        self.sock = sock
        self.buffer = b""

    def next_frame(self, timeout: float) -> tuple[int, bytes] | None:
        deadline = time.time() + timeout
        while time.time() < deadline:
            if len(self.buffer) >= 4:
                (length,) = struct.unpack("<H", self.buffer[:2])
                if len(self.buffer) >= length:
                    opcode = struct.unpack("<H", self.buffer[2:4])[0]
                    body = self.buffer[4:length]
                    self.buffer = self.buffer[length:]
                    return opcode, body
            try:
                chunk = self.sock.recv(4096)
            except socket.timeout:
                continue
            if not chunk:
                return None
            self.buffer += chunk
        return None


def read_token(db_path: Path, user_index_id: int) -> str:
    db = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    row = db.execute(
        "select TUSER_ID from t_o2jam_login where USER_INDEX_ID = ?",
        (user_index_id,),
    ).fetchone()
    if row is None:
        raise SystemExit(f"no login row for user_index_id={user_index_id} in {db_path}")
    return row[0]


def send(sock: socket.socket, opcode: int, body: bytes) -> None:
    sock.sendall(frame(opcode, body))


def await_answer(frames: Frames, wanted: int, log: Log, timeout: float = 20.0) -> bytes:
    while True:
        frame_or_none = frames.next_frame(timeout)
        if frame_or_none is None:
            raise SystemExit(f"timeout waiting for {wanted:04X}")
        opcode, body = frame_or_none
        if opcode != wanted:
            log.write(f"host event cmd={opcode:04X} len={len(body)} hex={body[:64].hex()}")
            continue
        return body


def authorise_and_enter_channel(sock, frames, log, token: str, user_index_id: int) -> None:
    local_ip, local_port = sock.getsockname()[:2]
    body = (
        b"\x01"
        + struct.pack("<ii", 0, 0)
        + encode_endpoint(local_ip, local_port)
        + encode_endpoint(local_ip, local_port)
        + struct.pack("<i", user_index_id)
        + encode_string(token)
        + encode_string("")
        + b"\x01"
        + encode_string(CLIENT_ID)
    )
    send(sock, AUTHORIZE, body)
    answer = await_answer(frames, AUTHORIZE_ANSWER, log)
    if struct.unpack("<i", answer[:4])[0] != 0:
        raise SystemExit(f"authorize rejected: {answer.hex()}")
    log.write(f"host authorized user_index_id={user_index_id}")
    send(sock, GET_CHANNEL_LIST, b"")
    await_answer(frames, CHANNEL_LIST, log)
    send(sock, CHANNEL_LOGIN, struct.pack("<hh", 0, 0))
    answer = await_answer(frames, CHANNEL_LOGIN_ANSWER, log)
    if struct.unpack("<i", answer[:4])[0] != 0:
        raise SystemExit(f"channel login rejected: {answer.hex()}")
    log.write("host in channel 0/0")


def create_room(sock, frames, log, title: str, music_id: int) -> None:
    body = (
        encode_string(title, 22)
        + bytes([0, 0, 0])
        + encode_string("", 12)
        + bytes([1, 99])
        + struct.pack("<h", 0)
    )
    send(sock, CREATE_ROOM, body)
    answer = await_answer(frames, CREATE_ROOM_ANSWER, log)
    result = struct.unpack("<i", answer[:4])[0]
    number = struct.unpack("<i", answer[4:8])[0] if len(answer) >= 8 else -1
    if result != 0:
        raise SystemExit(f"create room rejected: result={result} body={answer.hex()}")
    log.write(f"host room created number={number} title={title}")
    time.sleep(1)
    send(sock, SET_ROOM_MUSIC, struct.pack("<HBB", music_id, 2, 1))
    log.write(f"host music set {music_id}/2/1")
    time.sleep(0.5)
    send(sock, SET_ROOM_ARENA, struct.pack("<i", 0))
    log.write("host arena 0 (flaps stay off: an in-room arena change trips the Wine DXSurface error)")


def send_start(sock, log) -> None:
    send(sock, SYNC_MEMBER_MUSIC_STATE, bytes([0]))
    send(sock, SET_DOWNLOAD_PROGRESS, bytes([100, 0]))
    send(sock, START_GAME, b"")
    send(sock, CONFIRM_MUSIC_LOADED, struct.pack("<iB", 0, ROOM_SPEED))
    log.write("host start sent 4010+4012")


def take_commands(cmd_file: Path) -> list[str]:
    """Return the tokens written since the last poll, then clear the file."""
    try:
        text = cmd_file.read_text(encoding="utf-8")
    except OSError:
        return []
    if not text.strip():
        return []
    cmd_file.write_text("", encoding="utf-8")
    return text.split()


class HostSession:
    """The socket plus the room it hosts, so `recreate` can rebuild both."""

    def __init__(self, log: Log, user_index_id: int, token: str, title: str, music_id: int):
        self.log = log
        self.user_index_id = user_index_id
        self.token = token
        self.title = title
        self.music_id = music_id
        self.sock = socket.create_connection(GATEWAY, timeout=10)
        self.sock.settimeout(0.5)
        self.frames = Frames(self.sock)

    def open(self) -> None:
        authorise_and_enter_channel(
            self.sock, self.frames, self.log, self.token, self.user_index_id
        )
        create_room(self.sock, self.frames, self.log, self.title, self.music_id)

    def recreate(self) -> None:
        self.sock.close()
        time.sleep(2)
        self.sock = socket.create_connection(GATEWAY, timeout=10)
        self.sock.settimeout(0.5)
        self.frames = Frames(self.sock)
        self.open()
        self.log.write("host recreate done (fuse suppression)")


def relay_wire(source: socket.socket, target: socket.socket, direction: str,
               wire_log: WireLog) -> None:
    """Forward frames unchanged while recording them; never rewrite."""
    pending = b""
    try:
        while True:
            chunk = source.recv(65536)
            if not chunk:
                return
            pending += chunk
            frames, pending = split_wire_frames(pending)
            for data in frames:
                wire_log.frame(direction, data)
            target.sendall(b"".join(frames))
    except OSError as error:
        wire_log.connection(f"relay {direction} end: {error}")
    finally:
        try:
            target.shutdown(socket.SHUT_WR)
        except OSError:
            pass


def serve_wire_tap(listen_port: int, upstream_port: int, wire_log: WireLog) -> None:
    """Accept exe connections and log both directions in frame order."""
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", listen_port))
    server.listen(4)
    wire_log.connection(
        f"wire-tap listening 127.0.0.1:{listen_port} "
        f"-> 127.0.0.1:{upstream_port} (pass-through)"
    )
    while True:
        client, _ = server.accept()
        upstream = socket.create_connection(("127.0.0.1", upstream_port), timeout=10)
        wire_log.connection(f"wire-tap client {client.getpeername()} connected")
        threads = [
            threading.Thread(target=relay_wire, args=(client, upstream, "C2S", wire_log)),
            threading.Thread(target=relay_wire, args=(upstream, client, "S2C", wire_log)),
        ]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        client.close()
        upstream.close()
        wire_log.connection("wire-tap client disconnected")


def hold_session(session: HostSession, log: Log, start_delay: float, hold: float,
                 stop_file: Path, cmd_file: Path) -> None:
    joined_at: float | None = None
    started = False
    deadline = time.time() + hold
    while time.time() < deadline:
        if stop_file.is_file():
            log.write("host stop file seen")
            return
        for token in take_commands(cmd_file):
            if token == "recreate" and joined_at is None:
                session.recreate()
            elif token == "start" and not started:
                send_start(session.sock, log)
                started = True
            else:
                log.write(f"host command ignored token={token} joined={joined_at is not None} started={started}")
        event = session.frames.next_frame(0.5)
        if event is None:
            send(session.sock, PING, b"")
        else:
            opcode, body = event
            if opcode == JOIN_WAITING_EVENT:
                joined_at = time.time()
                log.write("host guest JOINED")
            elif opcode != PING:
                log.write(f"host opcode {opcode:04X} len={len(body)} hex={body[:64].hex()}")
        if joined_at is not None and not started and time.time() - joined_at >= start_delay:
            send_start(session.sock, log)
            started = True


def main() -> None:
    parser = argparse.ArgumentParser(prog="wine-room-host")
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--user-index-id", type=int, default=3)
    parser.add_argument("--db", default=str(Path.home() / "memoryer-local" / "O2JAM.db"))
    parser.add_argument("--title", default="WINEJOIN")
    parser.add_argument("--music-id", type=int, default=757)
    parser.add_argument("--start-delay", type=float, default=20.0)
    parser.add_argument("--hold", type=float, default=300.0)
    parser.add_argument("--stop-file", default="")
    parser.add_argument("--cmd-file", default="")
    parser.add_argument("--wire-tap", action="store_true",
                        help="log the exe's game-TCP frames without changing them")
    parser.add_argument("--listen-port", type=int, default=15012)
    parser.add_argument("--upstream-port", type=int, default=15010)
    parser.add_argument("--wire-log", default="")
    arguments = parser.parse_args()

    out_dir = Path(arguments.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    if arguments.wire_tap:
        wire_log_path = Path(arguments.wire_log) if arguments.wire_log else out_dir / "wire" / "room-stage.log"
        serve_wire_tap(arguments.listen_port, arguments.upstream_port,
                       WireLog(wire_log_path))
        return
    log = Log(out_dir / "host.log")
    stop_file = Path(arguments.stop_file) if arguments.stop_file else out_dir / "host.stop"
    cmd_file = Path(arguments.cmd_file) if arguments.cmd_file else out_dir / "host.cmd"
    cmd_file.write_text("", encoding="utf-8")

    token = read_token(Path(arguments.db), arguments.user_index_id)
    session = HostSession(
        log, arguments.user_index_id, token, arguments.title, arguments.music_id
    )
    try:
        session.open()
        hold_session(
            session, log, arguments.start_delay, arguments.hold, stop_file, cmd_file
        )
    finally:
        session.sock.close()
        log.write("host done")


if __name__ == "__main__":
    main()
