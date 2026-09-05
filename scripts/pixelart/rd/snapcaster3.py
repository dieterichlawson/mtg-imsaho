import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.dont_write_bytecode = True
import variants

# Round three. Round one was too clean and high-fantasy; round two went weird
# in the right direction but overshot into undead — "gaunt", "pale", "filthy"
# plus the sickly mint made corpses. The target is between them: a LIVING
# young man, ordinary face, dark hair, who happens to have a heavy piece of
# occult machinery clamped to his forearm like a weapon. The weirdness should
# come from the apparatus, not from him being a ghoul.
WHO = ("a living young man with an ordinary alert face, short black hair, "
       "normal skin, mid twenties, clearly alive and unhurt, NOT undead, "
       "not a corpse, not a skull, not gaunt")
NEG = ("grimy painterly oil-painted, muted desaturated grey-green, occult "
       "technology, weird fiction, NOT anime, NOT JRPG, no fantasy armour, "
       "no shining plate")
BASE = ("gothic horror pixel art, Innistrad, dark stone interior, the only bright "
        "colour is mint-teal glow, limited palette, no text. " + NEG)

OPTS = [
 ("gun arm, aimed at viewer",
  WHO + ", wearing a heavy machine strapped the length of his forearm like a rifle, a round "
  "aperture at the wrist glowing mint-teal, arm raised and levelled straight at the viewer, "
  "brass clamps and leather straps, his other hand steadying it. " + BASE),
 ("gun arm, reloading",
  WHO + ", looking down at the bulky apparatus clamped to his forearm and working a lever on "
  "the side of it with his other hand, mint light spilling out between the plates, "
  "concentrating, a mechanic's expression. " + BASE),
 ("gun arm + lens rig, three-quarter",
  WHO + ", a jeweled lens rig bracketed over his left eye with green gems, and a heavy glowing "
  "device clamped along his right forearm, three-quarter view from the chest up, both the eye "
  "rig and the arm device large and clearly visible. " + BASE),
 ("low angle, device foregrounded",
  WHO + ", seen from below with the machine on his forearm thrust forward into the foreground "
  "so it fills a third of the frame, mint aperture glowing at the wrist, his face small and "
  "calm behind it, dark vaulted ceiling above. " + BASE),
 ("recoil, lit by his own device",
  WHO + ", braced as the machine on his forearm discharges a bolt of mint-teal light, the flash "
  "lighting his ordinary face and his black coat from the side, hair blown back, a lens rig "
  "over one eye catching the light. " + BASE),
]
D = f"{os.path.dirname(os.path.abspath(__file__))}/snap3"
os.makedirs(D, exist_ok=True)
paths = []
for i, (lab, p) in enumerate(OPTS):
    out = f"{D}/opt{i+1}.png"; variants.gen(p, 31300+i*97, out); paths.append(out)
variants.sheet("Snapcaster Mage", [[l] for l, _ in OPTS], paths, f"{D}/options.png")
