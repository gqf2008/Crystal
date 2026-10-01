# rust_origin_audit.py — 「本端窗原点用了**图头**而不是 C# `GetTrueSize`」的机械扫描
#
# 为什么单做一条：§3.2cl ③ 的全仓扫描只认 `center_origin(PANEL…)` 那种写法，
# **把数字直接写进公式**的原点它扫不到 —— 2026-10-01 §3.2cu 就是这样漏到第⑦批才发现两处：
#   商城 `((1024.0-696.0)/2.0, (768.0-476.0)/2.0)`（`Title[749]` 真 694x475 ⇒ 应 (165,146)，本端 164）
#   输入框 `(1024.0-288.0)/2.0`（`Prguse[660]` 真 286x156 ⇒ 应 369，本端 368）
#
# 判据（启发式，输出必须人工过一遍）：
#   1. 扫 `Client-Bevy/src/**/*.rs`，取每个文件里出现的 `(LibraryName::X, idx)` 当**候选美术**；
#   2. 找「原点表达式」：`1024.0 - <数字或本文件常量>`、`768.0 - …`、`(1024.0 - …)/2.0`、
#      `center_origin(<…>, <…>)` —— 符号若是本文件 `const NAME: f32 = N`，解析成 N；
#   3. 把解析出的数与候选美术比：**该数 == 某美术的图头宽/高 而真尺寸≠图头** ⇒ 报一行
#      （这正是"拿图头当 `Size`"的特征；若用的是真尺寸，数就不会等于图头 ⇒ 不报）。
#   4. 已知**故意用图头/字面量**的窗放在 `rust_origin_audit_known.txt`（只提示，不算 FAIL）。
#
# 用法：py -3.12 rust_origin_audit.py --repo <Rust 仓库根> --data <含 *.Lib 的 Data> [--csv out.csv]
# **门禁语义**：新命中（不在 known 表内）>0 即 exit 1；`--selftest` 跑正/负对照。
import argparse
import csv
import io
import os
import re
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from libtruesize import frame, load, resolve_lib, true_size  # noqa: E402

ART_RE = re.compile(r"LibraryName::(\w+)\s*,\s*(\d+)")
CONST_RE = re.compile(r"(?:pub )?const (\w+)\s*:\s*f32\s*=\s*([\d.]+)\s*;")
# 常量也支持 `const NAME: (f32, f32) = (W, H);` 里的 `NAME.0` / `NAME.1`（§3.2cp 那批就是这么写的）
TUPLE_CONST_RE = re.compile(
    r"(?:pub )?const (\w+)\s*:\s*\(\s*f32\s*,\s*f32\s*\)\s*=\s*\(\s*([\d.]+)\s*,\s*([\d.]+)\s*\)\s*;"
)
NUM = r"(?:[A-Za-z_][A-Za-z0-9_]*(?:\.\d)?|\d+(?:\.\d+)?)"
CENTER_RE = re.compile(r"1024(?:\.0)?\s*-\s*(" + NUM + r")\s*\)\s*/\s*2(?:\.0)?")
CENTER_Y_RE = re.compile(r"768(?:\.0)?\s*-\s*(" + NUM + r")\s*\)\s*/\s*2(?:\.0)?")
ANCHOR_RE = re.compile(r"1024(?:\.0)?\s*-\s*(" + NUM + r")")
ANCHOR_Y_RE = re.compile(r"768(?:\.0)?\s*-\s*(" + NUM + r")")
CENTER_FN_RE = re.compile(r"center_origin\(\s*(" + NUM + r")\s*,\s*(" + NUM + r")\s*\)")


class Art:
    def __init__(self, lib, idx, header, true):
        self.lib, self.idx, self.header, self.true = lib, idx, header, true

    @property
    def swapped(self):
        return self.true != self.header


def art_size(data_dir, cache, lib, idx):
    key = (lib, idx)
    if key in cache:
        return cache[key]
    out = None
    try:
        path = resolve_lib(data_dir, lib)
        if path not in cache:
            cache[path] = load(path)
        data, _, offsets = cache[path]
        if 0 <= idx < len(offsets):
            w, h, bgra = frame(data, offsets[idx])
            (tw, th), _ = true_size(w, h, bgra)
            out = Art(lib, idx, (w, h), (tw, th))
    except SystemExit:
        out = None
    cache[key] = out
    return out


def resolve(sym, consts, tuples=None):
    # ⚠️ 顺序要紧：先试**数字**（`264.0` 也带点，不能先走 `NAME.0` 那条）
    try:
        return float(sym)
    except ValueError:
        pass
    if sym in consts:
        return consts[sym]
    if tuples and "." in sym:
        name, _, idx = sym.partition(".")
        if name in tuples and idx in ("0", "1"):
            return tuples[name][int(idx)]
    return None


