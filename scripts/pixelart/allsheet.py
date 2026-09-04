import sys, math, json; sys.path.insert(0,'.')
import art, render
from PIL import Image, ImageDraw
ORDER = ["Geist of Saint Traft","Doomed Traveler","Chapel Geist","Elite Inquisitor","Midnight Haunting",
 "Snapcaster Mage","Delver of Secrets","Invisible Stalker","Laboratory Maniac","Stitched Drake",
 "Liliana of the Veil","Diregraf Ghoul","Unburial Rites","Bloodline Keeper","Grimgrin, Corpse-Born",
 "Brimstone Volley","Devil's Play","Balefire Dragon","Instigator Gang","Blasphemous Act",
 "Mayor of Avabruck","Kessig Cagebreakers","Garruk Relentless","Spider Spawning","Gatstaf Shepherd",
 "Plains","Island","Swamp","Mountain","Forest","Kessig Wolf Run","Blazing Torch"]
greek = '--named' not in sys.argv
half = 16
for part,(a,b) in enumerate([(0,half),(half,32)]):
    sel = ORDER[a:b]
    ims=[(n, render.build(n, greek, 9)) for n in sel]
    w,h = ims[0][1].size; cols=4; pad=12; lab=16
    rows = math.ceil(len(ims)/cols)
    im = Image.new('RGB',(cols*(w+pad)+pad, rows*(h+pad+lab)+pad),(16,14,20))
    d = ImageDraw.Draw(im)
    for i,(n,ci) in enumerate(ims):
        x=pad+(i%cols)*(w+pad); y=pad+(i//cols)*(h+pad+lab)
        im.paste(ci,(x,y)); d.text((x+2,y+h+3), f"{a+i}. {n}", fill=(205,200,190))
    suffix = '' if greek else '_named'
    im.save(f'set{part+1}{suffix}.png'); print(f'set{part+1}{suffix}.png', im.size)
