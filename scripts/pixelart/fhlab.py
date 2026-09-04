import sys
sys.dont_write_bytecode = True
sys.path.insert(0, '.')
from engine import Canvas, PAL
import freehand
from PIL import Image, ImageDraw, ImageFont

def render(path, scale=13):
    ims = []
    for n, rows in freehand.SPRITES.items():
        w, h = len(rows[0]), len(rows)
        cv = Canvas(w+4, h+4, fill='1')
        freehand.blit(cv, 2, 2, n)
        im = Image.new('RGB', (cv.w, cv.h), (0x14,0x11,0x1c)); p = im.load()
        for y in range(cv.h):
            for x in range(cv.w):
                c = PAL.get(cv.px[y][x])
                if c: p[x, y] = c
        ims.append((n, im.resize((cv.w*scale, cv.h*scale), Image.NEAREST)))
    f = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 17)
    W = sum(i.width+18 for _, i in ims) + 18
    H = max(i.height for _, i in ims) + 58
    out = Image.new('RGB', (W, H), (0x0d,0x0b,0x14)); d = ImageDraw.Draw(out)
    x = 18
    for n, i in ims:
        out.paste(i, (x, 18)); d.text((x, i.height+26), n, font=f, fill=(0xf2,0xea,0xd9))
        x += i.width + 18
    out.save(path); print(path, out.size)

render(sys.argv[1] if len(sys.argv) > 1 else 'out/fhlab.png')
