"""Board mockup: what a real game in progress looks like.

Card faces stay pure identity (art + colour). Live game state — P/T, tapped,
damage, summoning sickness — is drawn as an overlay ON the card, never baked
into it. That's the 'mouse over for more data' split.
"""
import sys, json; sys.path.insert(0,'.')
import art, card, render, font
from engine import Canvas, PAL
from PIL import Image

meta = json.load(open('refs/meta.json'))
CW, CH = card.CW, card.CH

def px_text(im, x, y, s, col=(0xf2,0xea,0xd9)):
    p = im.load()
    for ch in s.upper():
        g = font.F.get(ch)
        if g:
            for dy, row in enumerate(g):
                for dx, b in enumerate(row):
                    if b == '1' and 0 <= x+dx < im.width and 0 <= y+dy < im.height:
                        p[x+dx, y+dy] = col
        x += 4
    return x

def chip(im, x, y, s, fg, bg):
    w = len(s)*4 + 1
    d = im.load()
    for yy in range(y-1, y+7):
        for xx in range(x-1, x+w+1):
            if 0 <= xx < im.width and 0 <= yy < im.height: d[xx, yy] = bg
    px_text(im, x+1, y+1, s, fg)
    return w

def face(name, greek=True):
    return render.build(name, greek, 1)

def permanent(name, pt=None, tapped=False, sick=False, dmg=0, greek=True):
    im = face(name, greek).convert('RGB')
    if dmg:                                     # damage: a red bar eating the frame
        d = im.load()
        for x in range(2, 2 + min(CW-4, dmg*6)):
            d[x, CH-3] = (0xb5,0x34,0x3a); d[x, CH-4] = (0x7a,0x2f,0x2f)
    if pt:  chip(im, CW-1-(len(pt)*4+1), CH-9, pt, (0xf2,0xea,0xd9), (0x0d,0x0b,0x14))
    if sick: chip(im, 3, 3, 'Z', (0x7f,0xc4,0xd9), (0x0d,0x0b,0x14))
    if tapped:
        im = im.rotate(-90, expand=True)
        ov = Image.new('RGB', im.size, (0x1c,0x18,0x26))
        im = Image.blend(im, ov, 0.28)
    return im

def row(im, entries, x0, y0, gap=4, greek=True):
    x = x0
    for e in entries:
        c = permanent(e['name'], e.get('pt'), e.get('tapped'), e.get('sick'),
                      e.get('dmg', 0), greek)
        yy = y0 + (CH - c.height if c.height < CH else 0)
        im.paste(c, (x, yy)); x += c.width + gap
    return x

def render_board(state, path, scale=3, greek=True):
    W, H = 342, 372
    im = Image.new('RGB', (W, H), (0x0d,0x0b,0x14))
    d = im.load()
    for y in range(H):                          # a subtle vertical ground tone
        for x in range(W):
            d[x, y] = (0x14,0x11,0x1c) if (y//2 + x//2) % 2 == 0 else (0x11,0x0f,0x18)
    def bar(y, h, col):
        for yy in range(y, y+h):
            for x in range(W): d[x, yy] = col
    bar(0, 11, (0x2e,0x27,0x40)); bar(H-13, 13, (0x2e,0x27,0x40))
    bar(150, 2, (0x45,0x3b,0x5c))
    px_text(im, 4, 3, state['opp']['name'], (0xb5,0x34,0x3a))
    px_text(im, 96, 3, "%d LIFE" % state['opp']['life'], (0xb5,0x34,0x3a))
    px_text(im, 160, 3, "H%d" % state['opp']['hand'])
    px_text(im, 190, 3, "L%d" % state['opp']['lib'])
    px_text(im, 250, 3, state['phase'], (0xd9,0xc4,0x6a))
    px_text(im, 4, H-10, state['you']['name'], (0x8f,0xbf,0x5a))
    px_text(im, 96, H-10, "%d LIFE" % state['you']['life'], (0x8f,0xbf,0x5a))
    px_text(im, 160, H-10, "H%d" % len(state['you']['hand']))
    px_text(im, 190, H-10, "L%d" % state['you']['lib'])
    row(im, state['opp']['lands'],     4, 14,  gap=-22, greek=greek)
    row(im, state['opp']['creatures'], 4, 88,  greek=greek)
    row(im, state['you']['creatures'], 4, 158, greek=greek)
    row(im, state['you']['lands'],     4, 228, gap=-22, greek=greek)
    px_text(im, 4, 292, "HAND", (0x6b,0x5f,0x80))
    row(im, state['you']['hand'],      4, 300, gap=-14, greek=greek)
    im = im.resize((W*scale, H*scale), Image.NEAREST)
    im.save(path); print(path, im.size)
