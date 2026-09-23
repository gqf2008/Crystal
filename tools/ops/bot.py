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
OP_LOGIN = 5
OP_STARTGAME = 8
OP_LOGOUT = 9
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
    """后台读线程：只记录帧数/字节数，直到 stop。"""
    frames = 0
    bytes_read = 0
    try:
        while not stop.is_set():
            opcode, body = recv_frame(sock)
            frames += 1
            bytes_read += len(body) + 4
    except Exception as exc:  # 断开/超时都记下来，供错误率统计
        out["read_error"] = f"{type(exc).__name__}: {exc}"
    out["frames"] = frames
    out["bytes"] = bytes_read


def one_session(idx: int, host: str, port: int, account: str, password: str,
                login_only: bool, hold_sec: float, timeout: float) -> dict:
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
        res["stage"] = "login"
        sock.sendall(frame(OP_LOGIN, dotnet_string(account) + dotnet_string(password)))
        res["login_sent"] = round(time.time() - t0, 3)

        if login_only:
            # 等 LoginSuccess（读到任意一帧即认为账号路径通了），随后主动登出
            opcode, body = recv_frame(sock)
            res["login_reply_bytes"] = len(body) + 4
            res["ok"] = True
            res["stage"] = "login_only_done"
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
        # 进图后保持会话：定期发 KeepAlive（服务端按心跳判活）
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
        reader.join(timeout=2)
        try:
            sock.sendall(frame(OP_LOGOUT, b""))
        except Exception:
            pass
        res["ok"] = res.get("read_error") is None and res["frames"] > 0
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
    ap.add_argument("--accounts", default="", help="逗号分隔账号（默认用下面 prefix+序号）")
    ap.add_argument("--account-prefix", default="smoke")
    ap.add_argument("--password", default="123456")
    a = ap.parse_args()

    accounts = [x for x in a.accounts.split(",") if x] or \
        [f"{a.account_prefix}{i}" for i in range(a.sessions)]

    t0 = time.time()
    results: list[dict] = [None] * len(accounts)  # type: ignore[list-item]

    def run(i: int) -> None:
        results[i] = one_session(i, a.host, a.port, accounts[i], a.password,
                                 a.login_only, a.hold, a.timeout)

    threads = [threading.Thread(target=run, args=(i,)) for i in range(len(accounts))]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    ok = [r for r in results if r and r.get("ok")]
    lat = sorted(r["login_sent"] for r in results if r and "login_sent" in r)
    summary = {
        "sessions": len(accounts),
        "ok": len(ok),
        "failed": len(accounts) - len(ok),
        "wall_sec": round(time.time() - t0, 2),
        "login_p50_sec": round(lat[len(lat) // 2], 3) if lat else None,
        "login_p95_sec": round(lat[min(len(lat) - 1, int(len(lat) * 0.95))], 3) if lat else None,
        "login_only": a.login_only,
        "hold_sec": a.hold,
    }
    print(json.dumps({"summary": summary, "sessions": results}, ensure_ascii=False))
    return 0 if len(ok) == len(accounts) else 1


if __name__ == "__main__":
    raise SystemExit(main())
