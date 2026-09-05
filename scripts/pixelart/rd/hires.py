"""Render cards at 3x so generated art sits at its native resolution.

The 42x34 art window was sized for hand-drawing. The generator works at
126x102, and downsampling that by 3 destroys it — Grimgrin turns to mush.
So the frame is rendered at 1x and upscaled 3x (nearest, so it stays crisp
pixel art), and the native 126x102 generation is pasted into the window
untouched. Same frame design, three times the art.
"""
import os, sys, json, math
sys.dont_write_bytecode = True
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)
import art, card, render
from engine import AW, AH
from PIL import Image, ImageDraw, ImageFont

S = 3
meta = json.load(open(f"{ROOT}/refs/meta.json"))
def slug(n): return n.lower().replace(' ','_').replace(',','').replace("'",'')

def hires(name, greek=True):
    base = render.build(name, greek, 1).convert('RGB')       # 48x56 frame
    big = base.resize((base.width*S, base.height*S), Image.NEAREST)
    p = f"{ROOT}/rd/out/{slug(name)}_raw.png"
    if os.path.exists(p):
        a = Image.open(p).convert('RGB')
        if a.size != (AW*S, AH*S): a = a.resize((AW*S, AH*S), Image.NEAREST)
        big.paste(a, (card.ART_X*S, card.ART_Y*S))
    return big

def sheet(names, path, scale=2, cols=4):
    ims = [(n, hires(n)) for n in names]
    w, h = ims[0][1].size; w*=scale; h*=scale
    pad, lab = 16, 26
    rows = math.ceil(len(ims)/cols)
    out = Image.new('RGB', (cols*(w+pad)+pad, rows*(h+pad+lab)+pad), (13,11,20))
    d = ImageDraw.Draw(out)
    f = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 17)
    for i,(n,im) in enumerate(ims):
        x = pad+(i%cols)*(w+pad); y = pad+(i//cols)*(h+pad+lab)
        out.paste(im.resize((w,h), Image.NEAREST), (x,y))
        d.text((x, y+h+5), n, font=f, fill=(0xf2,0xea,0xd9))
    out.save(path); print(path, out.size)

if __name__ == '__main__':
    sheet(sys.argv[2:], sys.argv[1])
