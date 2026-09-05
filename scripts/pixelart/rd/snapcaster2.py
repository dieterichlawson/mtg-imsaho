import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.dont_write_bytecode = True
import variants

# Round two. The first five all drifted high-fantasy / JRPG because the
# prompts kept saying "armour" and "pauldrons", which is the wrong register
# entirely. The interesting thing about this character is not that he is
# armoured — it is that he is a scholar with strange machinery bolted onto
# him. The forearm piece reads like a weapon or an instrument, not a bracer,
# and the eyepiece looks surgically fixed to his skull.
NEG = ("grimy painterly oil-painted, unglamorous, NOT anime, NOT JRPG, "
       "not a knight, no fantasy armour, no shining plate, muted desaturated "
       "grey-green, occult technology, weird fiction, uneasy")
BASE = "gothic horror pixel art, Innistrad, dark stone interior, the only bright colour is a sickly mint-teal glow, limited palette, no text. " + NEG

OPTS = [
 ("wrist weapon, aimed",
  "a pale scholar in a filthy black coat with a heavy machine strapped along his forearm like "
  "a weapon, a round aperture at the wrist glowing sickly mint-teal, arm raised and aimed "
  "forward at the viewer, brass clamps and leather straps holding it to his flesh. " + BASE),
 ("surgically grafted eyepiece",
  "close on the face of a gaunt scholar with a heavy lens apparatus bolted directly into his "
  "skull around one eye, metal brackets and screws set into the bone at his temple, small "
  "green gems glowing in it, the eye behind the lens too large, unsettling. " + BASE),
 ("occult technician at work",
  "a stooped unglamorous man in a grimy black coat operating a bulky arcane instrument clamped "
  "to his own forearm, adjusting a dial on it with his other hand, sickly mint light leaking "
  "between his fingers, a workshop of apparatus behind him, not heroic at all. " + BASE),
 ("half man half apparatus",
  "a mage whose left side is a man and whose right side is machinery, a lens rig bracketed over "
  "one eye and a heavy glowing device replacing the forearm, the join between flesh and metal "
  "visible and wrong, cold mint light from the device. " + BASE),
 ("instrument fired, recoil",
  "a thin scholar bracing himself as the machine on his forearm discharges, a lance of sickly "
  "mint-teal light firing from the wrist aperture, his coat and hair blown back by it, his "
  "bracketed lens eye lit by the flash, dark hall. " + BASE),
]
D = f"{os.path.dirname(os.path.abspath(__file__))}/snap2"
os.makedirs(D, exist_ok=True)
paths = []
for i, (lab, p) in enumerate(OPTS):
    out = f"{D}/opt{i+1}.png"; variants.gen(p, 7700+i*211, out); paths.append(out)
variants.sheet("Snapcaster Mage", [[l] for l, _ in OPTS], paths, f"{D}/options.png")
