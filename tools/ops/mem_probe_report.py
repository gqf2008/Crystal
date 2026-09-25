"""把 `MEM_PROBE*` 日志读成「每轮增量 + 泄漏指纹」——只读，不起服务端、不写任何东西。

背景（2026-09-25）：计数分配器（`--features mem-probe` + `MIR2_LEAK_PROBE=1`）会在每轮
「全员登出 + 整图清理」之后打印
  `MEM_PROBE_IDLE map=… live_bytes=… allocs=… deallocs=… live_le256b=… live_le16k=… live_gt16k=… live_player_actors=…`
  以及紧随其后的 8 行 `MEM_PROBE_DELTA size=… live_bytes=… count=… dbytes=… dcount=…`（**相对上一轮的变化**）。
本脚本把这条流变成两个可直接用的结论：
  ① **每轮增量表**（live_bytes 与上一轮 idle 的差、以及本轮增长最大的几个尺寸）；
  ② **泄漏指纹**——在「测量轮」里**每一轮 dbytes 都为正**的那些尺寸。
     预热轮（平台/高水位台阶）里的增长不算指纹：泄漏的特征是「每轮都在同一个尺寸上加同样量级」，
     而预热尾巴是「前几轮涨、之后停」。
修复前/后各跑一次同一夹具，指纹应从一个非空集合变成空集——这就是修好了的可判定证据。

用法：
    py -3.12 tools/ops/mem_probe_report.py                     # 默认读 tools/ops/out/leak_plateau.log
    py -3.12 tools/ops/mem_probe_report.py <日志路径> --skip 3  # 前 3 个 idle 点算预热，不计入指纹
退出码：0 读到样本 / 2 前置失败（没找到任何 MEM_PROBE_IDLE 行）。
"""
import argparse
import os
import re
import sys


def find_default_log() -> str:
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(here, "out", "leak_plateau.log")


KV = re.compile(r"(\w+)=(-?\d+)")


def parse(text: str):
    """→ [{"idle": kvs, "deltas": [...], "net": kvs|None, "cum": [...]}, ...]（按 idle 点分组）"""
    points = []
    cur = None
    for line in text.splitlines():
        i = line.find("MEM_PROBE_IDLE")
        if i >= 0:
            cur = {"idle": {k: int(v) for k, v in KV.findall(line[i:])}, "deltas": [], "net": None,
                   "cumnet": None, "cum": []}
            points.append(cur)
            continue
        # 注意顺序：`MEM_PROBE_CUMNET` 以 `MEM_PROBE_CUM` 为前缀，必须先匹配更长的那个，
        # 否则汇总行会被当成一条尺寸记录（实测：KeyError: 'size'）。
        for marker, key in (("MEM_PROBE_DELTA", "deltas"), ("MEM_PROBE_NET", "net"),
                            ("MEM_PROBE_CUMNET", "cumnet"), ("MEM_PROBE_CUM", "cum")):
            j = line.find(marker)
            if j < 0 or cur is None:
                continue
            kvs = {k: int(v) for k, v in KV.findall(line[j:])}
            if key == "deltas":
                cur["deltas"].append(kvs)
            elif key == "cum":
                cur["cum"].append(kvs)
            else:
                cur[key] = kvs
            break
    return points


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("log", nargs="?", default="")
    ap.add_argument("--skip", type=int, default=3, help="前 N 个 idle 点当预热，不计入泄漏指纹（默认 3）")
    ap.add_argument("--top", type=int, default=5, help="每轮列出的增长尺寸条数（默认 5）")
    a = ap.parse_args()

    path = a.log or find_default_log()
    if not os.path.exists(path):
        print(f"FAIL(前置)：日志不存在：{path}")
        return 2
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        text = fh.read()
    points = parse(text)
    if not points:
        print(f"FAIL(前置)：{path} 里没有任何 MEM_PROBE_IDLE 行（探针没开？二进制不是 --features mem-probe？）")
        return 2

    print(f"日志：{path}")
    print(f"idle 采样点：{len(points)} 个（前 {a.skip} 个当预热）")
    print()
    print("序号  登出后live_bytes     环比Δ     本轮正/负增量          增长最大的尺寸(size:dbytes×dcount)")
    prev = None
    for idx, pt in enumerate(points):
        live = pt["idle"].get("live_bytes", 0)
        delta = "" if prev is None else f"{live - prev:+,}"
        net = pt["net"] or {}
        netcell = f"{net.get('dpos', 0):+,}/{net.get('dneg', 0):+,}" if net else "-"
        growing = sorted([d for d in pt["deltas"] if d.get("dbytes", 0) > 0], key=lambda d: -d["dbytes"])[: a.top]
        cell = "  ".join(f"{d['size']}:{d['dbytes']:+}×{d['dcount']:+}" for d in growing) or "（无增长尺寸）"
        marker = "预热" if idx < a.skip else "测量"
        print(f"{idx:>3}   {live:>16,}  {delta:>9}  {netcell:>20}  [{marker}] {cell}")
        prev = live
    print()

    cum = [pt["cum"] for pt in points]
    if not any(cum):
        print("（这份日志没有 MEM_PROBE_CUM 行——探针是旧版二进制，或本次没跑到第二轮以后。）")
        return 0

    lastnet = points[-1].get("cumnet") or {}
    live_first = points[0]["idle"].get("live_bytes", 0)
    live_last = points[-1]["idle"].get("live_bytes", 0)
    print(f"总量对照：live_bytes {live_first:,} → {live_last:,}（{live_last - live_first:+,} B）；"
          f"精确尺寸（≤{65536 // 1024}KB）累计 {lastnet.get('cum_pos', 0):+,}/{lastnet.get('cum_neg', 0):+,}")
    print(f"          ⇒ 差额 {live_last - live_first - lastnet.get('cum_net', 0):+,} B 落在 **>64KB 的分配**上"
          f"（精确直方图只覆盖 ≤64KB，故这部分只能由三档分桶的 live_gt16k 佐证）")
    print()

    last = cum[-1]
    if last:
        print(f"**累计增长榜**（相对第一个 idle 点，第 {len(points) - 1} 个采样点）")
        for d in sorted(last, key=lambda d: -d.get("cum_dbytes", 0)):
            print(f"  size={d['size']:>6}  累计 {d.get('cum_dbytes', 0):+,} B  现存 {d.get('live_bytes', 0):,} B"
                  f" × {d.get('count', 0)} 个（+{d.get('cum_dcount', 0)} 个）")
        print()

    measured = [set(d["size"] for d in c) for c in cum[a.skip:] if c]
    if len(measured) < 2:
        print("（测量轮不足 2 轮，跳过指纹交集。）")
        return 0
    common = set.intersection(*measured)
    if common:
        print("**泄漏指纹**：以下尺寸在**每个测量轮**的累计榜里都出现（说明一直在涨）")
        for size in sorted(common, key=lambda s: -max(d.get("cum_dbytes", 0) for d in last if d["size"] == s)):
            row = next(d for d in last if d["size"] == size)
            print(f"  size={size:>6}  末轮累计 {row.get('cum_dbytes', 0):+,} B  现存 {row.get('live_bytes', 0):,} B"
                  f" × {row.get('count', 0)} 个")
        print()
        print("→ 下一步：拿这些 size 去比对结构体/缓冲区的字节数（同尺寸每轮都涨 = 有东西被保留）。")
    else:
        print("**泄漏指纹为空**：没有任何尺寸在每个测量轮的累计榜上都出现——与「预热台阶/高水位」一致。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
