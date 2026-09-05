"""Generate the 32 card illustrations with Retro Diffusion, img2img from the
real Scryfall art.

Three API parameters do the three things hand-drawing failed at:
  input_image + strength   composition and identity come from the actual
                           painting rather than from my recollection of it
  reference_images         one house style enforced across all 32
  input_palette            output constrained to the 31-colour ramp the card
                           frames already use, so results drop straight in

Usage:
  RD_KEY=rdpk-... python3 rd/rdgen.py --estimate         # free dry run
  RD_KEY=rdpk-... python3 rd/rdgen.py --pilot            # 4 cards only
  RD_KEY=rdpk-... python3 rd/rdgen.py                    # all 32
  RD_KEY=rdpk-... python3 rd/rdgen.py --styles           # list styles
"""
import base64, io, json, os, sys, time, urllib.error, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
API  = "https://api.retrodiffusion.ai/v2"
KEY  = os.environ.get("RD_KEY", "").strip()
AW, AH = 42, 34                      # the card art window
SCALE  = int(os.environ.get("RD_SCALE", "3"))   # generate at 3x, downsample
STYLE  = os.environ.get("RD_STYLE", "rd_plus__default")
STRENGTH = float(os.environ.get("RD_STRENGTH", "0.9"))
SEED_MODE = os.environ.get("RD_SEED_IMAGE", "0") == "1"

sys.path.insert(0, ROOT)
from PIL import Image

ORDER = ["Geist of Saint Traft","Doomed Traveler","Chapel Geist","Elite Inquisitor","Midnight Haunting",
 "Snapcaster Mage","Delver of Secrets","Invisible Stalker","Laboratory Maniac","Stitched Drake",
 "Liliana of the Veil","Diregraf Ghoul","Unburial Rites","Bloodline Keeper","Grimgrin, Corpse-Born",
 "Brimstone Volley","Devil\'s Play","Balefire Dragon","Instigator Gang","Blasphemous Act",
 "Mayor of Avabruck","Kessig Cagebreakers","Garruk Relentless","Spider Spawning","Gatstaf Shepherd",
 "Plains","Island","Swamp","Mountain","Forest","Kessig Wolf Run","Blazing Torch"]
PILOT = ["Doomed Traveler","Liliana of the Veil","Grimgrin, Corpse-Born","Forest"]

# One shared style clause on every prompt — this is what makes 32 separate
# generations read as one set.
STYLE_CLAUSE = ("gothic horror pixel art, Innistrad, moonlit night, muted blacks and "
                "deep greys, cold bone highlights, one blood-red accent, heavy shadow, "
                "woodcut feel, limited palette, strong silhouette, no text, no border")

