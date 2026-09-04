import sys
sys.dont_write_bytecode = True
import json, sys, math
from PIL import Image, ImageDraw
sys.path.insert(0,'.')
import art, card
meta = json.load(open('refs/meta.json'))
def cost_syms(mc):
    out=[]
    for tok in mc.replace('}','').split('{'):
        if not tok: continue
        if tok in ('W','U','B','R','G'): out.append(tok)
        elif tok.isdigit() or tok=='X': out.append('X')
        else: out.append('C')
    return ''.join(out[:5])
def build(name, greek=True, scale=6):
    m = meta[name]
    cv = art.finish(name, art.ART[name]())
    c = card.make_card(cv.rows(), m['bucket'], name, m['type_line'],
                       cost_syms(m.get('mana_cost','')), greek=greek)
    return card.to_image(c, scale)
def sheet(names, path, cols=5, scale=6, greek=True, label=True):
    ims = [(n, build(n, greek, scale)) for n in names]
    w, h = ims[0][1].size
    pad, lab = 10, (14 if label else 0)
    rows = math.ceil(len(ims)/cols)
    im = Image.new('RGB', (cols*(w+pad)+pad, rows*(h+pad+lab)+pad), (16,14,20))
    d = ImageDraw.Draw(im)
    for i,(n,ci) in enumerate(ims):
        x = pad + (i%cols)*(w+pad); y = pad + (i//cols)*(h+pad+lab)
        im.paste(ci, (x,y))
        if label: d.text((x+1, y+h+2), n[:26], fill=(200,195,185))
    im.save(path); print(path, im.size)
if __name__ == '__main__':
    names = sys.argv[2:] or list(art.ART)
    sheet(names, sys.argv[1])
