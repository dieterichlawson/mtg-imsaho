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
 "Geist of Saint Traft": "a seated translucent hooded spirit on a stone throne, surrounded by dozens of floating candle flames in a dark crypt",
 "Doomed Traveler": "a lone soldier with a tall spear walking a cobbled road at night, a village with lit windows on the ridge above, full moon",
 "Chapel Geist": "an empty billowing pale robe with no body inside it, a hanging chain and pendant, dark chapel windows behind",
 "Elite Inquisitor": "an armoured figure in a pale storm-coat raising a thin rapier, tall gothic windows, a wolf trophy on the wall",
 "Midnight Haunting": "a stream of white spirits pouring out through a broken window with firelight burning behind it",
 "Snapcaster Mage": "a dark-haired man in black and silver armour with a glowing teal lantern bound to his forearm, gothic castle behind",
 "Delver of Secrets": "a bald robed scholar holding up a single glowing moth, specimen jars and birdcages around him in a dim study",
 "Invisible Stalker": "an empty coat and hat hanging in the air in the rain beside a wet stone wall, nobody inside them",
 "Laboratory Maniac": "a wild-eyed scientist at a machine with both arms flung up, blue lightning arcing overhead, green reagent vials",
 "Stitched Drake": "a skeletal undead drake with tattered membrane wings flying over grey ruins, pale moon behind",
 "Liliana of the Veil": "a pale woman with long dark hair in a deep magenta corset gown, an orange flame burning in each open palm",
 "Diregraf Ghoul": "a hunched pale zombie in torn wrappings hauling a long rusted scythe through dark woods",
 "Unburial Rites": "a robed figure leaning over an opening stone sarcophagus with a carved face on its lid, cold crypt windows",
 "Bloodline Keeper": "a vampire in a long flaring black cloak with arms spread, bats around him, an enormous sickly-green moon, a cathedral spire",
 "Grimgrin, Corpse-Born": "a huge hulking stitched zombie with iron shoulder plates and a chain hooked to one arm, dead trees behind",
 "Brimstone Volley": "flaming meteors streaking down on a hard diagonal onto a burning ridge, dead trees in silhouette against the fire",
 "Devil\'s Play": "a small horned devil crouched on a fallen beam pouring a stream of fire down onto burning wreckage",
 "Balefire Dragon": "a black dragon with spread wings against a blood-red sky, breathing fire, iron grave-crosses below",
 "Instigator Gang": "three angry brawlers with fists raised, khaki and sepia, a mob closing in",
 "Blasphemous Act": "a cathedral interior with bodies strewn down the steps, red banners, a lone armoured figure standing in firelight",
 "Mayor of Avabruck": "a man in a dark coat leaning on a desk in a lamplit study, a black hound at his knee, cold window behind",
 "Kessig Cagebreakers": "an iron cage with a snarling wolf pressed against the bars, men in tricorn hats outside it, a torch burning",
 "Garruk Relentless": "a bearded hunter in a heavy fur mantle holding a great axe across his body, pale spectral forest",
 "Spider Spawning": "a tide of long-legged spiders descending a dark forest slope, silhouetted against pale fog",
 "Gatstaf Shepherd": "a shepherd with a tall crook standing among a flock of sheep on a hillside at dusk",
 "Plains": "a wide low marshy plain, a slit of gold sky through grey cloud, bare trees, a pale stream",
 "Island": "a tall white waterfall plunging into a dark gorge between sheer cliffs",
 "Swamp": "a dead gnarled tree and a broken wooden fence in flat green mist",
 "Mountain": "a dark rocky gorge with a small low sun burning in a warm slit of sky",
 "Forest": "a violet misty forest of tall pale trunks, fog between them",
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
        "num_images": 1, "seed": 7,
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
