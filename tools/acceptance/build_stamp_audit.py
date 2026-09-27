# build_stamp_audit.py —— 门禁：构建戳断言的**对象**必须是夹具真正会跑的那个 exe
#
# 为什么需要它（2026-09-28 实测）：
#   一批夹具的写法是「先 `Assert-ClientBuildStamp -Exe $exe`（校验**副本**）→ 再
#   `Copy-Item $ExeSrc -Destination $exe`（用规范 exe **覆盖**那个副本）→ 跑副本」。
#   于是断言断的是一个**即将被替换的文件**：
#     · 副本不存在 / 是旧的 ⇒ 假红（FAIL(2)「陈旧二进制」），白跑一轮；
#     · 副本恰好是 HEAD 的旧拷贝、而规范 exe 已经被别的提交重建 ⇒ **假绿**：
#       断言通过后，副本被换成了**未经校验**的那份，夹具却"有构建戳前置"。
#   修法只有两种：① 断言**规范 exe**（`$exeSrc`/`$ExeSrc`/`$ClientExe`，即将要被拷贝的那份）；
#   ② 或者把断言挪到拷贝**之后**（断言真正要跑的那个文件）。
#
# 判据（纯静态，秒级）：对每个 `Assert-ClientBuildStamp -Exe $X`，若在它**之后**还有
# `-Destination $X`（Copy-Item）或 `-Path $X -Target ...`（New-Item -HardLink）⇒ FAIL。
#
# 用法：
#   py -3.12 tools/acceptance/build_stamp_audit.py --repo .              # 常规门禁（0 命中 = exit 0）
#   py -3.12 tools/acceptance/build_stamp_audit.py --selftest            # 判据自检（负例 + 两条正例）
import argparse
import glob
import os
import re
import shutil
import sys
import tempfile

ASSERT_RE = re.compile(r"Assert-ClientBuildStamp\s+-Exe\s+\$(\w+)")
DEST_RE = re.compile(r"-Destination\s+\$(\w+)")
LINK_PATH_RE = re.compile(r"-Path\s+\$(\w+)")
LINK_TARGET_RE = re.compile(r"-Target\s+\$(\w+)")


def scan_text(text: str):
    """返回 [(行号, 变量名, 原因)]——断言之后才写入的副本。"""
    lines = text.splitlines()
    hits = []
    for i, ln in enumerate(lines):
        m = ASSERT_RE.search(ln)
        if not m:
            continue
        var = m.group(1)
        for j in range(i + 1, len(lines)):
            later = lines[j]
            d = DEST_RE.search(later)
            if d and d.group(1) == var:
                hits.append((i + 1, var, f"第 {j + 1} 行 `-Destination ${var}`（断言之后才写入副本）"))
                break
            lp, lt = LINK_PATH_RE.search(later), LINK_TARGET_RE.search(later)
            if lp and lt and lp.group(1) == var:
                hits.append((i + 1, var, f"第 {j + 1} 行 `-Path ${var} -Target ...`（断言之后才建链接）"))
                break
    return hits


def scan_file(path: str):
    with open(path, encoding="utf-8", errors="replace") as f:
        return scan_text(f.read())


def run_repo(repo: str) -> int:
    acc = os.path.join(repo, "tools", "acceptance")
    files = sorted(glob.glob(os.path.join(acc, "**", "*.ps1"), recursive=True))
    bad = 0
    for p in files:
        for line, var, why in scan_file(p):
            bad += 1
            print(f"FAIL {os.path.relpath(p, repo)}:{line}  断言 -Exe ${var} —— {why}")
    print(f"build_stamp_audit: 扫描 {len(files)} 个 ps1，命中 {bad} 处")
    return 1 if bad else 0


BAD = """\
. "$PSScriptRoot\\build_stamp.ps1"
$exe = "$root\\x_client.exe"
Assert-ClientBuildStamp -Exe $exe -ScriptName 'bad'
Copy-Item -LiteralPath $ExeSrc -Destination $exe -Force
"""
GOOD_ASSERT_SRC = """\
. "$PSScriptRoot\\build_stamp.ps1"
$exe = "$root\\x_client.exe"
Assert-ClientBuildStamp -Exe $ExeSrc -Worktree $Worktree -ScriptName 'good-src'
Copy-Item -LiteralPath $ExeSrc -Destination $exe -Force
"""
GOOD_ASSERT_AFTER = """\
. "$PSScriptRoot\\build_stamp.ps1"
$exe = "$root\\x_client.exe"
Copy-Item -LiteralPath $ExeSrc -Destination $exe -Force
Assert-ClientBuildStamp -Exe $exe -ScriptName 'good-after'
"""


def selftest() -> int:
    tmp = tempfile.mkdtemp(prefix="bsa_selftest_")
    try:
        cases = [("bad.ps1", BAD, 1), ("good_src.ps1", GOOD_ASSERT_SRC, 0),
                 ("good_after.ps1", GOOD_ASSERT_AFTER, 0)]
        bad = 0
        for name, text, want in cases:
            p = os.path.join(tmp, name)
            with open(p, "w", encoding="utf-8") as f:
                f.write(text)
            got = len(scan_file(p))
            ok = got == want
            print(f"  [{'PASS' if ok else 'FAIL'}] {name}：期望命中 {want}，实得 {got}")
            bad += 0 if ok else 1
        print("SelfTest PASS（负例命中 + 两条正例不命中）" if not bad else "SelfTest FAIL")
        return 1 if bad else 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", default=".")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()
    return run_repo(a.repo)


if __name__ == "__main__":
    sys.exit(main())
