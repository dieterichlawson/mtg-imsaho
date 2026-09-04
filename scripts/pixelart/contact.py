"""One contact sheet: all 32 cards, grouped by colour, with legible names."""
import sys
sys.dont_write_bytecode = True
import json, math
sys.path.insert(0, '.')
import art, render
from PIL import Image, ImageDraw, ImageFont

meta = json.load(open('refs/meta.json'))
ORDER = ["Geist of Saint Traft","Doomed Traveler","Chapel Geist","Elite Inquisitor","Midnight Haunting",
 "Snapcaster Mage","Delver of Secrets","Invisible Stalker","Laboratory Maniac","Stitched Drake",
 "Liliana of the Veil","Diregraf Ghoul","Unburial Rites","Bloodline Keeper","Grimgrin, Corpse-Born",
 "Brimstone Volley","Devil's Play","Balefire Dragon","Instigator Gang","Blasphemous Act",
 "Mayor of Avabruck","Kessig Cagebreakers","Garruk Relentless","Spider Spawning","Gatstaf Shepherd",
 "Plains","Island","Swamp","Mountain","Forest","Kessig Wolf Run","Blazing Torch"]
BUCKET_NAME = {'W':'WHITE','U':'BLUE','B':'BLACK','R':'RED','G':'GREEN','L':'LAND','A':'ARTIFACT'}
BUCKET_COL  = {'W':(0xcf,0xc7,0xc2),'U':(0x3f,0x7f,0xa6),'B':(0xa9,0x7f,0xd0),
               'R':(0xb5,0x34,0x3a),'G':(0x4a,0x80,0x46),'L':(0x8a,0x65,0x40),'A':(0x8f,0x9a,0xa6)}

def font(sz, bold=False):
    p = '/usr/share/fonts/truetype/dejavu/DejaVuSans%s.ttf' % ('-Bold' if bold else '')
    return ImageFont.truetype(p, sz)

SCALE = 7
greek = '--named' not in sys.argv
COLS  = 8
cards = [(n, render.build(n, greek, SCALE)) for n in ORDER]
CWp, CHp = cards[0][1].size
PAD, LAB, HDR = 18, 46, 76
rows = math.ceil(len(cards)/COLS)
W = COLS*(CWp+PAD) + PAD
H = HDR + rows*(CHp+PAD+LAB) + PAD + 34

im = Image.new('RGB', (W, H), (0x0d,0x0b,0x14))
d  = ImageDraw.Draw(im)
d.rectangle([0,0,W,HDR-12], fill=(0x1c,0x18,0x26))
d.text((PAD, 12), "INNISTRAD — 32 PIXEL CARDS", font=font(26, True), fill=(0xf2,0xea,0xd9))
d.text((PAD, 42), "hand-authored 42x34 art  ·  shared 31-colour gothic ramp  ·  "
                  "aperture / candle-ledger / contact-shadow grammar",
       font=font(15), fill=(0x9a,0x8f,0xa8))

for i, (n, ci) in enumerate(cards):
    x = PAD + (i % COLS)*(CWp+PAD)
    y = HDR + (i // COLS)*(CHp+PAD+LAB)
    b = meta[n]['bucket']
    d.rectangle([x-3, y-3, x+CWp+2, y+CHp+2], outline=BUCKET_COL[b], width=2)
    im.paste(ci, (x, y))
    ty = y + CHp + 8
    d.text((x, ty), "%02d" % i, font=font(13, True), fill=(0x6b,0x5f,0x80))
    d.text((x+26, ty), BUCKET_NAME[b], font=font(13, True), fill=BUCKET_COL[b])
    name = n if len(n) < 26 else n.split(',')[0]
    f = font(17, True)
    while d.textlength(name, font=f) > CWp and f.size > 11:
        f = font(f.size-1, True)
    d.text((x, ty+17), name, font=f, fill=(0xf2,0xea,0xd9))

d.text((PAD, H-26), "card face = identity only (name is deliberately greeked; live P/T, tapped and "
                    "damage are board overlays, never baked in)" if greek else
                    "named variant: 3x5 pixel font in the title bar",
       font=font(14), fill=(0x6b,0x5f,0x80))
out = 'out/contact.png' if greek else 'out/contact_named.png'
im.save(out); print(out, im.size)