def scan_file(path, data_dir, cache, base=None):
    src = io.open(path, encoding="utf-8", errors="ignore").read()
    if "\r\n" in src:
        src = src.replace("\r\n", "\n")
    # 只扫**生产代码**：`#[cfg(test)]` 之后的单测里会有"故意用图头模型"的对照断言（§3.2cl/§3.2co 都写过），
    # 那不是缺陷；同时跳过注释行与断言/打印行（它们只复述公式）。
    cut = src.find("#[cfg(test)]")
    if cut >= 0:
        src = src[:cut]
    consts = {m.group(1): float(m.group(2)) for m in CONST_RE.finditer(src)}
    tuples = {
        m.group(1): (float(m.group(2)), float(m.group(3)))
        for m in TUPLE_CONST_RE.finditer(src)
    }
    arts = []
    for m in ART_RE.finditer(src):
        a = art_size(data_dir, cache, m.group(1), int(m.group(2)))
        if a and a.swapped:
            arts.append(a)
    if not arts:
        return []

    hits = []
    lines = src.split("\n")
    for lineno, line in enumerate(lines, 1):
        stripped = line.strip()
        if stripped.startswith("//") or stripped.startswith("assert") or stripped.startswith("println"):
            continue
        checks = []
        for m in CENTER_FN_RE.finditer(line):
            checks.append((resolve(m.group(1), consts, tuples), "w"))
            checks.append((resolve(m.group(2), consts, tuples), "h"))
        for rx, axis in (
            (CENTER_RE, "w"),
            (CENTER_Y_RE, "h"),
            (ANCHOR_RE, "w"),
            (ANCHOR_Y_RE, "h"),
        ):
            for m in rx.finditer(line):
                v = resolve(m.group(1), consts, tuples)
                if v is not None:
                    checks.append((v, axis))
        for value, axis in checks:
            if value is None:
                continue
            for a in arts:
                # 只有**该轴真的被裁**才算"拿图头当 Size"（高没裁的美术不该因为宽被裁而误报）
                if a.true[0 if axis == "w" else 1] == a.header[0 if axis == "w" else 1]:
                    continue
                header = a.header[0] if axis == "w" else a.header[1]
                if abs(value - header) < 0.5:
                    hits.append(
                        {
                            "file": os.path.relpath(path, base).replace(os.sep, "/")
                            if base
                            else path.replace(os.sep, "/"),
                            "line": lineno,
                            "value": int(value),
                            "axis": axis,
                            "art": "%s[%d]" % (a.lib, a.idx),
                            "header": "%dx%d" % a.header,
                            "true": "%dx%d" % a.true,
                            "text": line.strip(),
                        }
                    )
    return hits


def load_known(path):
    known = set()
    if path and os.path.exists(path):
        for line in io.open(path, encoding="utf-8"):
            line = line.split("#", 1)[0].strip()
            if line:
                known.add(line)
    return known


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True)
    ap.add_argument("--data", required=True)
    ap.add_argument("--known", default="")
    ap.add_argument("--csv", default="")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args(argv)

    if a.selftest:
        return selftest(a)

    known = load_known(a.known)
    root = os.path.join(a.repo, "Client-Bevy", "src")
    cache = {}
    hits = []
    for dirpath, _dirs, files in os.walk(root):
        for fn in sorted(files):
            if fn.endswith(".rs"):
                hits.extend(scan_file(os.path.join(dirpath, fn), a.data, cache, a.repo))

    hits.sort(key=lambda h: (h["file"], h["line"]))
    for h in hits:
        key = "%s:%d" % (h["file"], h["line"])
        mark = "known" if key in known else "**NEW**"
        print(
            "%-4s %s:%d  %s=%s  ≈ %s 图头 %s / 真尺寸 %s   | %s"
            % (
                mark,
                h["file"],
                h["line"],
                h["axis"],
                h["value"],
                h["art"],
                h["header"],
                h["true"],
                h["text"][:70],
            )
        )
    if a.csv:
        with io.open(a.csv, "w", encoding="utf-8", newline="") as fh:
            w = csv.DictWriter(fh, fieldnames=list(hits[0]) if hits else ["file"])
            w.writeheader()
            for h in hits:
                w.writerow(h)
        print("->", a.csv)

    new = [h for h in hits if "%s:%d" % (h["file"], h["line"]) not in known]
    print("命中 %d 条（其中新命中 %d 条）" % (len(hits), len(new)))
    return 1 if new else 0


def selftest(a):
    import shutil
    import tempfile

    root = os.path.join(a.repo, "Client-Bevy", "src")
    bad = 0

    def run(repo):
        known = load_known(os.path.join(a.repo, "tools/acceptance/csharp_golden/rust_origin_audit_known.txt"))
        cache = {}
        out = []
        for dp, _d, fs in os.walk(os.path.join(repo, "Client-Bevy", "src")):
            for fn in sorted(fs):
                if fn.endswith(".rs"):
                    out.extend(scan_file(os.path.join(dp, fn), a.data, cache, repo))
        return out, [h for h in out if "%s:%d" % (h["file"], h["line"]) not in known]

    all_hits, new = run(a.repo)
    if new:
        print("FAIL(负对照)：干净树不该有**新**命中，实测 %d 条" % len(new))
        for h in new[:5]:
            print("   ", h["file"], h["line"], h["text"][:60])
        bad += 1
    else:
        print("OK(负对照)：干净树新命中 0 条（已知 %d 条全在表内）" % len(all_hits))

    tmp = tempfile.mkdtemp(prefix="rust_origin_selftest_")
    try:
        dst = os.path.join(tmp, "Client-Bevy", "src")
        shutil.copytree(root, dst)
        probe = os.path.join(dst, "game", "dialogs", "game_shop.rs")
        s = io.open(probe, encoding="utf-8", errors="ignore").read()
        s = s.replace(
            "let (px, py) = PANEL_ORIGIN;",
            "let (px, py) = ((1024.0 - 696.0) / 2.0, (768.0 - 476.0) / 2.0);",
        )
        io.open(probe, "w", encoding="utf-8").write(s)
        _all, got_new = run(tmp)
        if any(h["file"].endswith("game_shop.rs") for h in got_new):
            print("OK(正对照)：把商城原点改回图头字面量 ⇒ 被抓到（新命中 %d 条）" % len(got_new))
        else:
            print("FAIL(正对照)：改坏后没抓到")
            bad += 1
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
