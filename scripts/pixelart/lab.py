"""Sprite lab: humans drawn in isolation until they stop being potatoes."""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, '.')
from engine import Canvas, PAL
import body
from PIL import Image, ImageDraw, ImageFont

def torso(cv, ox, oy, s, c, shade, hi, waist=None, belt=None, rim=None):
    """Chest tapering to waist then flaring to hips — not a rectangle.

    Light comes from the upper left throughout the set, so the left column is
    the highlight, the right two are shadow, and an optional rim picks out the
    lit edge. A flat single-tone torso is what made these read as cardboard.
    """
    for y in range(s.shoulder_y, s.hip_y + 1):
        t = (y - s.shoulder_y) / max(1, (s.hip_y - s.shoulder_y))
        w = s.sw + (s.hw - s.sw) * (t if waist is None else min(1, t/waist))
        w = max(4, int(round(w)))
        x0 = -w//2 + int(round(s.lean * (1-t)))
        for k in range(w):
            cv.set(ox + x0 + k, oy + y, hi if k == 0 else (shade if k >= w-2 else c))
        # a fold: one darker column drifting across the chest
        f = int(round(w*0.62 + 1.5*t))
        if 0 < f < w-1: cv.set(ox + x0 + f, oy + y, shade)
        cv.set(ox + x0 - 1, oy + y, '0'); cv.set(ox + x0 + w, oy + y, '0')
        if rim: cv.set(ox + x0, oy + y, rim if y % 3 else hi)
    if belt:
        by = s.hip_y - 2
        t = (by - s.shoulder_y) / max(1, (s.hip_y - s.shoulder_y))
        w = max(4, int(round(s.sw + (s.hw - s.sw) * (t if waist is None else min(1, t/waist)))))
        cv.hline(oy + by, ox - w//2, ox - w//2 + w - 1, belt)
        cv.set(ox, oy + by, '8')

def legs(cv, ox, oy, s, c, shade, stance=None, boot=None):
    (hlx, hly), (hrx, hry) = s.hips()
    (flx, fly), (frx, fry) = s.feet(stance)
    kx_l = (hlx + flx)//2 - 1; kx_r = (hrx + frx)//2 + 1
    body.limb(cv, ox, oy, (hlx, hly), (kx_l, s.knee_y), c, shade, w=3)
    body.limb(cv, ox, oy, (kx_l, s.knee_y), (flx, fly), c, shade, w=3)
    body.limb(cv, ox, oy, (hrx, hry), (kx_r, s.knee_y), c, shade, w=3)
    body.limb(cv, ox, oy, (kx_r, s.knee_y), (frx, fry), c, shade, w=3)
    if boot:
        for (fx, fy) in ((flx, fly), (frx, fry)):
            cv.hline(oy+fy, ox+fx-1, ox+fx+2, boot)
            cv.hline(oy+fy-1, ox+fx-1, ox+fx+2, boot)

def arms(cv, ox, oy, s, c, shade, lhand, rhand, skin='n', hi='o', elbow_out=4):
    (lx, ly), (rx, ry) = s.shoulders()
    lel = ((lx + lhand[0])//2 - elbow_out, (ly + lhand[1])//2)
    rel = ((rx + rhand[0])//2 + elbow_out, (ry + rhand[1])//2)
    body.limb(cv, ox, oy, (lx, ly+1), lel, c, shade, w=3, taper=1, outer=-1)
    body.limb(cv, ox, oy, lel, lhand, c, shade, w=2, outer=-1)
    body.limb(cv, ox, oy, (rx, ry+1), rel, c, shade, w=3, taper=1, outer=+1)
    body.limb(cv, ox, oy, rel, rhand, c, shade, w=2, outer=+1)
    body.hand(cv, ox, oy, lhand[0], lhand[1]-1, skin, hi)
    body.hand(cv, ox, oy, rhand[0], rhand[1]-1, skin, hi)

FIGS = {}
def fig(name):
    def d(f): FIGS[name] = f; return f
    return d

@fig("wireframe 6-head")
def _():
    cv = Canvas(26, 34, fill='1'); s = body.Skel(30); ox, oy = 13, 2
    for y in (s.chin, s.shoulder_y, s.pelvis_y, s.hip_y, s.knee_y, s.foot_y):
        cv.hline(oy+y, ox-11, ox+11, '3')
    legs(cv, ox, oy, s, '4', '3')
    arms(cv, ox, oy, s, '4', '3', (-8, s.hip_y+1), (8, s.hip_y+1))
    torso(cv, ox, oy, s, '4', '3', '5')
    body.neck(cv, ox, oy, 0, s.chin, s.shoulder_y); body.head(cv, ox, oy, 0, 0, s.head_h, '4', '3', '5')
    return cv

@fig("soldier")
def _():
    cv = Canvas(26, 34, fill='1'); s = body.Skel(30, weight=+1, lean=0); ox, oy = 13, 2
    legs(cv, ox, oy, s, 'k', '0', stance=4, boot='0')
    arms(cv, ox, oy, s, 'h', 'r', (-7, s.hip_y-1), (6, s.shoulder_y+3))
    torso(cv, ox, oy, s, 'h', 'r', 'i', waist=0.7, belt='k', rim='j')
    for y in range(s.shoulder_y, s.shoulder_y+3):      # pauldrons
        cv.hline(oy+y, ox-7, ox-5, 't'); cv.hline(oy+y, ox+5, ox+7, 's')
    body.neck(cv, ox, oy, 0, s.chin, s.shoulder_y); body.head(cv, ox, oy, 0, 0, s.head_h, 'n', 'm', 'o', hair='k')
    for x in range(-2, 3): cv.set(ox+x, oy-1, 't')     # helm brim
    cv.vline(ox+7, oy+1, oy+s.foot_y-2, 'l')           # spear
    cv.vline(ox+8, oy+1, oy+s.foot_y-2, '0')
    cv.stamp(ox+6, oy-3, ["0t0","tt0","0t0"])
    return cv

@fig("zombie (hunched)")
def _():
    cv = Canvas(26, 34, fill='1'); s = body.Skel(28, weight=-1, lean=-2); ox, oy = 14, 4
    s.shoulder_y += 2                                   # slumped shoulders
    legs(cv, ox, oy, s, 'm', '0', stance=3)
    arms(cv, ox, oy, s, 'm', '0', (-8, s.hip_y+3), (6, s.hip_y+4), skin='m', hi='6')
    torso(cv, ox, oy, s, 'm', '0', '6', waist=0.9, rim='6')
    body.neck(cv, ox, oy, 0, s.chin, s.shoulder_y); body.head(cv, ox, oy, -2, 1, s.head_h, '6', 'm', '7')
    cv.set(ox-4, oy+3, '0'); cv.set(ox-1, oy+3, '0')
    for (x, y) in ((-4,10),(2,13),(-1,17)):             # torn flesh
        cv.set(ox+x, oy+y, '0'); cv.set(ox+x+1, oy+y+1, '0')
    return cv

@fig("vampire (caped)")
def _():
    cv = Canvas(30, 34, fill='1'); s = body.Skel(30, weight=+1); ox, oy = 15, 2
    for i in range(14):                                 # cape, asymmetric flare
        cv.hline(oy+s.shoulder_y+i, ox-7-i, ox-5, '0')
        cv.hline(oy+s.shoulder_y+i, ox-7-i, ox-6-i, '2')
        cv.hline(oy+s.shoulder_y+i, ox+5, ox+6+i//2, '0')
    legs(cv, ox, oy, s, '0', '0', stance=3)
    arms(cv, ox, oy, s, '1', '0', (-8, s.shoulder_y+4), (8, s.shoulder_y+5), skin='o', hi='7')
    torso(cv, ox, oy, s, '1', '0', '3', waist=0.6, rim='5')
    body.neck(cv, ox, oy, 0, s.chin, s.shoulder_y); body.head(cv, ox, oy, 0, 0, s.head_h, 'o', 'n', '7', hair='0')
    cv.set(ox-2, oy+2, 'b'); cv.set(ox+2, oy+2, 'b')    # red eyes
    for y in range(s.shoulder_y, s.shoulder_y+3):       # high collar
        cv.set(ox-4, oy+y, '0'); cv.set(ox+4, oy+y, '0')
    return cv

@fig("scholar (robed)")
def _():
    cv = Canvas(26, 34, fill='1'); s = body.Skel(30, weight=-1); ox, oy = 13, 2
    arms(cv, ox, oy, s, 'p', '2', (-7, s.hip_y), (7, s.shoulder_y-4), skin='n', hi='o')
    for y in range(s.shoulder_y, s.foot_y+1):           # robe widens to the floor
        t = (y - s.shoulder_y)/(s.foot_y - s.shoulder_y)
        w = int(round(s.sw*0.85 + 7*t))
        for k in range(w):
            cv.set(ox - w//2 + k, oy+y, 'q' if k == 0 else ('p' if k < w-2 else '2'))
        cv.set(ox-w//2-1, oy+y, '0'); cv.set(ox-w//2+w, oy+y, '0')
    arms(cv, ox, oy, s, 'p', '2', (-7, s.hip_y), (7, s.shoulder_y-4), skin='n', hi='o')
    body.neck(cv, ox, oy, 0, s.chin, s.shoulder_y); body.head(cv, ox, oy, 0, 0, s.head_h, 'n', 'm', 'o')
    return cv

@fig("brawler (fists up)")
def _():
    cv = Canvas(28, 34, fill='1'); s = body.Skel(30, weight=+1, shoulder_w=14); ox, oy = 14, 2
    legs(cv, ox, oy, s, 'k', '0', stance=5, boot='0')
    arms(cv, ox, oy, s, 'n', 'm', (-9, s.shoulder_y-3), (9, s.shoulder_y-4),
         skin='n', hi='o', elbow_out=4)
    torso(cv, ox, oy, s, 'l', 'k', '9', waist=0.75, belt='k')
    body.neck(cv, ox, oy, 0, s.chin, s.shoulder_y); body.head(cv, ox, oy, 0, 0, s.head_h, 'n', 'm', 'o', hair='k')
    return cv

@fig("brute (armoured)")
def _():
    cv = Canvas(30, 34, fill='1'); s = body.Skel(31, weight=-1, shoulder_w=17, hip_w=11)
    ox, oy = 15, 2
    legs(cv, ox, oy, s, 'm', '0', stance=5, boot='0')
    arms(cv, ox, oy, s, 'n', 'm', (-11, s.hip_y+3), (10, s.hip_y+1), skin='n', hi='o')
    torso(cv, ox, oy, s, 'n', 'm', 'o', waist=0.95, rim='6')
    for y in range(s.shoulder_y-1, s.shoulder_y+4):     # heavy shoulder plates
        cv.hline(oy+y, ox-9, ox-6, 's'); cv.hline(oy+y, ox+6, ox+9, '3')
        cv.set(ox-10, oy+y, '0'); cv.set(ox+10, oy+y, '0')
    body.neck(cv, ox, oy, 0, s.chin, s.shoulder_y); body.head(cv, ox, oy, 1, 0, s.head_h, '6', 'm', '7')
    for i, y in enumerate(range(s.shoulder_y+3, s.hip_y, 4)):   # stitches, tapering
        w = 5 - i
        cv.hline(oy+y, ox-w, ox+w, '0')
        for x in range(-w, w+1, 2): cv.set(ox+x, oy+y-1, '6')
    return cv

def render(path, scale=13):
    names = list(FIGS)
    ims = []
    for n in names:
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