# Per-card SUBJECT descriptions, written from actually looking at each of the
# 32 source paintings earlier in this project. This is the point of the whole
# approach: the model is told WHAT THE CARD IS, and composes it freshly, so
# the output is original pixel art that carries the card's identity — not a
# filtered copy of Wizards' painting.
PROMPTS = {
 "Geist of Saint Traft": "the translucent glowing ghost of a hooded saint, semi-transparent and see-through, hovering above the floor with no legs, his edges dissolving into cold light, a field of floating candle flames around him in a dark crypt, spectral apparition not a living man",
 "Doomed Traveler": "a lone soldier with a tall spear walking a cobbled road at night, a village with lit windows on the ridge above, full moon",
 "Chapel Geist": "an empty set of billowing white funeral robes floating in mid-air with NO BODY inside them, the fabric twisting and unfurling weightlessly as if underwater, sleeves hanging empty, a hanging chain and pendant, dark chapel, hollow, nobody is wearing it",
 "Elite Inquisitor": "an armoured figure in a pale storm-coat raising a thin rapier, tall gothic windows, a wolf trophy on the wall",
 "Midnight Haunting": "two thin wailing spectral women streaming out through a lit window as a ribbon of pale mist, their forms trailing away into vapour with no solid bodies and no legs, cold white against warm firelight, eerie not cute",
 "Snapcaster Mage": "a young mage in ornate silver and black armour with an elaborate glowing cyan machine clamped to his forearm, brass tubes lenses and dials, a jeweled monocle over one eye, arcane technology, gothic castle behind",
 "Delver of Secrets": "a bald scholar in a dim study holding a live praying mantis on his open palm, the walls behind him covered in pinned insects, moths and beetles in cases, specimen jars, unsettling entomologist",
 "Invisible Stalker": "an empty long coat and a wide-brimmed hat worn by an INVISIBLE man walking forward, the hat floating exactly where a head would be with a clear gap of nothing between hat and collar, rain outlining a body that is not there",
 "Laboratory Maniac": "a wild-eyed scientist at a machine with both arms flung up, blue lightning arcing overhead, green reagent vials",
 "Stitched Drake": "a skeletal undead drake with tattered membrane wings flying over grey ruins, pale moon behind",
 "Liliana of the Veil": "a pale woman with long dark hair in a deep magenta corset gown, an orange flame burning in each open palm",
 "Diregraf Ghoul": "the reanimated corpse of a young woman in a torn dress lurching forward, a broken sword still run through her chest, pale dead skin, head lolling to one side, dark woods",
 "Unburial Rites": "a corpse rising from a stone sarcophagus with its head hanging backwards upside down staring straight at the viewer, mouth open, a robed necromancer leaning over it, cold crypt, deeply unsettling",
 "Bloodline Keeper": "an elegant beautiful male vampire lord with long dark hair and fine aristocratic clothing, floating gracefully in mid-air with his cloak drifting below him, refined and cruel, bats around him, an enormous pale green moon",
 "Grimgrin, Corpse-Born": "a huge hulking stitched zombie with iron shoulder plates and a chain hooked to one arm, dead trees behind",
 "Brimstone Volley": "flaming meteors streaking down on a hard diagonal onto a burning ridge, dead trees in silhouette against the fire",
 "Devil\'s Play": "a small horned devil crouched on a fallen beam pouring a stream of fire down onto burning wreckage",
 "Balefire Dragon": "a black dragon with spread wings against a blood-red sky, breathing fire, iron grave-crosses below",
 "Instigator Gang": "three angry brawlers with fists raised, khaki and sepia, a mob closing in",
 "Blasphemous Act": "a cathedral interior awash with blood, bodies strewn down the steps, blood pooling and running down the stairs, red banners, a lone armoured figure standing in the carnage",
 "Mayor of Avabruck": "a man in a dark coat leaning on a desk in a lamplit study, a black hound at his knee, cold window behind",
 "Kessig Cagebreakers": "men in tricorn hats standing OUTSIDE an iron cage in the foreground seen from behind, hauling the cage door open, a snarling wolf inside the cage behind the bars, torchlight",
 "Garruk Relentless": "an enormous hulking beast hunter over eight feet tall, a rusted iron helm covering his hair and the top half of his face, heavy fur mantle over massive shoulders, a huge notched axe, scarred and menacing, pale forest",
 "Spider Spawning": "dozens of large spiders swarming down a pitch dark forest slope, a mass of long legs, overwhelming numbers, almost total darkness with only the faintest moonlight",
 "Gatstaf Shepherd": "a shepherd with a tall crook standing among a flock of sheep on a hillside at dusk",
 "Plains": "a wide open moonlit grassland plain, fields and low stone walls stretching to a distant horizon, a lone farmhouse, huge sky, open country",
 "Island": "a tall white waterfall plunging into a dark gorge between sheer cliffs",
 "Swamp": "a black stagnant bog, murky standing water filling the frame, half-submerged rotting roots, mist lying on the water, a drowned fence",
 "Mountain": "a dark rocky gorge with a small low sun burning in a warm slit of sky",
 "Forest": "a dense dark ancient forest, huge crowded tree trunks filling the frame, tangled canopy overhead, violet mist between the trees, no people",
 "Kessig Wolf Run": "an enormous wolf silhouetted on a rocky ridge above a dark valley, moon behind",
 "Blazing Torch": "a hooded figure seen from behind holding a burning torch aloft, sunset sky, a village far below",
}

def slug(n): return n.lower().replace(' ','_').replace(',','').replace("'",'')

def b64(path_or_img, box=None):
    im = path_or_img if isinstance(path_or_img, Image.Image) else Image.open(path_or_img)
    im = im.convert('RGB')
    if box: im = im.resize(box, Image.LANCZOS)
    buf = io.BytesIO(); im.save(buf, format='PNG')
    return base64.b64encode(buf.getvalue()).decode()

