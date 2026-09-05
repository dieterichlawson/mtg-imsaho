"""Map a generated PNG onto the card palette so it drops into the frames.

The generator is already constrained by input_palette, but nothing guarantees
exact hex matches, so every pixel is snapped to its nearest ramp entry. The
result is a grid of palette characters — exactly what art.py hands the card
compositor, so nothing downstream has to change.
"""
import os, sys
sys.dont_write_bytecode = True
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)
from engine import PAL, AW, AH
from PIL import Image

RAMP = [(k, v) for k, v in PAL.items() if v]

def nearest(px):
    r, g, b = px
    best, bd = '0', 1 << 30
    for k, (cr, cg, cb) in RAMP:
        d = (r-cr)**2 + (g-cg)**2 + (b-cb)**2
        if d < bd: bd, best = d, k
    return best

def rows_for(name_slug, d=None):
    p = os.path.join(d or os.path.join(ROOT, 'rd', 'out'), name_slug + '.png')
    if not os.path.exists(p): return None
    im = Image.open(p).convert('RGB')
    if im.size != (AW, AH): im = im.resize((AW, AH), Image.NEAREST)
    px = im.load()
    return [''.join(nearest(px[x, y]) for x in range(AW)) for y in range(AH)]
