"""无渲染压测/冒烟机器人：说服务端的线协议（纯标准库）。

为什么自己实现而不用客户端二进制：Bevy 客户端每个进程要渲染窗口 + 几百 MB 内存，
单机跑不了几十并发；而"上线要扛多少并发"问的是**服务端**在 N 条会话下的表现，
所以这里只做协议级会话（握手 → 登录 → 进图 → 保持心跳）。

线协议（两层，别只看内层）：
  外层（ServerRust/src/gate/codec.rs）：[length u16 LE][XOR 0xAA 的 payload]
  内层（SharedRust/src/packets/base.rs）：[length u16 LE][opcode i16 LE][body]，length = 4 + body 长度
  字符串 = DotNet 7-bit 长度前缀 + utf8（SharedRust/src/binary.rs；**不是** i32 前缀）
  opcode（ServerRust/src/enums.rs::ClientPacketIds）：ClientVersion=0 KeepAlive=2 Login=5 StartGame=8 LogOut=9

用法：
  python bot.py --host 127.0.0.1 --port 7000 --sessions 5 --hold 20 --account-prefix smoke
  python bot.py --login-only --sessions 30        # 只压 gate/账号路径，不进图
输出：JSON 到 stdout（每会话结果 + 汇总），供 ops 脚本当判据。
"""
import argparse
import json
import socket
import struct
import threading
import time

OP_CLIENT_VERSION = 0
OP_KEEPALIVE = 2
OP_NEW_ACCOUNT = 3
OP_LOGIN = 5
OP_NEW_CHARACTER = 6
OP_NEW_CHARACTER_SUCCESS = 11   # ServerPacketIds::NewCharacterSuccess
OP_STARTGAME = 8
OP_LOGOUT = 9
# 注意方向：**收到**的聊天是 ServerPacketIds::Chat = 30；ClientPacketIds::Chat = 13 是我们发出去的，
# 别拿 13 去过滤收包（2026-09-24 在存储降级演练里踩过：过滤了 13，78 条 opcode=30 全被漏掉）。
OP_CHAT = 30          # ServerPacketIds::Chat（SharedRust/src/enums.rs）
CHAT_TYPE_SYSTEM = 5  # ChatType::System
# 移动类（ClientPacketIds，SharedRust/src/enums.rs）：Turn=10 Walk=11 Run=12；body 只有 1 字节方向
OP_TURN = 10
OP_WALK = 11
OP_RUN = 12
XOR_KEY = 0xAA  # gate/codec.rs::DEFAULT_XOR_KEY


def frame(opcode: int, body: bytes) -> bytes:
    """内层包 + 外层 [u16 长度][XOR 0xAA] 封装。"""
    inner = struct.pack("<Hh", 4 + len(body), opcode) + body
    return struct.pack("<H", len(inner)) + bytes(b ^ XOR_KEY for b in inner)


def dotnet_string(s: str) -> bytes:
    """DotNet 7-bit 长度前缀（BinaryWriter.Write(string)）。"""
    b = s.encode("utf-8")
    v = len(b)
    out = bytearray()
    while True:
        chunk = v & 0x7F
        v >>= 7
        out.append(chunk | 0x80 if v else chunk)
        if not v:
            break
    return bytes(out) + b


def read_dotnet_string(buf: bytes, pos: int) -> tuple[str, int]:
    """读 DotNet 7-bit 长度前缀字符串，返回 (文本, 新位置)。"""
    shift = 0
    length = 0
    while True:
        b = buf[pos]
        pos += 1
        length |= (b & 0x7F) << shift
        if not (b & 0x80):
            break
        shift += 7
    text = buf[pos:pos + length].decode("utf-8", errors="replace")
    return text, pos + length


def recv_exact(sock: socket.socket, n: int) -> bytes:
    buf = b""
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise ConnectionError("连接被对端关闭")
        buf += chunk
    return buf


def recv_frame(sock: socket.socket) -> tuple[int, bytes]:
    outer_len = struct.unpack("<H", recv_exact(sock, 2))[0]
    payload = bytes(b ^ XOR_KEY for b in recv_exact(sock, outer_len))
    length, opcode = struct.unpack("<Hh", payload[:4])
    body = payload[4:length] if length > 4 else b""
    return opcode, body


