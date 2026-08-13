#!/usr/bin/env python3
"""逐项对比 Bevy 选角截图与 C# 原版参考截图（静态 UI 元素）。"""
import sys, glob, os
from PIL import Image
import numpy as np

REF = os.environ.get("REF_SELECT_PNG", os.path.join(os.path.dirname(__file__), "..", "..", "tools", "ref_select.png"))
REGIONS = {
    "title": (468, 20, 84, 19),
    "slot1": (637, 194, 288, 56),
    "slot2": (637, 298, 288, 56),
    "slot3": (637, 402, 288, 56),
    "slot4": (637, 506, 288, 56),
    "btn_start": (132, 736, 100, 25),
    "btn_new": (296, 736, 100, 25),
    "btn_del": (460, 736, 100, 25),
    "btn_credits": (624, 736, 100, 25),
    "btn_exit": (788, 736, 100, 25),
}
THRESHOLD = 25.0

def main(paths):
    ref = Image.open(REF).convert("RGB")
    for path in paths:
        actual = Image.open(path).convert("RGB").resize((1024, 768), Image.LANCZOS)
        print(f"== {path} ==")
        maes = []
        for name, (x, y, w, h) in REGIONS.items():
            a = np.asarray(actual.crop((x, y, x + w, y + h)), dtype=np.float32)
            b = np.asarray(ref.crop((x, y, x + w, y + h)), dtype=np.float32)
            mae = float(np.mean(np.abs(a - b)))
            maes.append(mae)
            flag = "OK" if mae <= THRESHOLD else "DIFF"
            print(f"  {name:12s} MAE={mae:6.2f} {flag}")
        mean = sum(maes) / len(maes)
        print(f"  MEAN={mean:.2f}")
    return 0

if __name__ == "__main__":
    files = sys.argv[1:]
    if not files:
        files = sorted(glob.glob(os.path.join(os.path.dirname(__file__), "bevy_shot_*.png")))
    raise SystemExit(main(files))
