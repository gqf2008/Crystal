#!/usr/bin/env python3
"""按 manifest 里该窗的矩形裁剪截图（可选再放大）。用法: crop_kind.py <kind> <out.png> [scale] [pad]"""
import json, sys
from PIL import Image
S = 1.5
kind, out = sys.argv[1], sys.argv[2]
scale = float(sys.argv[3]) if len(sys.argv) > 3 else 2.0
pad = float(sys.argv[4]) if len(sys.argv) > 4 else 4.0
rows = json.load(open("tools/acceptance/shots/matrix/manifest.json", encoding="utf-8"))
r = next(x for x in rows if x["kind"] == kind)
rc = r["rect"]
im = Image.open(r["shot"])
box = (max(0, int(rc["rx"] * S - pad)), max(0, int(rc["ry"] * S - pad)),
       min(im.width, int((rc["rx"] + rc["rw"]) * S + pad)), min(im.height, int((rc["ry"] + rc["rh"]) * S + pad)))
c = im.crop(box)
c = c.resize((int(c.width * scale), int(c.height * scale)), Image.LANCZOS)
c.save(out)
print(out, c.size)
