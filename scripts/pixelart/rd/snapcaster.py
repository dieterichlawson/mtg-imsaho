import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.dont_write_bytecode = True
import variants

# Details read off the actual painting: an Asian man with slicked black hair;
# round spectacles whose left lens is an ornate GREEN-GEMMED monocle on a
# bracket curving over the brow to the temple; enormous flared silver
# pauldrons with faces embossed on them; a silver gorget with a scallop-shell
# boss; a black leather coat of round buttons; and on the forearm a big
# ornate silver vambrace holding a glowing pale MINT-TEAL panel. The mint
# glow is the only bright thing on an otherwise grey-green card.
BASE = ("gothic horror pixel art, Innistrad, dark stone columns, desaturated "
        "grey-green, the only bright colour is pale mint-teal glow, limited "
        "palette, no text")
OPTS = [
 ("tight bust, monocle hero",
  "extreme close portrait of a young Asian mage filling the frame, slicked black hair, "
  "an ornate jeweled monocle set with bright green gems clamped over his left eye on a "
  "silver bracket curving over his eyebrow to his temple, huge flared silver pauldrons "
  "cropped at the edges of frame, calm expressionless face. " + BASE),
 ("forearm raised, lit from below",
  "a young Asian mage holding his left forearm up across his chest, a large ornate silver "
  "vambrace on it containing a glowing pale mint-teal panel of light, the glow lighting his "
  "face from below, green-gemmed monocle over one eye, huge silver pauldrons. " + BASE),
 ("mid-cast, energy spilling",
  "a young Asian mage caught mid-spell, pale mint-teal energy pouring out of an ornate "
  "silver device on his forearm, black leather coat flaring, a dark staff in his other hand, "
  "green-gemmed monocle, gothic columns behind. " + BASE),
 ("near-silhouette, two lights",
  "a mage almost entirely in shadow in a dark gothic hall, only two things lit: a glowing "
  "mint-teal panel on his forearm device and the green gems of the monocle over his eye, "
  "the silhouette of enormous flared pauldrons. " + BASE),
 ("heraldic full figure",
  "a young Asian mage standing square between two dark stone columns, enormous flared silver "
  "pauldrons with faces embossed on them, ornate silver gorget, black buttoned leather coat, "
  "glowing mint-teal device on his forearm, green-gemmed monocle, a gargoyle statue behind. " + BASE),
]
os.makedirs(f"{os.path.dirname(os.path.abspath(__file__))}/snap", exist_ok=True)
D = f"{os.path.dirname(os.path.abspath(__file__))}/snap"
paths = []
for i, (lab, p) in enumerate(OPTS):
    out = f"{D}/opt{i+1}.png"; variants.gen(p, 1000+i*137, out); paths.append(out)
variants.sheet("Snapcaster Mage", [[l] for l, _ in OPTS], paths, f"{D}/options.png")
