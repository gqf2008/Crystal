#!/usr/bin/env python3
"""把 ui_shot_matrix 的截图按对话框矩形裁好，拼成若干张联络表（contact sheet）便于逐窗目检。"""
import json, os, sys
from PIL import Image, ImageDraw

M = sys.argv[1] if len(sys.argv) > 1 else "tools/acceptance/shots/matrix"
OUT = sys.argv[2] if len(sys.argv) > 2 else "tools/acceptance/shots/sheet"
S = 1.5
os.makedirs(OUT, exist_ok=True)
rows = json.load(open(f"{M}/manifest.json", encoding="utf-8"))
tiles = []
for r in rows:
    if not r.get("open") or not r.get("rect"):
        continue
    rc = r["rect"]
    im = Image.open(r["shot"])
    pad = 6
    x0 = max(0, int(rc["rx"] * S) - pad); y0 = max(0, int(rc["ry"] * S) - pad)
    x1 = min(im.width, int((rc["rx"] + rc["rw"]) * S) + pad)
    y1 = min(im.height, int((rc["ry"] + rc["rh"]) * S) + pad)
    c = im.crop((x0, y0, x1, y1))
    # 统一缩放到高 300（保持比例），太宽的再压
    h = 300
    w = max(1, int(c.width * h / c.height))
    c = c.resize((w, h), Image.LANCZOS)
    tiles.append((r["kind"], c))

per = 6
cols = 3
for gi in range(0, len(tiles), per):
    grp = tiles[gi:gi + per]
    cw = max(t[1].width for t in grp) + 8
    rowsn = (len(grp) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * cw, rowsn * (300 + 26)), (24, 26, 25))
    d = ImageDraw.Draw(sheet)
    for i, (kind, im) in enumerate(grp):
        cx = (i % cols) * cw + 4
        cy = (i // cols) * (300 + 26)
        d.text((cx, cy + 6), f"{kind}  ({im.width}x{im.height})", fill=(230, 230, 230))
        sheet.paste(im, (cx, cy + 22))
    p = f"{OUT}/sheet_{gi // per}.png"
    sheet.save(p)
    print(p, sheet.size)
