"""Pixel-art card engine: a tiny drawing DSL + card frame compositor.

Art is authored on a 40x32 grid of palette chars. Scenes are composed from
primitives (sky, moon, horizon, silhouettes) plus hand-drawn character
"stamps", so identity is hand-authored but atmosphere is cheap to iterate.
"""
from PIL import Image
import math, random

# ── Master palette: one gothic Innistrad ramp shared by every card ──────
PAL = {
 '.': None,              # transparent
 '0': (0x0d,0x0b,0x14),  # near black
 '1': (0x1c,0x18,0x26),  # night violet
 '2': (0x2e,0x27,0x40),  # dusk purple
 '3': (0x45,0x3b,0x5c),  # stone violet
 '4': (0x6b,0x5f,0x80),  # pale stone
 '5': (0x9a,0x8f,0xa8),  # fog grey
 '6': (0xcf,0xc7,0xc2),  # bone
 '7': (0xf2,0xea,0xd9),  # moon cream
 '8': (0xd9,0xc4,0x6a),  # candle gold
 '9': (0xa8,0x86,0x3c),  # brass
 'a': (0x7a,0x2f,0x2f),  # dried blood
 'b': (0xb5,0x34,0x3a),  # blood red
 'c': (0xe0,0x62,0x3c),  # flame orange
 'd': (0xf2,0xa0,0x3c),  # fire yellow
 'e': (0x2b,0x4a,0x63),  # cold blue
 'f': (0x3f,0x7f,0xa6),  # spirit blue
 'g': (0x7f,0xc4,0xd9),  # ghost cyan
 'h': (0x27,0x4a,0x34),  # deep forest
 'i': (0x4a,0x80,0x46),  # moss
 'j': (0x8f,0xbf,0x5a),  # pale green
 'k': (0x5c,0x3a,0x24),  # bark
 'l': (0x8a,0x65,0x40),  # wood
 'm': (0x3a,0x2a,0x3a),  # flesh shadow
 'n': (0xb0,0x8c,0x7a),  # flesh
 'o': (0xe3,0xc3,0xa8),  # pale flesh
 'p': (0x6e,0x4a,0x7a),  # arcane violet
 'q': (0xa9,0x7f,0xd0),  # arcane light
 'r': (0x14,0x2a,0x22),  # swamp black-green
 's': (0x60,0x6a,0x74),  # steel
 't': (0x8f,0x9a,0xa6),  # bright steel
}

AW, AH = 42, 34          # art window

class Canvas:
    def __init__(self, w=AW, h=AH, fill='1'):
        self.w, self.h = w, h
        self.px = [[fill]*w for _ in range(h)]
    def set(self, x, y, c):
        if c is not None and 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = c
    def get(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h: return self.px[y][x]
        return '.'
    # ── primitives ──
    def rect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1+1):
            for x in range(x0, x1+1): self.set(x, y, c)
    def hline(self, y, x0, x1, c):
        for x in range(x0, x1+1): self.set(x, y, c)
    def vline(self, x, y0, y1, c):
        for y in range(y0, y1+1): self.set(x, y, c)
    def line(self, x0, y0, x1, y1, c):
        n = max(abs(x1-x0), abs(y1-y0)) or 1
        for i in range(n+1):
            self.set(round(x0+(x1-x0)*i/n), round(y0+(y1-y0)*i/n), c)
    def disc(self, cx, cy, r, c, edge=None):
        for y in range(int(cy-r-1), int(cy+r+2)):
            for x in range(int(cx-r-1), int(cx+r+2)):
                d = math.hypot(x-cx, y-cy)
                if d <= r: self.set(x, y, c)
                elif edge and d <= r+0.9: self.set(x, y, edge)
    def ring(self, cx, cy, r, c):
        for a in range(0, 360, 3):
            self.set(round(cx+r*math.cos(math.radians(a))),
                     round(cy+r*math.sin(math.radians(a))), c)
    def poly(self, pts, c):
        """Scanline-fill a polygon."""
        ys = [p[1] for p in pts]
        for y in range(int(min(ys))-1, int(max(ys))+1):
            yc = y + 0.5
            xs = []
            for i in range(len(pts)):
                x0,y0 = pts[i]; x1,y1 = pts[(i+1) % len(pts)]
                if (y0 <= yc < y1) or (y1 <= yc < y0):
                    xs.append(x0 + (x1-x0)*(yc-y0)/(y1-y0))
            xs.sort()
            for i in range(0, len(xs)-1, 2):
                for x in range(int(round(xs[i])), int(round(xs[i+1]))+1):
                    self.set(x, y, c)
    def stamp(self, x0, y0, art, key=None):
        """Blit a hand-drawn ASCII sprite; '.' is transparent."""
        for dy, rowstr in enumerate(art):
            for dx, ch in enumerate(rowstr):
                if ch == '.': continue
                self.set(x0+dx, y0+dy, key.get(ch, ch) if key else ch)
    def noise(self, x0, y0, x1, y1, c, density, seed=0):
        rng = random.Random(seed)
        for y in range(y0, y1+1):
            for x in range(x0, x1+1):
                if rng.random() < density: self.set(x, y, c)
    def gradient_sky(self, ramp):
        """ramp: list of (row_fraction, char) top->bottom."""
        for y in range(self.h):
            f = y/(self.h-1)
            c = ramp[0][1]
            for fr, ch in ramp:
                if f >= fr: c = ch
            self.hline(y, 0, self.w-1, c)
    def stars(self, n, c='4', seed=1, maxy=None):
        rng = random.Random(seed)
        maxy = maxy if maxy is not None else self.h//2
        for _ in range(n):
            self.set(rng.randrange(self.w), rng.randrange(maxy), c)
    def stroke(self, pts, r, c, steps=24):
        """Discs along a polyline — thick organic strokes (smoke, spirits)."""
        for i in range(len(pts)-1):
            (x0,y0),(x1,y1) = pts[i], pts[i+1]
            rr = r[i] if isinstance(r, (list,tuple)) else r
            r1 = r[i+1] if isinstance(r, (list,tuple)) else r
            for s in range(steps+1):
                t = s/steps
                self.disc(x0+(x1-x0)*t, y0+(y1-y0)*t, rr+(r1-rr)*t, c)
    def outline(self, c='0', bg=None):
        """Add a keyline around every non-bg region — makes shapes read at 1x."""
        src = [row[:] for row in self.px]
        for y in range(self.h):
            for x in range(self.w):
                if src[y][x] != bg: continue
                for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
                    nx,ny = x+dx, y+dy
                    if 0<=nx<self.w and 0<=ny<self.h and src[ny][nx] not in (bg,c):
                        self.px[y][x] = c; break
    def limb(self, x0, y0, x1, y1, core, edge='0', w=2):
        """An arm or leg: >=2px of core colour inside a dark edge. Never 1px."""
        import math as _m
        n = max(abs(x1-x0), abs(y1-y0)) or 1
        perp = (-(y1-y0)/n, (x1-x0)/n)
        for i in range(n+1):
            cx = x0 + (x1-x0)*i/n; cy = y0 + (y1-y0)*i/n
            for k in range(-1, w+1):
                c = edge if k in (-1, w) else core
                self.set(round(cx + perp[0]*k), round(cy + perp[1]*k), c)

    def hand(self, x, y, skin, shade='0'):
        """A 3x4 hand block. Every arm must end in one, touching its object."""
        self.stamp(x, y, [shade+skin+shade, skin+skin+skin,
                          skin+skin+skin, shade+skin+shade])

    def rows(self): return [''.join(r) for r in self.px]
