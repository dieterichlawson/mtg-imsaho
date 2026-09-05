import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.dont_write_bytecode = True
import variants

# Round four. I had been reading "pale" as a horror cue. It is not — he is an
# East Asian man with a fair complexion, and that is simply his skin. Worse,
# "grimy" and "filthy" were wrong in the other direction too: the painting is
# refined, not squalid. His gear is expensive ornate silverwork and his hair
# is neatly cut. He is a well-dressed young professional.
#
# So: nothing about the MAN is strange. He is healthy, calm, tidy. All of the
# strangeness lives in the apparatus bolted to him.
WHO = ("a healthy well-groomed young East Asian man in his mid twenties, fair "
       "complexion, neat short black hair, calm composed expression, clean and "
       "expensively dressed, a young professional, NOT undead, not gaunt, not "
       "dirty, not sickly")
GEAR = ("ornate polished silver fittings and a well-cut black leather coat, "
        "refined and costly")
NEG = "painterly oil-painted, muted desaturated grey-green, NOT anime, NOT JRPG, no shining hero armour"
BASE = ("gothic horror pixel art, Innistrad, dark stone interior, the only bright "
        "colour is a mint-teal glow from his device, limited palette, no text. " + NEG)

OPTS = [
 ("levelled at the viewer",
  WHO + ", " + GEAR + ", a heavy occult machine strapped the length of his forearm like a "
  "weapon with a round mint-teal aperture at the wrist, arm raised and levelled straight at "
  "the viewer, his other hand steadying it, unbothered. " + BASE),
 ("chest-up, both devices legible",
  WHO + ", " + GEAR + ", seen from the chest up, a jeweled lens rig with green gems bracketed "
  "over his left eye, and a bulky glowing machine clamped along his forearm held across his "
  "chest, both devices large and clearly readable. " + BASE),
 ("adjusting the mechanism",
  WHO + ", " + GEAR + ", looking down and turning a dial on the heavy apparatus clamped to his "
  "own forearm, mint light spilling between its plates onto his calm face, entirely "
  "matter-of-fact about it. " + BASE),
 ("seated, device across the knee",
  WHO + ", " + GEAR + ", seated on a stone bench in a dark hall with the heavy machine on his "
  "forearm resting across his knee, mint glow at the wrist, green lens rig over one eye, "
  "waiting, composed. " + BASE),
 ("discharge, lit by his own machine",
  WHO + ", " + GEAR + ", the machine on his forearm firing a lance of mint-teal light, the "
  "flash lighting his composed face and neat hair from the side, green lens rig catching it, "
  "coat edged in light, dark vaulted hall. " + BASE),
]
D = f"{os.path.dirname(os.path.abspath(__file__))}/snap4"
os.makedirs(D, exist_ok=True)
paths = []
for i, (lab, p) in enumerate(OPTS):
    out = f"{D}/opt{i+1}.png"; variants.gen(p, 55500+i*173, out); paths.append(out)
variants.sheet("Snapcaster Mage", [[l] for l, _ in OPTS], paths, f"{D}/options.png")
