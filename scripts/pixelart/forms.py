"""Hand-authored body contours. No interpolation, no primitives.

A torso built by lerping shoulder-width to hip-width is a quadrilateral, and
a head or hand built by narrowing a rectangle at both ends is an orb. Neither
is what a body looks like. These are explicit per-row half-width tables and
explicit pixel stamps, so the silhouette carries deltoid bulge, rib taper,
waist pinch and hip flare — and a skull carries a brow, a cheek and a jaw.
"""

# ── Torso half-widths, shoulder row → hip row ────────────────────────
# Read down the column: traps slope in at the neck, deltoids bulge, ribcage
# tapers, waist pinches, hips flare back out. That S-curve is the whole
# difference between a person and a trapezoid.
TORSO = {
 'man':    [4,6,6,6,5,5,4,4,3,4,4,4],
 'gaunt':  [3,5,5,4,4,3,3,3,2,3,3,4],
 'hulk':   [5,8,9,9,8,8,7,7,6,7,7,7],
 'robed':  [3,5,5,5,5,6,6,7,8,9,10,11],
 'caped':  [4,6,6,5,5,4,4,4,3,4,5,5],
}

def sample(prof, t):
    """Read a profile at 0..1 with linear blend between authored rows."""
    f = t * (len(prof) - 1)
    i = int(f); frac = f - i
    if i >= len(prof) - 1: return prof[-1]
    return prof[i] * (1 - frac) + prof[i + 1] * frac

def torso(cv, ox, oy, s, kind, c, shade, hi, rim=None, belt=None, twist=0):
    """Draw a torso from its authored contour. `twist` offsets the two sides
    by different amounts, so the body turns instead of standing flat-on."""
    prof = TORSO[kind]
    n = s.hip_y - s.shoulder_y + 1
    for i in range(n):
        t = i / max(1, n - 1)
        hw = sample(prof, t)
        lean = s.lean * (1 - t)
        xl = int(round(-hw + lean - twist * (1 - t)))
        xr = int(round( hw + lean - twist * (1 - t)))
        y = s.shoulder_y + i
        for x in range(xl, xr + 1):
            d = (x - xl) / max(1, (xr - xl))
            cv.set(ox + x, oy + y, hi if d < 0.18 else (shade if d > 0.72 else c))
        # a fold that drifts, so the front isn't a flat field
        f = int(round(xl + (xr - xl) * (0.58 + 0.18 * t)))
        cv.set(ox + f, oy + y, shade)
        cv.set(ox + xl - 1, oy + y, '0'); cv.set(ox + xr + 1, oy + y, '0')
        if rim and i % 3 != 2: cv.set(ox + xl, oy + y, rim)
    if belt:
        i = n - 4
        t = i / max(1, n - 1); hw = sample(prof, t)
        xl = int(round(-hw + s.lean * (1 - t))); xr = int(round(hw + s.lean * (1 - t)))
        cv.hline(oy + s.shoulder_y + i, ox + xl, ox + xr, belt)
        cv.set(ox + (xl + xr)//2, oy + s.shoulder_y + i, '8')

# ── Heads: skulls, not ellipses ──────────────────────────────────────
# Flat crown under the hair mass, straight temples, one cheek step, and a
# jaw that narrows only at the bottom. Narrowing top AND bottom is an egg.
# '.' transparent  H hair  s skin  h highlight  d shadow  e eye  0 keyline
HEADS = {
 # FIVE rows, restoring the six-head model at 30px. Note there is no keyline
 # down the sides: at 5px wide, two keyline columns leave three interior
 # pixels and the eye row collapses into a visor slit. The hair mass above
 # and the jaw below carry the silhouette instead, and against these dark
 # backgrounds a lit face reads on its own.
 # H hair  s skin  h highlight  d shadow  e eye  n nose  t steel
 'man':    ["0HHH0", "hsssd", "hesed", "hsnsd", "0sdd0"],
 'woman':  ["HHHHH", "Hsssd", "Hesed", "Hsnsd", "H0dd0"],
 'bald':   ["0sss0", "hsssd", "hesed", "hsnsd", "0sdd0"],
 'skull':  ["00000", "hsssd", "0e0e0", "0s0s0", "0d0d0"],
 'hooded': ["0HHH0", "HH0HH", "H000H", "HH0HH", "0HHH0"],
 'helm':   ["00000", "tttdd", "0e0ed", "hsssd", "0sdd0"],
}

def head(cv, ox, oy, x, y, kind='man', skin='n', shade='m', hi='o',
         hair='k', eye='0', flip=False):
    art = HEADS[kind]
    key = {'H': hair, 's': skin, 'h': hi, 'd': shade, 'e': eye,
           'n': shade, 't': 't', '0': '0'}
    w = len(art[0])
    for dy, row in enumerate(art):
        if flip: row = row[::-1]
        for dx, ch in enumerate(row):
            if ch == '.': continue
            cv.set(ox + x - w//2 + dx, oy + y + dy, key.get(ch, ch))

def head_h(kind='man'): return len(HEADS[kind])

# ── Hands: mittens with a thumb, not orbs ────────────────────────────
# 3x4. The thumb notch on one side is the entire reason it reads as a hand.
HANDS = {
 'open':  ["0s0", "hss", "sss", "0s0"],
 'fist':  ["0s0", "hsd", "ssd", "0d0"],
 'grip':  ["ts0", "hss", "0sd", "0d0"],     # closed round a shaft
 'point': ["0s0", "hss", "0sd", "0s0"],
}
def hand(cv, ox, oy, x, y, kind='open', skin='n', shade='m', hi='o', flip=False):
    art = HANDS[kind]
    key = {'s': skin, 'd': shade, 'h': hi, 't': hi, '0': '0'}
    for dy, row in enumerate(art):
        if flip: row = row[::-1]
        for dx, ch in enumerate(row):
            if ch == '.': continue
            cv.set(ox + x - 1 + dx, oy + y - 1 + dy, key.get(ch, ch))