def crop_to_art(path):
    """Centre-crop the art crop to the card window's aspect, then upscale."""
    im = Image.open(path).convert('RGB')
    want = AW / AH
    w, h = im.size
    if w / h > want:
        nw = int(h * want); im = im.crop(((w - nw)//2, 0, (w + nw)//2, h))
    else:
        nh = int(w / want); im = im.crop((0, (h - nh)//2, w, (h + nh)//2))
    return im.resize((AW*SCALE, AH*SCALE), Image.LANCZOS)

def post(payload):
    req = urllib.request.Request(API + "/inferences",
        data=json.dumps(payload).encode(),
        headers={"X-RD-Token": KEY, "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=90) as r:
        return json.loads(r.read())

def get(path):
    req = urllib.request.Request(API + path, headers={"X-RD-Token": KEY})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.loads(r.read())

def payload_for(name, meta, estimate=False):
    m = meta[name]
    subject = m['type_line'].split('//')[0].strip()
    subj = PROMPTS.get(name, subject)
    p = {
        "prompt": f"{subj}. {subject}. {STYLE_CLAUSE}",
        "prompt_style": STYLE,
        "width": AW*SCALE, "height": AH*SCALE,
        "num_images": 1, "seed": 20261105,
        "input_palette": b64(f"{HERE}/palette.png"),
    }
    # Style references, if any, carry the house look across all 32 without
    # carrying any card's composition.
    refs = sorted(os.path.join(HERE, 'style', f) for f in
                  (os.listdir(os.path.join(HERE, 'style'))
                   if os.path.isdir(os.path.join(HERE, 'style')) else []))
    if refs:
        p["reference_images"] = [b64(r) for r in refs[:9]]
    # Opt-in only: seed from the original painting at high strength, so it
    # informs the composition without the result being a pixelation of it.
    if SEED_MODE:
        p["input_image"] = b64(crop_to_art(f"{ROOT}/refs/{slug(name)}_art.jpg"))
        p["strength"] = STRENGTH
    if estimate: p["check_cost"] = True
    return p

def run(names, estimate=False):
    meta = json.load(open(f"{ROOT}/refs/meta.json"))
    total = 0.0
    for i, n in enumerate(names):
        outp = f"{HERE}/out/{slug(n)}.png"
        if os.path.exists(outp) and not estimate:
            print(f"  = {n} (already done)"); continue
        try:
            r = post(payload_for(n, meta, estimate))
        except urllib.error.HTTPError as e:
            print(f"  ! {n}: HTTP {e.code} {e.read()[:200].decode(errors='replace')}"); continue
        if estimate:
            c = r.get('cost', r.get('balance_cost', 0)) or 0
            total += float(c); print(f"  $ {n:26} {c}"); continue
        task = r.get("task_id")
        if not task:
            print(f"  ! {n}: {json.dumps(r)[:200]}"); continue
        for _ in range(90):
            t = get(f"/inferences/tasks/{task}")
            st = t.get("status")
            if st in ("pending", "running", "queued"): time.sleep(2); continue
            if st == "failed": print(f"  ! {n}: {t.get('error')}"); break
            res = t.get("result", t)
            imgs = res.get("base64_images") or []
            if not imgs: print(f"  ! {n}: no image in result"); break
            raw = Image.open(io.BytesIO(base64.b64decode(imgs[0]))).convert('RGB')
            raw.save(f"{HERE}/out/{slug(n)}_raw.png")
            raw.resize((AW, AH), Image.NEAREST).save(outp)
            total += float(res.get("balance_cost") or 0)
            print(f"  + {n:26} {raw.size} -> {AW}x{AH}   ${res.get('balance_cost')}")
            break
        time.sleep(6.5)      # 10 req/min rate limit
    print(f"\ntotal ${total:.3f}")

if __name__ == "__main__":
    if not KEY:
        sys.exit("Set RD_KEY=rdpk-... (get one at retrodiffusion.ai -> Developer Tools)")
    if "--styles" in sys.argv:
        print(json.dumps(get("/inferences/styles"), indent=1)[:4000]); sys.exit()
    if "--balance" in sys.argv:
        print(get("/inferences/credits")); sys.exit()
    names = PILOT if "--pilot" in sys.argv else ORDER
    run(names, estimate="--estimate" in sys.argv)
