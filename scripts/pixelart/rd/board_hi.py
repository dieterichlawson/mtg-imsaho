"""Board mockup at 3x with the generated art."""
import os, sys, json
sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__)); ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE); sys.path.insert(0, ROOT)
import hires, font
from PIL import Image
S = 3
CW, CH = 48*S, 56*S

def px_text(im, x, y, s, col=(0xf2,0xea,0xd9), sc=2):
    p = im.load()
    for ch in s.upper():
        g = font.F.get(ch)
        if g:
            for dy,row in enumerate(g):
                for dx,b in enumerate(row):
                    if b=='1':
                        for oy in range(sc):
                            for ox in range(sc):
                                xx,yy = x+dx*sc+ox, y+dy*sc+oy
                                if 0<=xx<im.width and 0<=yy<im.height: p[xx,yy]=col
        x += 4*sc
    return x

def chip(im, x, y, s, fg, bg, sc=2):
    w = (len(s)*4+1)*sc; d = im.load()
    for yy in range(y-2, y+5*sc+2):
        for xx in range(x-2, x+w+2):
            if 0<=xx<im.width and 0<=yy<im.height: d[xx,yy]=bg
    px_text(im, x, y, s, fg, sc)

def permanent(e):
    im = hires.hires(e['name']).convert('RGB')
    if e.get('dmg'):
        d = im.load()
        for x in range(4, min(CW-4, 4+e['dmg']*18)):
            for yy in (CH-7, CH-8, CH-9): d[x,yy] = (0xb5,0x34,0x3a)
    if e.get('pt'):  chip(im, CW-4-(len(e['pt'])*4+1)*2, CH-24, e['pt'], (0xf2,0xea,0xd9), (0x0d,0x0b,0x14))
    if e.get('sick'): chip(im, 8, 8, 'Z', (0x7f,0xc4,0xd9), (0x0d,0x0b,0x14))
    if e.get('tapped'):
        im = im.rotate(-90, expand=True)
        im = Image.blend(im, Image.new('RGB', im.size, (0x1c,0x18,0x26)), 0.28)
    return im

def row(im, entries, x0, y0, gap):
    x = x0
    for e in entries:
        c = permanent(e)
        im.paste(c, (x, y0 + (CH - c.height if c.height < CH else 0)))
        x += c.width + gap

def render(state, path):
    W, H = 342*S, 372*S
    im = Image.new('RGB', (W, H), (0x11,0x0f,0x18))
    d = im.load()
    for y in range(0, 11*S):
        for x in range(W): d[x,y] = (0x2e,0x27,0x40)
    for y in range(H-13*S, H):
        for x in range(W): d[x,y] = (0x2e,0x27,0x40)
    for y in range(150*S, 152*S):
        for x in range(W): d[x,y] = (0x45,0x3b,0x5c)
    px_text(im, 4*S, 3*S, state['opp']['name'], (0xb5,0x34,0x3a))
    px_text(im, 96*S, 3*S, "%d LIFE" % state['opp']['life'], (0xb5,0x34,0x3a))
    px_text(im, 250*S, 3*S, state['phase'], (0xd9,0xc4,0x6a))
    px_text(im, 4*S, H-10*S, state['you']['name'], (0x8f,0xbf,0x5a))
    px_text(im, 96*S, H-10*S, "%d LIFE" % state['you']['life'], (0x8f,0xbf,0x5a))
    row(im, state['opp']['lands'],     4*S, 14*S,  -22*S)
    row(im, state['opp']['creatures'], 4*S, 88*S,   4*S)
    row(im, state['you']['creatures'], 4*S, 158*S,  4*S)
    row(im, state['you']['lands'],     4*S, 228*S, -22*S)
    px_text(im, 4*S, 292*S, "HAND", (0x6b,0x5f,0x80))
    row(im, state['you']['hand'],      4*S, 300*S, -14*S)
    im.save(path); print(path, im.size)

if __name__ == '__main__':
    sys.path.insert(0, ROOT)
    from scene import STATE
    render(STATE, f"{ROOT}/rd/board_hi.png")
