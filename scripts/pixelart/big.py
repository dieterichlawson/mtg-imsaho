import sys
sys.dont_write_bytecode = True
sys.path.insert(0, '.')
from engine import Canvas, PAL
import freehand
from PIL import Image, ImageDraw, ImageFont
names = sys.argv[2:]
S = int(sys.argv[1])
ims = []
for n in names:
    rows = freehand.SPRITES[n]
    cv = Canvas(len(rows[0])+2, len(rows)+2, fill='1')
    freehand.blit(cv, 1, 1, n)
    im = Image.new('RGB', (cv.w, cv.h), (0x14,0x11,0x1c)); p = im.load()
    for y in range(cv.h):
        for x in range(cv.w):
            c = PAL.get(cv.px[y][x])
            if c: p[x, y] = c
    im = im.resize((cv.w*S, cv.h*S), Image.NEAREST)
    d = ImageDraw.Draw(im)
    for gx in range(0, im.width, S):  d.line([(gx,0),(gx,im.height)], fill=(40,36,50))
    for gy in range(0, im.height, S): d.line([(0,gy),(im.width,gy)], fill=(40,36,50))
    ims.append((n, im))
f = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 18)
W = sum(i.width+20 for _, i in ims) + 20
H = max(i.height for _, i in ims) + 56
out = Image.new('RGB', (W, H), (0x0d,0x0b,0x14)); d = ImageDraw.Draw(out)
x = 20
for n, i in ims:
    out.paste(i, (x, 16)); d.text((x, i.height+26), n, font=f, fill=(0xf2,0xea,0xd9))
    x += i.width + 20
out.save('out/big.png'); print('out/big.png', out.size)
