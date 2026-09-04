"""Contact sheet pairing each pixel card with the real art it was drawn from."""
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
BN = {'W':'WHITE','U':'BLUE','B':'BLACK','R':'RED','G':'GREEN','L':'LAND','A':'ARTIFACT'}
BC = {'W':(0xcf,0xc7,0xc2),'U':(0x3f,0x7f,0xa6),'B':(0xa9,0x7f,0xd0),'R':(0xb5,0x34,0x3a),
      'G':(0x4a,0x80,0x46),'L':(0x8a,0x65,0x40),'A':(0x8f,0x9a,0xa6)}
def slug(n): return n.lower().replace(' ','_').replace(',','').replace("'",'')
def font(sz, bold=False):
    return ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans%s.ttf'
                              % ('-Bold' if bold else ''), sz)

SCALE, COLS = 5, 4
cards = [(n, render.build(n, True, SCALE)) for n in ORDER]
CWp, CHp = cards[0][1].size
RW = int(CHp * 626/457)                      # ref art crop, height-matched
GAP, PAD, LAB, HDR = 10, 22, 56, 84
PAIR = RW + GAP + CWp
rows = math.ceil(len(cards)/COLS)
W = COLS*(PAIR+PAD) + PAD
H = HDR + rows*(CHp+PAD+LAB) + PAD

im = Image.new('RGB', (W, H), (0x0d,0x0b,0x14))
d = ImageDraw.Draw(im)
d.rectangle([0,0,W,HDR-14], fill=(0x1c,0x18,0x26))
d.text((PAD, 14), "INNISTRAD — SOURCE ART vs PIXEL CARD", font=font(28, True), fill=(0xf2,0xea,0xd9))
d.text((PAD, 50), "left: the original Wizards illustration (Scryfall art crop).   "
                  "right: the 42x34 hand-authored pixel card drawn from it.",
       font=font(15), fill=(0x9a,0x8f,0xa8))

for i, (n, ci) in enumerate(cards):
    x = PAD + (i % COLS)*(PAIR+PAD)
    y = HDR + (i // COLS)*(CHp+PAD+LAB)
    b = meta[n]['bucket']
    try:
        ref = Image.open('refs/%s_art.jpg' % slug(n)).convert('RGB').resize((RW, CHp))
        im.paste(ref, (x, y))
    except Exception as e:
        d.rectangle([x, y, x+RW, y+CHp], fill=(0x2e,0x27,0x40))
    d.rectangle([x-2, y-2, x+RW+1, y+CHp+1], outline=(0x45,0x3b,0x5c), width=2)
    im.paste(ci, (x+RW+GAP, y))
    d.rectangle([x+RW+GAP-3, y-3, x+RW+GAP+CWp+2, y+CHp+2], outline=BC[b], width=3)
    ty = y + CHp + 9
    d.text((x, ty), "%02d" % i, font=font(14, True), fill=(0x6b,0x5f,0x80))
    d.text((x+28, ty), BN[b], font=font(14, True), fill=BC[b])
    d.text((x, ty+18), n, font=font(20, True), fill=(0xf2,0xea,0xd9))
    d.text((x+RW+GAP, ty+20), "art: %s" % meta[n].get('artist',''),
           font=font(13), fill=(0x6b,0x5f,0x80))
im.save('out/compare.png'); print('out/compare.png', im.size)
