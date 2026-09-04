import sys
sys.dont_write_bytecode = True
import json, sys, math
from PIL import Image, ImageDraw
meta = json.load(open('refs/meta.json'))
names = list(meta)
def slug(n): return n.lower().replace(' ','_').replace(',','').replace("'",'')
group = sys.argv[1]; start, end = int(sys.argv[2]), int(sys.argv[3])
sel = names[start:end]
CW, CHh = 420, 306
cols = 4; rows = math.ceil(len(sel)/cols)
im = Image.new('RGB', (cols*CW, rows*(CHh+20)), (20,18,24))
d = ImageDraw.Draw(im)
for i, n in enumerate(sel):
    try: a = Image.open(f'refs/{slug(n)}_art.jpg').convert('RGB').resize((CW-8, CHh-8))
    except Exception as e: print('skip', n, e); continue
    x, y = (i%cols)*CW, (i//cols)*(CHh+20)
    im.paste(a, (x+4, y+4))
    d.text((x+6, y+CHh+2), f"{i+start}. {n}  [{meta[n]['bucket']}]", fill=(230,225,215))
im.save(f'refs/sheet_{group}.png')
print('refs/sheet_%s.png' % group, im.size, len(sel), 'cards')