def drain(sock: socket.socket, stop: threading.Event, out: dict) -> None:
    """后台读线程：记录帧数/字节数 + **捕获 S.Chat 的系统消息**（`ChatType::System`），直到 stop。

    为什么要捕获 Chat：owner 2026-09-24 拍板「落库失败一律直接反馈到客户端」，
    服务端用 `S.Chat` + `ChatType::System(=5)` 发 `存档失败：…`（`db::persist_failure_notice`）。
    存储降级演练要证明**玩家真的看得到**，不能只看服务端日志。
    """
    frames = 0
    bytes_read = 0
    idle = 0
    system_messages: list = []
    out["system_messages"] = system_messages
    opcodes: dict = {}
    out["opcodes"] = opcodes
    try:
        while not stop.is_set():
            try:
                opcode, body = recv_frame(sock)
            except socket.timeout:
                # 空闲不是错误：服务端只在有事件时推送（进图时几百 KB，之后可能长时间静默）。
                # 早期版本把空闲当 read_error → 保持型会话全被判失败（假红）。
                idle += 1
                continue
            frames += 1
            bytes_read += len(body) + 4
            opcodes[str(opcode)] = opcodes.get(str(opcode), 0) + 1
            # ServerPacketIds::Chat = 13；body = [dotnet string][chat_type u8]
            if opcode == OP_CHAT and body:
                try:
                    text, pos = read_dotnet_string(body, 0)
                    chat_type = body[pos] if pos < len(body) else -1
                    if chat_type == CHAT_TYPE_SYSTEM:
                        system_messages.append(text)
                except Exception:
                    pass
    except Exception as exc:  # 断开/超时都记下来，供错误率统计
        # 自己关掉 socket 造成的 10038 不是错误（主线程 join 不到位时的竞态）。
        # 这类噪声会把"已经跑得好好的会话"判成失败（实测 20 会话里 9 条假失败）。
        text = f"{type(exc).__name__}: {exc}"
        if "10038" not in text:
            out["read_error"] = text
        else:
            out["read_closed_by_us"] = True
    out["frames"] = frames
    out["bytes"] = bytes_read
    out["idle_waits"] = idle


