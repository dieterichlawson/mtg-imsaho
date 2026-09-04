"""Sprite lab: humans in isolation until they stop being potatoes."""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, '.')
from engine import Canvas, PAL
import body, forms
from PIL import Image, ImageDraw, ImageFont

def legs(cv, ox, oy, s, c, shade, stance=None, boot=None, split=1):
    (hlx, hly), (hrx, hry) = s.hips()
    (flx, fly), (frx, fry) = s.feet(stance)
    kl = (hlx + flx)//2 - split; kr = (hrx + frx)//2 + split
    body.limb(cv, ox, oy, (hlx, hly), (kl, s.knee_y), c, shade, w=3, outer=-1)
    body.limb(cv, ox, oy, (kl, s.knee_y), (flx, fly), c, shade, w=3, outer=-1)
    body.limb(cv, ox, oy, (hrx, hry), (kr, s.knee_y), c, shade, w=3, outer=+1)
    body.limb(cv, ox, oy, (kr, s.knee_y), (frx, fry), c, shade, w=3, outer=+1)
    if boot:
        for fx, fy in ((flx, fly), (frx, fry)):
            cv.hline(oy+fy, ox+fx-1, ox+fx+2, boot)
            cv.hline(oy+fy-1, ox+fx-1, ox+fx+2, boot)

