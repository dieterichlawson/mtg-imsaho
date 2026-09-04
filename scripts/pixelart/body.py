"""A parametric human for small sprites — wireframe first, then flesh.

Every figure in the set so far was a 20px blob with a 7px head: three heads
tall, which is chibi proportion, which is why they all read as potatoes.
Both pixel-art references converge on a SIX-head model for small sprites
(stocky but unmistakably adult). At 30px tall that means a 5px head.

The skeleton is numeric so proportion is tunable instead of eyeballed, and
so contrapposto is expressible: weight on one foot, hips tilt one way and
shoulders tilt the OPPOSITE way. That single asymmetry is what kills the
stiff mirrored pose that made every figure look like the same doll.
"""
import math

class Skel:
    """Joint positions for a figure `h` px tall, in figure-local coords."""
    def __init__(self, h=30, weight=+1, lean=0, shoulder_w=None, hip_w=None):
        self.h = h
        hd = max(4, round(h/6.0))          # head height == 1/6 of total
        self.head_h = hd
        self.head_w = hd
        self.crown  = 0
        self.chin   = hd
        self.neck   = hd + 1
        self.shoulder_y = hd + 1   # exactly one row of neck
        self.pelvis_y   = round(h * 0.52)  # waist a little above mid
        self.hip_y      = round(h * 0.58)
        self.knee_y     = round(h * 0.78)
        self.foot_y     = h - 1
        self.sw = shoulder_w if shoulder_w else round(hd * 2.4)   # ~2.4 heads wide
        self.hw = hip_w if hip_w else round(hd * 1.7)
        # contrapposto: hips tilt toward the weighted leg, shoulders counter it
        self.hip_tilt      = +1 * weight
        self.shoulder_tilt = -1 * weight
        self.lean = lean
        self.cx = 0

    def shoulders(self):
        h = self.sw // 2
        return ((-h + self.lean, self.shoulder_y - self.shoulder_tilt),
                ( h + self.lean, self.shoulder_y + self.shoulder_tilt))
    def hips(self):
        h = self.hw // 2
        return ((-h, self.hip_y - self.hip_tilt), (h, self.hip_y + self.hip_tilt))
    def feet(self, stance=None):
        s = stance if stance is not None else max(2, self.hw // 2)
        return ((-s, self.foot_y), (s + 1, self.foot_y))

def limb(cv, ox, oy, a, b, c, shade, w=2, taper=0, outer=0):
    """A tapered limb segment with a lit side — never a bare 1px line."""
    (x0, y0), (x1, y1) = a, b
    n = max(abs(x1-x0), abs(y1-y0), 1)
    for i in range(n+1):
        t = i / n
        x = x0 + (x1-x0)*t; y = y0 + (y1-y0)*t
        ww = max(1, int(round(w - taper*t)))
        for k in range(ww):
            cv.set(ox + int(round(x)) + k, oy + int(round(y)), c if k else shade)
        if outer <= 0: cv.set(ox + int(round(x)) - 1, oy + int(round(y)), '0')
        if outer >= 0: cv.set(ox + int(round(x)) + ww, oy + int(round(y)), '0')

HAND = ["0nn0", "nono", "nnnn", "0nn0"]
def hand(cv, ox, oy, x, y, skin='n', hi='o'):
    art = ["0"+skin*2+"0", skin+hi+skin+skin, skin*4, "0"+skin*2+"0"]
    cv.stamp(ox + x - 1, oy + y - 1, art)

def head(cv, ox, oy, x, y, hd, skin='n', shade='m', hi='o', hair=None, eyes='0'):
    """A skull, not an oval: flat crown, brow ridge, cheek step, tapered jaw."""
    w = hd
    for r in range(hd):
        if r == 0:            x0, x1 = x-w//2+1, x+w//2-1     # crown, narrowed
        elif r >= hd-1:       x0, x1 = x-w//2+1, x+w//2-1     # jaw, narrowed
        else:                 x0, x1 = x-w//2,   x+w//2
        for xx in range(x0, x1+1):
            cv.set(ox+xx, oy+y+r, hi if xx <= x0 else (shade if xx >= x1 else skin))
        cv.set(ox+x0-1, oy+y+r, '0'); cv.set(ox+x1+1, oy+y+r, '0')
    cv.hline(oy+y-1, ox+x-w//2+1, ox+x+w//2-1, '0')
    br = y + max(1, hd//3)                                   # brow + eyes
    cv.set(ox+x-w//2+1, oy+br, eyes); cv.set(ox+x+w//2-1, oy+br, eyes)
    if hair:
        for xx in range(x-w//2, x+w//2+1):
            cv.set(ox+xx, oy+y, hair)
        cv.set(ox+x-w//2, oy+y+1, hair); cv.set(ox+x+w//2, oy+y+1, hair)

def neck(cv, ox, oy, x, chin, shoulder_y, skin='n', shade='m', w=3):
    """Close the gap between jaw and chest — a floating head reads as a mask."""
    for yy in range(chin, shoulder_y):
        for k in range(w):
            cv.set(ox + x - w//2 + k, oy + yy, shade if k >= w-1 else skin)
        cv.set(ox + x - w//2 - 1, oy + yy, '0')
        cv.set(ox + x - w//2 + w, oy + yy, '0')
