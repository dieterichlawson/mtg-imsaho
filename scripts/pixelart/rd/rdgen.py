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
STRENGTH = float(os.environ.get("RD_STRENGTH", "0.62"))

sys.path.insert(0, ROOT)
from PIL import Image

ORDER = ["Geist of Saint Traft","Doomed Traveler","Chapel Geist","Elite Inquisitor","Midnight Haunting",
 "Snapcaster Mage","Delver of Secrets","Invisible Stalker","Laboratory Maniac","Stitched Drake",
 "Liliana of the Veil","Diregraf Ghoul","Unburial Rites","Bloodline Keeper","Grimgrin, Corpse-Born",
 "Brimstone Volley","Devil's Play","Balefire Dragon","Instigator Gang","Blasphemous Act",
 "Mayor of Avabruck","Kessig Cagebreakers","Garruk Relentless","Spider Spawning","Gatstaf Shepherd",
 "Plains","Island","Swamp","Mountain","Forest","Kessig Wolf Run","Blazing Torch"]
PILOT = ["Doomed Traveler","Liliana of the Veil","Grimgrin, Corpse-Born","Forest"]

# One shared style clause on every prompt. The subject clause is per card and
# comes from the real type line, so the model is told what it is looking at.
STYLE_CLAUSE = ("gothic horror pixel art, Innistrad, moonlit, muted blacks and "
                "deep greys, cold bone highlights, one blood-red accent, heavy "
                "shadow, woodcut feel, limited palette, no text")

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
    p = {
        "prompt": f"{name}, {subject}. {STYLE_CLAUSE}",
        "prompt_style": STYLE,
        "width": AW*SCALE, "height": AH*SCALE,
        "num_images": 1, "seed": 7,
        "input_image": b64(crop_to_art(f"{ROOT}/refs/{slug(name)}_art.jpg")),
        "strength": STRENGTH,
        "input_palette": b64(f"{HERE}/palette.png"),
    }
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