def one_session(idx: int, host: str, port: int, account: str, password: str,
                login_only: bool, hold_sec: float, timeout: float,
                self_provision: bool = False, char_name: str = '',
                activity: str = 'none', step_ms: int = 600) -> dict:
    t0 = time.time()
    res = {"idx": idx, "account": account, "ok": False, "stage": "connect",
           "frames": 0, "bytes": 0}
    sock = socket.create_connection((host, port), timeout=timeout)
    sock.settimeout(timeout)
    res["t_connected"] = round(time.time() - t0, 3)
    try:
        opcode, _ = recv_frame(sock)              # Connected
        res["t_connected_frame"] = round(time.time() - t0, 3)
        res["stage"] = "client_version"
        sock.sendall(frame(OP_CLIENT_VERSION, struct.pack("<i", 16) + b"\x00" * 16))
        deadline = time.time() + timeout
        while time.time() < deadline:             # 等 ClientVersion 回执
            opcode, body = recv_frame(sock)
            if opcode == 0x0000 or body:           # 服务端用 opcode=0 回 result=1
                break
        res["t_version_reply"] = round(time.time() - t0, 3)

        if self_provision:
            # 「新玩家」全路径：注册账号 → 登录 → 建角色 → 进图。
            # 用途：容量标定需要**大量独立会话**（同账号重复登录会互踢，不能拿来压世界路径）。
            res["stage"] = "new_account"
            body = (dotnet_string(account) + dotnet_string(password)
                    + struct.pack("<q", 0) + dotnet_string("ops bot")
                    + dotnet_string("q") + dotnet_string("a") + dotnet_string("ops@example.invalid"))
            sock.sendall(frame(OP_NEW_ACCOUNT, body))
            opcode, ack = recv_frame(sock)
            res["new_account_result"] = ack[0] if ack else -1   # 0 = 成功（C# Result 0-8）
            res["stage"] = "login"
            sock.sendall(frame(OP_LOGIN, dotnet_string(account) + dotnet_string(password)))
            res["login_sent"] = round(time.time() - t0, 3)
            opcode, body = recv_frame(sock)                     # LoginSuccess（characters）
            res["stage"] = "new_character"
            sock.sendall(frame(OP_NEW_CHARACTER,
                               dotnet_string(char_name) + bytes([0, 0])))
            opcode, ack = recv_frame(sock)
            # 建角**成功**时服务端回的是 `S.NewCharacterSuccess`（opcode 11），body 是
            # dotnet 字符串（角色名）+ index + level + class + gender + last_access。
            # 旧代码不看 opcode、直接读 `ack[0]` 当 Result —— 那其实是**名字长度前缀**
            # （实测 "OpsBot0" ⇒ 7），于是把「建角成功」判成失败，fresh-deploy 冒烟报假红。
            res["new_character_frame"] = int(opcode)
            if opcode == OP_NEW_CHARACTER_SUCCESS:
                res["new_character_result"] = 0            # 成功（与旧口径兼容）
                res["new_character_name_len"] = ack[0] if ack else -1
            else:
                # 兼容「回 S.NewCharacter{Result}」的实现（被限流/被拒时走这条）
                res["new_character_result"] = ack[0] if ack else -1
            res["stage"] = "start_game"
            sock.sendall(frame(OP_STARTGAME, struct.pack("<i", 0)))
            res["start_sent"] = round(time.time() - t0, 3)
            stop = threading.Event()
            reader = threading.Thread(target=drain, args=(sock, stop, res), daemon=True)
            reader.start()
            for _ in range(max(1, int(hold_sec / 5))):
                time.sleep(min(5, hold_sec))
                if stop.is_set():
                    break
                try:
                    sock.sendall(frame(OP_KEEPALIVE, b""))
                except Exception:
                    break
            res["held_sec"] = round(time.time() - t0, 2)
            stop.set()
            # 先缩短超时再 join：drain 还阻塞在 recv 时主线程 close 会撞出 WinError 10038（假失败）
            try:
                sock.settimeout(0.3)
            except Exception:
                pass
            reader.join(timeout=3)
            try:
                sock.sendall(frame(OP_LOGOUT, b""))
            except Exception:
                pass
            ok_stages = (res.get("new_account_result") == 8 and res.get("new_character_result") in (0, 8)
                         and res.get("read_error") is None)
            res["ok"] = bool(ok_stages and res["frames"] > 0)
            res["stage"] = "done"
            return res

        res["stage"] = "login"
        sock.sendall(frame(OP_LOGIN, dotnet_string(account) + dotnet_string(password)))
        res["login_sent"] = round(time.time() - t0, 3)

        if login_only:
            # 等 LoginSuccess（读到任意一帧即认为账号路径通了），随后主动登出
            opcode, body = recv_frame(sock)
            res["login_reply_bytes"] = len(body) + 4
            # 真延迟：从发起连接算到**收到登录回执**。此前只记 login_sent（包交给 OS 的时刻），
            # 那是"服务端读得多快"的代理量，不是登录延迟——据此报 p95 会误导（实测两者能差 10 倍）。
            res["login_reply_sec"] = round(time.time() - t0, 3)
            res["ok"] = True
            res["stage"] = "login_only_done"
            # `--hold` 期间**保持已登录会话**再登出：此前这里立即 LOGOUT，导致
            # `wall_sec≈0.08s`、`frames=0` —— 容量标定想测的"登录后稳态 per-session 成本"
            # 根本测不到（拿到的 0.09MB/会话是瞬态；`mem_attribution.ps1` 的三档对比因此缺了基线）。
            # 只影响 `--login-only`（其它路径本来就会在 hold 后返回）；登录延迟字段不受影响。
            remaining = hold_sec - (time.time() - t0)
            if remaining > 0:
                time.sleep(remaining)
            sock.sendall(frame(OP_LOGOUT, b""))
            return res

        # 等角色列表 → StartGame(index=0)
        opcode, body = recv_frame(sock)
        res["stage"] = "start_game"
        sock.sendall(frame(OP_STARTGAME, struct.pack("<i", 0)))
        res["start_sent"] = round(time.time() - t0, 3)

        stop = threading.Event()
        reader = threading.Thread(target=drain, args=(sock, stop, res), daemon=True)
        reader.start()
        # 进图后保持会话：定期发 KeepAlive（服务端按心跳判活），并按 `--activity` 走位/奔跑。
        #
        # 为什么要"活动负载"：纯保持连接几乎不产生**视野扇出**——真玩家每成功走一步都会
        # `send_user_location` + 向同图其它会话广播 `ObjectWalk`，20 个同图玩家走动时的扇出
        # 是"零活动"压不出来的（CAPACITY.md §6.1/§6.2 要判的就是这条路径）。
        #
        # 步频：默认 600ms = C# `HumanObject` MoveDelay（真玩家走速）；服务端下限
        # `MIN_MOVE_INTERVAL_MS=50`（`ServerRust/src/actors/world/session.rs:1727`），
        # 低于它会被判 `Speed hack detected` 并拒绝移动，所以 `--step-ms` 不该小于 60。
        dirs = [2, 4, 6, 0]  # 右→下→左→上（MirDirection：Up=0 UpRight=1 Right=2 DownRight=3 Down=4 DownLeft=5 Left=6 UpLeft=7）
        steps = 0
        next_move = time.time()
        last_keepalive = time.time()
        deadline = time.time() + hold_sec
        while time.time() < deadline:
            if stop.is_set():
                break
            now = time.time()
            if activity != 'none' and now >= next_move:
                op = OP_RUN if activity == 'run' else OP_WALK
                try:
                    sock.sendall(frame(op, bytes([dirs[steps % len(dirs)]])))
                    steps += 1
                except Exception:
                    break
                next_move = now + max(0.02, step_ms / 1000.0)
                continue
            if now - last_keepalive >= 5:
                try:
                    sock.sendall(frame(OP_KEEPALIVE, b""))
                except Exception:
                    break
                last_keepalive = now
            time.sleep(0.02)
        res["steps_sent"] = steps
        res["held_sec"] = round(time.time() - t0, 2)
        stop.set()
        # 先缩短超时再 join：drain 还阻塞在 recv 时主线程 close 会撞出 WinError 10038（假失败）
        try:
            sock.settimeout(0.3)
        except Exception:
            pass
        reader.join(timeout=3)
        try:
            sock.sendall(frame(OP_LOGOUT, b""))
        except Exception:
            pass
        # ok 的判据不要只看 drain 线程写的 frames：重载下 reader.join 可能超时，
        # 主线程先跑到这里时 frames 还没落（实测 30 会话里 12 条"跑得好好的会话"被判失败）。
        # 改成「无真错误 且 (收到了数据 或 活满了预定保持时长)」。
        survived = res["held_sec"] >= max(1.0, hold_sec * 0.8)
        res["ok"] = res.get("read_error") is None and (res.get("frames", 0) > 0 or survived)
        res["stage"] = "done"
        return res
    except Exception as exc:
        res["error"] = f"{type(exc).__name__}: {exc}"
        return res
    finally:
        try:
            sock.close()
        except Exception:
            pass


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=7000)
    ap.add_argument("--sessions", type=int, default=5)
    ap.add_argument("--hold", type=float, default=10.0, help="进图后保持秒数")
    ap.add_argument("--timeout", type=float, default=10.0)
    ap.add_argument("--login-only", action="store_true")
    ap.add_argument("--self-provision", action="store_true",
                    help="注册新账号+建角色+进图（容量标定要独立会话，不能复用同一账号）")
    ap.add_argument("--char-prefix", default="OpsBot")
    ap.add_argument("--accounts", default="", help="逗号分隔账号（默认用下面 prefix+序号）")
    ap.add_argument("--account-prefix", default="smoke")
    ap.add_argument("--password", default="123456")
    ap.add_argument("--activity", choices=["none", "walk", "run"], default="none",
                    help="进图后的活动负载：none=只保持连接；walk/run=按 --step-ms 走位/奔跑"
                         "（真玩家每步都会触发 UserLocation + 同图视野广播，是纯保持压不出的扇出路径）")
    ap.add_argument("--step-ms", type=int, default=600,
                    help="走位间隔毫秒（默认 600 = C# HumanObject MoveDelay；服务端下限 50ms，低于它判 speed hack）")
    a = ap.parse_args()

    accounts = [x for x in a.accounts.split(",") if x] or \
        [f"{a.account_prefix}{i}" for i in range(a.sessions)]

    t0 = time.time()
    results: list[dict] = [None] * len(accounts)  # type: ignore[list-item]

    def run(i: int) -> None:
        results[i] = one_session(i, a.host, a.port, accounts[i], a.password,
                                 a.login_only, a.hold, a.timeout,
                                 a.self_provision, f"{a.char_prefix}{i}",
                                 a.activity, a.step_ms)

    threads = [threading.Thread(target=run, args=(i,)) for i in range(len(accounts))]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    ok = [r for r in results if r and r.get("ok")]
    lat = sorted(r.get("login_reply_sec", r["login_sent"]) for r in results
                 if r and "login_sent" in r)
    summary = {
        "sessions": len(accounts),
        "ok": len(ok),
        "failed": len(accounts) - len(ok),
        "wall_sec": round(time.time() - t0, 2),
        "login_p50_sec": round(lat[len(lat) // 2], 3) if lat else None,
        "login_p95_sec": round(lat[min(len(lat) - 1, int(len(lat) * 0.95))], 3) if lat else None,
        "login_only": a.login_only,
        "self_provision": a.self_provision,
        "hold_sec": a.hold,
        "activity": a.activity,
        "step_ms": a.step_ms,
        "steps_sent": sum(r.get("steps_sent", 0) for r in results if r),
        "frames_recv": sum(r.get("frames", 0) for r in results if r),
        "bytes_recv": sum(r.get("bytes", 0) for r in results if r),
    }
    print(json.dumps({"summary": summary, "sessions": results}, ensure_ascii=False))
    return 0 if len(ok) == len(accounts) else 1


if __name__ == "__main__":
    raise SystemExit(main())
