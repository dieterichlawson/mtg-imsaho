import sys
sys.dont_write_bytecode = True
sys.path.insert(0, '.')
from engine import Canvas, PAL
import freehand
from PIL import Image, ImageDraw, ImageFont

PAIRS = [("soldier_h","soldier_h2"),("zombie_h","zombie_h2"),
         ("vampire_h","vampire_h2"),("scholar_h","scholar_h2")]
def img(name, scale):
    rows = freehand.SPRITES[name]
    cv = Canvas(len(rows[0])+4, len(rows)+4, fill='1')
    freehand.blit(cv, 2, 2, name)
    im = Image.new('RGB', (cv.w, cv.h), (0x14,0x11,0x1c)); p = im.load()
    for y in range(cv.h):
        for x in range(cv.w):
            c = PAL.get(cv.px[y][x])
            if c: p[x, y] = c
    return im.resize((cv.w*scale, cv.h*scale), Image.NEAREST)

S = 11
ims = [(a, img(a,S), img(b,S)) for a, b in PAIRS]
fb = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 16)
fs = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 14)
CW = max(a.width + b.width + 12 for _, a, b in ims)
H  = max(max(a.height, b.height) for _, a, b in ims)
W  = sum(a.width + b.width + 12 + 28 for _, a, b in ims) + 28
out = Image.new('RGB', (W, H + 76), (0x0d,0x0b,0x14)); d = ImageDraw.Draw(out)
d.text((20, 12), "HORROR 1  →  HORROR 2", font=fb, fill=(0xf2,0xea,0xd9))
d.text((240, 14), "pass 1 kept the values and lost the silhouette. pass 2 puts hard angles back in the outline.",
       font=fs, fill=(0x6b,0x5f,0x80))
x = 20
for n, a, b in ims:
    out.paste(a, (x, 44)); out.paste(b, (x + a.width + 12, 44))
    d.text((x, 44 + H + 8), n, font=fb, fill=(0xf2,0xea,0xd9))
    x += a.width + b.width + 12 + 28
out.save('out/horror2.png'); print('out/horror2.png', out.size)
