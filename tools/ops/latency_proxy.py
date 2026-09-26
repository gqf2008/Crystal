"""可注入延迟/丢包的 TCP 代理（故障注入用，纯标准库）。

用法：python latency_proxy.py --listen 7200 --target 127.0.0.1:7100 --delay-ms 200 --drop-pct 5
说明：单向延迟（服务端→客户端）在 delay-ms 基础上叠加；drop-pct 按包（TCP 段）概率丢弃。
用途：验证"网络抖动/丢包时服务端会话是否仍可用、是否产生错误日志"。
注意：这是**测试工具**，不追求吞吐（每连接两个线程 + 队列），够注入扰动即可。

⚠️ **`--drop-pct` 不模拟真实丢包，会破坏 TCP 语义（2026-09-26 实测定性，别再据此下"产品不容忍丢包"结论）**：
本代理在**字节流中间**直接丢数据且**不做重传** —— 而这正是 TCP 层本就保证不做的事（真实丢包由协议栈重传，
上层永远看不到"半个帧"）。后果是下游会收到**被拼接坏的帧**：实测客户端在 `200ms/5%` 下反复报
`🔌 帧解码错误: frame length 43690 exceeds max 32768`（43690=0xAAAA 是拼接产物）并因此停住。
所以：
  - 想测**延迟**：用 `--drop-pct 0`（实测客户端在纯延迟 200/400ms 下进图用时 11.6/11.4s，与直连 12.0s 无差）；
  - 想测**丢包/断流**：不要用本开关；应模拟"连接被重置/断开"（如按连接丢、或临时阻断后再放行），
    或直接在协议层注入重传/超时模型 —— 否则你测到的是"流被破坏"，不是"网络丢包"。
"""
import argparse
import random
import socket
import threading
import time


def pump(src: socket.socket, dst: socket.socket, delay_ms: float, drop_pct: float) -> None:
    try:
        while True:
            data = src.recv(8192)
            if not data:
                break
            if delay_ms > 0:
                time.sleep(delay_ms / 1000.0)
            if drop_pct > 0 and random.random() * 100.0 < drop_pct:
                continue          # 模拟丢包（TCP 层会重传，表现为延迟抖动/吞吐下降）
            dst.sendall(data)
    except OSError:
        pass
    finally:
        for s in (src, dst):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
            try:
                s.close()
            except OSError:
                pass


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--listen", type=int, default=7200)
    ap.add_argument("--target", default="127.0.0.1:7100")
    ap.add_argument("--delay-ms", type=float, default=200.0)
    ap.add_argument("--drop-pct", type=float, default=5.0)
    a = ap.parse_args()
    host, port = a.target.rsplit(":", 1)
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", a.listen))
    srv.listen(64)
    print(f"proxy 127.0.0.1:{a.listen} -> {a.target} delay={a.delay_ms}ms drop={a.drop_pct}%", flush=True)
    while True:
        cs, _ = srv.accept()
        us = socket.create_connection((host, int(port)))
        threading.Thread(target=pump, args=(cs, us, a.delay_ms, a.drop_pct), daemon=True).start()
        threading.Thread(target=pump, args=(us, cs, 0.0, a.drop_pct), daemon=True).start()


if __name__ == "__main__":
    raise SystemExit(main())