def arms(cv, ox, oy, s, c, shade, lh, rh, lk='open', rk='open',
         skin='n', hi='o', elbow=4):
    (lx, ly), (rx, ry) = s.shoulders()
    lel = ((lx + lh[0])//2 - elbow, (ly + lh[1])//2)
    rel = ((rx + rh[0])//2 + elbow, (ry + rh[1])//2)
    body.limb(cv, ox, oy, (lx, ly+1), lel, c, shade, w=3, taper=1, outer=-1)
    body.limb(cv, ox, oy, lel, lh, c, shade, w=2, outer=-1)
    body.limb(cv, ox, oy, (rx, ry+1), rel, c, shade, w=3, taper=1, outer=+1)
    body.limb(cv, ox, oy, rel, rh, c, shade, w=2, outer=+1)
    forms.hand(cv, ox, oy, lh[0], lh[1], lk, skin, shade, hi, flip=True)
    forms.hand(cv, ox, oy, rh[0], rh[1], rk, skin, shade, hi)

def seat_head(cv, ox, oy, s, kind, **kw):
    """Neck first, then the skull sitting on it."""
    body.neck(cv, ox, oy, 0, s.chin, s.shoulder_y, kw.get('skin','n'), kw.get('shade','m'))
    forms.head(cv, ox, oy, 0, 0, kind, **kw)

FIGS = {}
def fig(n):
    def d(f): FIGS[n] = f; return f
    return d

@fig("contour test")
def _():
    cv = Canvas(26, 36, fill='1'); s = body.Skel(30); ox, oy = 13, 3
    forms.torso(cv, ox, oy, s, 'man', '4', '3', '5')
    legs(cv, ox, oy, s, '4', '3')
    arms(cv, ox, oy, s, '4', '3', (-8, s.hip_y+1), (8, s.hip_y+1))
    seat_head(cv, ox, oy, s, 'man', skin='4', shade='3', hi='5', hair='2')
    return cv

@fig("soldier")
def _():
    cv = Canvas(26, 36, fill='1'); s = body.Skel(30, weight=+1); ox, oy = 12, 3
    legs(cv, ox, oy, s, 'k', '0', stance=3, boot='0')
    arms(cv, ox, oy, s, 'h', 'r', (-7, s.hip_y-1), (7, s.shoulder_y+4),
         lk='open', rk='grip')
    forms.torso(cv, ox, oy, s, 'man', 'h', 'r', 'i', rim='j', belt='k', twist=1)
    seat_head(cv, ox, oy, s, 'helm', skin='n', shade='m', hi='o', hair='k')
    cv.vline(ox+8, oy+1, oy+s.foot_y-1, 'l'); cv.vline(ox+9, oy+1, oy+s.foot_y-1, '0')
    cv.stamp(ox+7, oy-3, ["0t0","tt0","0t0"])
    return cv

@fig("zombie")
def _():
    cv = Canvas(26, 36, fill='1'); s = body.Skel(29, weight=-1, lean=-2); ox, oy = 13, 4
    s.shoulder_y += 2
    legs(cv, ox, oy, s, 'm', '0', stance=3, split=2)
    arms(cv, ox, oy, s, 'm', '0', (-9, s.hip_y+4), (6, s.hip_y+5),
         lk='point', rk='open', skin='6', hi='7')
    forms.torso(cv, ox, oy, s, 'gaunt', 'm', '0', '6', rim='6', twist=-2)
    seat_head(cv, ox, oy, s, 'skull', skin='6', shade='m', hi='7', hair='m')
    for x, y in ((-3,10),(2,14),(-1,18)):
        cv.set(ox+x, oy+y, '0'); cv.set(ox+x+1, oy+y+1, '0')
    return cv

@fig("vampire")
def _():
    cv = Canvas(30, 36, fill='1'); s = body.Skel(30, weight=+1); ox, oy = 15, 3
    for i in range(15):                       # cape behind, asymmetric
        cv.hline(oy+s.shoulder_y+i, ox-8-i, ox-5, '0')
        cv.hline(oy+s.shoulder_y+i, ox-8-i, ox-7-i, '2')
        cv.hline(oy+s.shoulder_y+i, ox+5, ox+7+i//2, '0')
    legs(cv, ox, oy, s, '1', '0', stance=3, boot='0')
    arms(cv, ox, oy, s, '1', '0', (-8, s.shoulder_y+4), (8, s.shoulder_y+5),
         lk='open', rk='point', skin='o', hi='7')
    forms.torso(cv, ox, oy, s, 'caped', '2', '0', '5', rim='5', twist=1)
    seat_head(cv, ox, oy, s, 'man', skin='o', shade='n', hi='7', hair='0', eye='b')
    for y in range(s.shoulder_y, s.shoulder_y+3):
        cv.set(ox-5, oy+y, '3'); cv.set(ox+5, oy+y, '3')
    return cv

@fig("scholar")
def _():
    cv = Canvas(28, 36, fill='1'); s = body.Skel(30, weight=-1); ox, oy = 14, 3
    arms(cv, ox, oy, s, 'p', '2', (-8, s.hip_y-1), (8, s.shoulder_y-4),
         lk='open', rk='point', skin='n', hi='o')
    forms.torso(cv, ox, oy, s, 'robed', 'p', '2', 'q', rim='q')
    for y in range(s.hip_y+1, s.foot_y+1):    # robe skirt to the floor
        t = (y - s.hip_y)/(s.foot_y - s.hip_y); w = int(round(11 + 3*t))
        for k in range(w):
            d = k/(w-1)
            cv.set(ox - w//2 + k, oy+y, 'q' if d < 0.15 else ('2' if d > 0.72 else 'p'))
        cv.set(ox-w//2-1, oy+y, '0'); cv.set(ox-w//2+w, oy+y, '0')
    seat_head(cv, ox, oy, s, 'man', skin='n', shade='m', hi='o', hair='5')
    return cv

@fig("brawler")
def _():
    cv = Canvas(30, 36, fill='1'); s = body.Skel(30, weight=+1); ox, oy = 15, 3
    legs(cv, ox, oy, s, 'k', '0', stance=4, boot='0')
    arms(cv, ox, oy, s, 'n', 'm', (-9, s.shoulder_y-3), (9, s.shoulder_y-4),
         lk='fist', rk='fist', skin='n', hi='o', elbow=5)
    forms.torso(cv, ox, oy, s, 'man', 'l', 'k', '9', belt='k', twist=-1)
    seat_head(cv, ox, oy, s, 'man', skin='n', shade='m', hi='o', hair='k')
    return cv

@fig("brute")
def _():
    cv = Canvas(32, 36, fill='1'); s = body.Skel(31, weight=-1); ox, oy = 16, 3
    legs(cv, ox, oy, s, 'm', '0', stance=4, boot='0', split=2)
    arms(cv, ox, oy, s, 'n', 'm', (-11, s.hip_y+3), (10, s.hip_y+1),
         lk='open', rk='grip', skin='n', hi='o', elbow=5)
    forms.torso(cv, ox, oy, s, 'hulk', 'n', 'm', 'o', rim='6')
    for y in range(s.shoulder_y-1, s.shoulder_y+4):
        cv.hline(oy+y, ox-10, ox-7, 's'); cv.hline(oy+y, ox+7, ox+10, '3')
        cv.set(ox-11, oy+y, '0'); cv.set(ox+11, oy+y, '0')
    for y in range(s.shoulder_y+3, s.hip_y - 1):        # one seam, off-centre
        cv.set(ox-2, oy+y, '0')
        if y % 2: cv.set(ox-3, oy+y, '6'); cv.set(ox-1, oy+y, '6')
    seat_head(cv, ox, oy, s, 'skull', skin='6', shade='m', hi='7', hair='m')
    return cv

def render(path, scale=13):
    ims = []
    for n in FIGS:
        cv = FIGS[n]()
        im = Image.new('RGB', (cv.w, cv.h), (0x14,0x11,0x1c)); p = im.load()
        for y in range(cv.h):
            for x in range(cv.w):
                c = PAL.get(cv.px[y][x])
                if c: p[x,y] = c
        ims.append((n, im.resize((cv.w*scale, cv.h*scale), Image.NEAREST)))
    f = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 17)
    W = sum(i.width+18 for _, i in ims) + 18
    H = max(i.height for _, i in ims) + 60
    out = Image.new('RGB', (W, H), (0x0d,0x0b,0x14)); d = ImageDraw.Draw(out)
    x = 18
    for n, i in ims:
        out.paste(i, (x, 18)); d.text((x, i.height+26), n, font=f, fill=(0xf2,0xea,0xd9))
        x += i.width + 18
    out.save(path); print(path, out.size)

if __name__ == '__main__':
    render(sys.argv[1] if len(sys.argv) > 1 else 'out/lab.png')
