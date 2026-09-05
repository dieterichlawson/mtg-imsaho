"""Generate N alternative compositions for one card and sheet them together."""
import base64, io, json, os, sys, time, urllib.request, urllib.error
HERE = os.path.dirname(os.path.abspath(__file__)); ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT); sys.path.insert(0, HERE)
sys.dont_write_bytecode = True
from PIL import Image, ImageDraw, ImageFont
API = "https://api.retrodiffusion.ai/v2"; KEY = os.environ["RD_KEY"]
AW, AH, S = 42, 34, 3

def b64(p):
    im = Image.open(p).convert('RGB'); buf = io.BytesIO(); im.save(buf, 'PNG')
    return base64.b64encode(buf.getvalue()).decode()

def gen(prompt, seed, out):
    if os.path.exists(out): return
    body = {"prompt": prompt, "prompt_style": "rd_plus__default",
            "width": AW*S, "height": AH*S, "num_images": 1, "seed": seed,
            "input_palette": b64(f"{HERE}/palette.png")}
    r = urllib.request.Request(API+"/inferences", data=json.dumps(body).encode(),
        headers={"X-RD-Token": KEY, "Content-Type": "application/json"})
    task = json.loads(urllib.request.urlopen(r, timeout=90).read())["task_id"]
    for _ in range(90):
        q = urllib.request.Request(f"{API}/inferences/tasks/{task}", headers={"X-RD-Token": KEY})
        t = json.loads(urllib.request.urlopen(q, timeout=60).read())
        if t.get("status") in ("pending","running","queued"): time.sleep(2); continue
        res = t.get("result", t)
        Image.open(io.BytesIO(base64.b64decode(res["base64_images"][0]))).save(out)
        print("  +", os.path.basename(out), res.get("balance_cost")); return
    print("  ! timeout", out)

def sheet(name, labels, paths, path):
    import hires, card, render
    ims = []
    for p in paths:
        base = render.build(name, True, 1).convert('RGB')
        big = base.resize((base.width*S, base.height*S), Image.NEAREST)
        a = Image.open(p).convert('RGB')
        if a.size != (AW*S, AH*S): a = a.resize((AW*S, AH*S), Image.NEAREST)
        big.paste(a, (card.ART_X*S, card.ART_Y*S)); ims.append(big)
    w, h = ims[0].size; sc = 2; w*=sc; h*=sc
    f = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 15)
    fs = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 12)
    out = Image.new('RGB', (len(ims)*(w+18)+18, h+72), (13,11,20)); d = ImageDraw.Draw(out)
    for i, im in enumerate(ims):
        x = 18+i*(w+18)
        out.paste(im.resize((w,h), Image.NEAREST), (x, 14))
        d.text((x, h+22), "OPTION %d" % (i+1), font=f, fill=(0xd9,0xc4,0x6a))
        for j, ln in enumerate(labels[i]):
            d.text((x, h+40+j*15), ln, font=fs, fill=(0x9a,0x8f,0xa8))
    out.save(path); print(path, out.size)
