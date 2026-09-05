"""Full 32-card contact sheet at 3x with generated art."""
import os, sys, math
sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import hires, json
from PIL import Image, ImageDraw, ImageFont
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
meta = json.load(open(f"{ROOT}/refs/meta.json"))
ORDER = ["Geist of Saint Traft","Doomed Traveler","Chapel Geist","Elite Inquisitor","Midnight Haunting",
 "Snapcaster Mage","Delver of Secrets","Invisible Stalker","Laboratory Maniac","Stitched Drake",
 "Liliana of the Veil","Diregraf Ghoul","Unburial Rites","Bloodline Keeper","Grimgrin, Corpse-Born",
 "Brimstone Volley","Devil's Play","Balefire Dragon","Instigator Gang","Blasphemous Act",
 "Mayor of Avabruck","Kessig Cagebreakers","Garruk Relentless","Spider Spawning","Gatstaf Shepherd",
 "Plains","Island","Swamp","Mountain","Forest","Kessig Wolf Run","Blazing Torch"]
BN = {'W':'WHITE','U':'BLUE','B':'BLACK','R':'RED','G':'GREEN','L':'LAND','A':'ARTIFACT'}
BC = {'W':(0xcf,0xc7,0xc2),'U':(0x3f,0x7f,0xa6),'B':(0xa9,0x7f,0xd0),'R':(0xb5,0x34,0x3a),
      'G':(0x4a,0x80,0x46),'L':(0x8a,0x65,0x40),'A':(0x8f,0x9a,0xa6)}
def f(sz, b=False):
    return ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans%s.ttf'%('-Bold' if b else ''), sz)
ims = [(n, hires.hires(n)) for n in ORDER]
w, h = ims[0][1].size
COLS, PAD, LAB, HDR = 8, 16, 44, 74
rows = math.ceil(len(ims)/COLS)
W = COLS*(w+PAD)+PAD; H = HDR + rows*(h+PAD+LAB) + PAD
out = Image.new('RGB', (W, H), (13,11,20)); d = ImageDraw.Draw(out)
d.rectangle([0,0,W,HDR-14], fill=(0x1c,0x18,0x26))
d.text((PAD,12), "INNISTRAD - 32 PIXEL CARDS", font=f(26,True), fill=(0xf2,0xea,0xd9))
d.text((PAD,44), "art generated with Retro Diffusion from per-card subject prompts; "
                 "frames, palette and layout are the project's own",
       font=f(14), fill=(0x9a,0x8f,0xa8))
for i,(n,im) in enumerate(ims):
    x = PAD+(i%COLS)*(w+PAD); y = HDR+(i//COLS)*(h+PAD+LAB)
    b = meta[n]['bucket']
    out.paste(im, (x,y))
    d.rectangle([x-2,y-2,x+w+1,y+h+1], outline=BC[b], width=2)
    d.text((x, y+h+6), "%02d  %s" % (i, BN[b]), font=f(12,True), fill=BC[b])
    ft = f(16, True)
    while d.textlength(n, font=ft) > w and ft.size > 10: ft = f(ft.size-1, True)
    d.text((x, y+h+22), n, font=ft, fill=(0xf2,0xea,0xd9))
out.save(f"{ROOT}/rd/contact_hi.png"); print("rd/contact_hi.png", out.size)
