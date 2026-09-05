import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.dont_write_bytecode = True
import variants

# Round five, from a tight crop of the face in the source painting.
#
# THE GLASSES, exactly: round WIRE-RIMMED spectacles. Over one eye, a plain
# round clear lens in a thin brass wire frame. Over the OTHER eye the lens is
# replaced by a dark ornate disc in a heavy brass bezel, ringed with round
# green cabochon gems that trail outward toward his temple. A thin brass
# armature crosses his brow just above the eyebrows, studded with small
# fittings, tying the two together. It is jewellery and instrument at once.
GLASSES = ("round thin wire-rimmed brass spectacles, a plain clear round lens over "
           "his right eye, and over his left eye a dark ornate disc in a heavy brass "
           "bezel ringed with round green gemstones that trail toward his temple, a "
           "slim studded brass armature crossing his brow above the eyebrows")

# THE FACE: he is a real man's likeness, not a fantasy lead. Broad and plain,
# heavy brow, wide nose, soft jaw, slightly jowly, hair receding at the
# temples. Ordinary. Every previous round made him a handsome model.
FACE = ("a PLAIN unglamorous middle-aged East Asian man, broad face, heavy brow, "
        "wide nose, soft jowly jaw, thinning side-parted black hair receding at "
        "the temples, forgettable features, an academic not a hero, NOT handsome, "
        "NOT a pretty young man, no chiselled jaw")

# THE REGISTER: aristocratic techno-horror, not cyberpunk. Antique brass and
# tarnished silver, baroque metalwork, candle-era. No neon, no chrome.
REG = ("aristocratic gothic techno-horror, antique tarnished brass and silver, "
       "baroque engraved metalwork, candlelit 19th century occultist, "
       "NOT cyberpunk, NOT neon, no chrome, no sci-fi, NOT anime, NOT JRPG, "
       "painterly oil-painted, muted desaturated grey-green")
BASE = ("gothic horror pixel art, Innistrad, dark stone hall, the only bright colour "
        "is a dim mint-teal glow, limited palette, no text. " + REG)

OPTS = [
 ("portrait, glasses hero",
  FACE + ", " + GLASSES + ", chest-up formal portrait, high collar and cravat, an ornate "
  "engraved brass instrument clamped to his forearm with a dim mint glow, composed and "
  "unremarkable. " + BASE),
 ("seated aristocrat, device on knee",
  FACE + ", " + GLASSES + ", seated in a high-backed chair in a dark panelled study, tailored "
  "black frock coat, a heavy engraved brass apparatus buckled along his forearm resting on his "
  "knee, dim mint light at the wrist, candles. " + BASE),
 ("adjusting the eyepiece",
  FACE + ", " + GLASSES + ", raising one hand to adjust the gemmed brass disc over his eye, "
  "the green stones catching the light, a bulky engraved brass mechanism strapped to his other "
  "forearm, dark library behind. " + BASE),
 ("three-quarter, instrument raised",
  FACE + ", " + GLASSES + ", three-quarter view, holding up the forearm apparatus so its "
  "engraved brass plates and dim mint aperture are clearly visible, tarnished silver gorget, "
  "black coat, entirely matter-of-fact. " + BASE),
 ("close, lit from the wrist",
  FACE + ", " + GLASSES + ", close portrait lit from below by the dim mint glow of the brass "
  "instrument on his own raised forearm, the green gems of the eyepiece reflecting it, "
  "gothic hall dark behind him. " + BASE),
]
D = f"{os.path.dirname(os.path.abspath(__file__))}/snap5"
os.makedirs(D, exist_ok=True)
paths = []
for i, (lab, p) in enumerate(OPTS):
    out = f"{D}/opt{i+1}.png"; variants.gen(p, 90100+i*151, out); paths.append(out)
variants.sheet("Snapcaster Mage", [[l] for l, _ in OPTS], paths, f"{D}/options.png")
