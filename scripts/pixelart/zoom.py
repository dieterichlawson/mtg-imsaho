import sys
sys.dont_write_bytecode = True
import sys; sys.path.insert(0,'.')
import art, card, render
from PIL import Image
names = sys.argv[2:]
ims = [render.build(n, True, 12) for n in names]
w,h = ims[0].size
im = Image.new('RGB',(len(ims)*(w+12)+12, h+24),(16,14,20))
for i,c in enumerate(ims): im.paste(c,(12+i*(w+12),12))
im.save(sys.argv[1]); print(sys.argv[1], im.size)
