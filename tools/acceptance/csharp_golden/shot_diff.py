import sys
from PIL import Image, ImageChops

a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size:
    print("size mismatch", a.size, b.size)
    sys.exit(1)
if len(sys.argv) > 3:
    x0, y0, x1, y1 = [int(v) for v in sys.argv[3].split(",")]
    a = a.crop((x0, y0, x1, y1))
    b = b.crop((x0, y0, x1, y1))
d = ImageChops.difference(a, b)
bbox = d.getbbox()
print("bbox of differences:", bbox)
if bbox:
    px = d.load()
    n = 0
    x0, y0, x1, y1 = bbox
    for y in range(y0, y1):
        for x in range(x0, x1):
            if sum(px[x, y]) > 12:
                n += 1
    print("changed pixels (>12 sum):", n, "of", (x1 - x0) * (y1 - y0))
