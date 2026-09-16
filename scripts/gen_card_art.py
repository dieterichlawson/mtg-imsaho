#!/usr/bin/env python3
"""Generate pixel-art placeholders for the GUI with Retro Diffusion.

One image per card face the engine knows, one per token kind, and a set
of UI pieces. Each card's prompt is built from its own data (type line,
subtypes, colours, Scryfall flavour text) and the real card art is fed in
as the image-to-image source, so the pixel art is a small cousin of the
printed illustration rather than a random creature.

    RETRO_DIFFUSION_API_KEY=rdpk-... scripts/gen_card_art.py cards --dry-run
    RETRO_DIFFUSION_API_KEY=rdpk-... scripts/gen_card_art.py cards --only "Abattoir Ghoul"
    RETRO_DIFFUSION_API_KEY=rdpk-... scripts/gen_card_art.py cards tokens ui

Outputs go to `mtg-gui/assets/art/{cards,tokens,ui}/<slug>.png`, and
`mtg-gui/assets/art/manifest.json` records the prompt, style, size, seed
and cost of every image so any one can be regenerated alone. Reference
art from Scryfall is cached under `logs/card-art-refs/` (gitignored: it
is Wizards' art, and only ever an input).

The key is read from the environment and never written anywhere.
"""

from __future__ import annotations

import argparse
import base64
import io
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARDS_SRC = ROOT / "mtg-engine" / "src" / "cards"
ORACLE_CACHE = ROOT / "data" / "oracle_cache.json"
OUT_ROOT = ROOT / "mtg-gui" / "assets" / "art"
MANIFEST = OUT_ROOT / "manifest.json"
REF_CACHE = ROOT / "logs" / "card-art-refs"

RD_BASE = "https://api.retrodiffusion.ai/v2"
SCRYFALL = "https://api.scryfall.com"

# The art window is 4:3, which is also the shape of a Magic art box.
ART_W, ART_H = 64, 48
DEFAULT_STYLE = "rd_plus__low_res"
# Chosen on a probe sheet (Abattoir Ghoul, Moorland Haunt, Forest, Lightning
# Bolt): the raw crop at 0.45-0.65 reproduces a dark smear, 0.85+ loses the
# composition, and prompt-only is clean but unrelated to the printed art. A
# contrast-lifted crop at 0.78 keeps the scene and reads as pixel art.
DEFAULT_STRENGTH = 0.78
ENHANCE_REF = True

COLOR_WORDS = {
    "W": "white and gold, holy light",
    "U": "blue and silver, cold moonlight",
    "B": "black and violet, grave shadow",
    "R": "red and orange, fire and blood",
    "G": "green and brown, wild forest",
}

SET_FLAVOR = "gothic horror, Innistrad, moonlit, muted palette, pixel art illustration"


def slug(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


# ---------------------------------------------------------------- card list


def registry_faces() -> list[str]:
    """Every card face name the engine registers, front and back.

    Read from the source rather than a binary: every card (and every
    transform back face) states its name as `name: "...".into()` in its
    `CardData`, and that is the name the view carries.
    """
    names: set[str] = set()
    pat = re.compile(r'^\s+name:\s+"([^"]+)"\.(?:into|to_string)\(\)', re.M)
    for path in CARDS_SRC.rglob("*.rs"):
        names.update(pat.findall(path.read_text()))
    return sorted(names)


# Tokens the set's cards create, as (name, colour letter, description).
# CR 111.4: a token's name is its subtype when the effect names none.
TOKENS = [
    ("Spirit", "W", "a small translucent ghost with tattered white robes, glowing softly"),
    ("Zombie", "B", "a shambling rotting corpse with grasping hands"),
    ("Wolf", "G", "a lean grey wolf with bared teeth, hunting"),
    ("Vampire", "B", "a pale aristocratic vampire in a red-lined black cloak"),
    ("Spider", "G", "a huge hairy black spider on a web"),
    ("Homunculus", "U", "a tiny stitched blue homunculus with mismatched eyes"),
    ("Ooze", "G", "a green dripping slime with skulls inside it"),
    ("Angel", "W", "a fierce armoured angel with spread white wings"),
    ("Demon", "B", "a horned black demon with burning eyes"),
]

# UI pieces: (file name, style, width, height, prompt).
UI_PIECES = [
    ("table", "rd_plus__mc_texture", 64, 64,
     "seamless dark wooden tavern table texture with candle wax stains, gothic"),
    ("frame-white", "rd_plus__ui_element", 64, 96,
     "ornate ivory and gold trading card frame border, empty center, gothic"),
    ("frame-blue", "rd_plus__ui_element", 64, 96,
     "ornate deep blue and silver trading card frame border, empty center, gothic"),
    ("frame-black", "rd_plus__ui_element", 64, 96,
     "ornate black and bone trading card frame border, empty center, gothic"),
    ("frame-red", "rd_plus__ui_element", 64, 96,
     "ornate dark red and iron trading card frame border, empty center, gothic"),
    ("frame-green", "rd_plus__ui_element", 64, 96,
     "ornate mossy green and bark trading card frame border, empty center, gothic"),
    ("frame-gold", "rd_plus__ui_element", 64, 96,
     "ornate golden multicolour trading card frame border, empty center, gothic"),
    ("frame-colorless", "rd_plus__ui_element", 64, 96,
     "ornate grey stone trading card frame border, empty center, gothic"),
    ("card-back", "rd_plus__ui_element", 64, 96,
     "back of a trading card, dark leather with a silver crescent moon emblem, symmetrical"),
    ("panel", "rd_plus__ui_element", 96, 64,
     "dark parchment and iron ui panel with riveted border, empty center"),
    ("button", "rd_plus__ui_element", 96, 64,
     "wide iron ui button with a rounded bevel, empty, gothic"),
    ("button-pressed", "rd_plus__ui_element", 96, 64,
     "wide iron ui button pressed down, darker, empty, gothic"),
    ("life-heart", "rd_plus__skill_icon", 32, 32,
     "a red anatomical heart icon on a dark background"),
    ("library-icon", "rd_plus__skill_icon", 32, 32,
     "a stack of old books icon on a dark background"),
    ("graveyard-icon", "rd_plus__skill_icon", 32, 32,
     "a cracked tombstone icon on a dark background"),
    ("exile-icon", "rd_plus__skill_icon", 32, 32,
     "a swirling purple void portal icon on a dark background"),
    ("mana-w", "rd_plus__skill_icon", 32, 32, "a white sun mana symbol icon, round, on a dark background"),
    ("mana-u", "rd_plus__skill_icon", 32, 32, "a blue water drop mana symbol icon, round, on a dark background"),
    ("mana-b", "rd_plus__skill_icon", 32, 32, "a black skull mana symbol icon, round, on a dark background"),
    ("mana-r", "rd_plus__skill_icon", 32, 32, "a red fireball mana symbol icon, round, on a dark background"),
    ("mana-g", "rd_plus__skill_icon", 32, 32, "a green tree mana symbol icon, round, on a dark background"),
    ("mana-c", "rd_plus__skill_icon", 32, 32, "a grey diamond colorless mana symbol icon, round, on a dark background"),
    ("tap-icon", "rd_plus__skill_icon", 32, 32, "a curved tap arrow icon, white on a dark background"),
    ("moon", "rd_plus__skill_icon", 32, 32, "a full moon behind wispy clouds icon"),
    ("sun", "rd_plus__skill_icon", 32, 32, "a pale sun behind clouds icon"),
]


# ------------------------------------------------------------- scryfall


def http_json(url: str, headers: dict | None = None, body: dict | None = None,
              method: str | None = None) -> dict:
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method or ("POST" if data else "GET"))
    req.add_header("User-Agent", "mtg-imsaho-art/1.0")
    req.add_header("Accept", "application/json")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    for k, v in (headers or {}).items():
        req.add_header(k, v)
    for attempt in range(5):
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                return json.loads(resp.read())
        except urllib.error.HTTPError as e:
            text = e.read().decode(errors="replace")
            if e.code in (429, 500, 502, 503, 504) and attempt < 4:
                time.sleep(2 ** attempt)
                continue
            raise RuntimeError(f"{method or 'GET'} {url}: HTTP {e.code}: {text[:300]}") from None
        except (urllib.error.URLError, TimeoutError) as e:
            if attempt < 4:
                time.sleep(2 ** attempt)
                continue
            raise RuntimeError(f"{url}: {e}") from None
    raise AssertionError("unreachable")


def scryfall_face(name: str) -> dict:
    """The Scryfall face record for `name`, cached: name, flavor, art url.

    Prefers the Innistrad printing; a back face is found through its
    front card's `card_faces`.
    """
    REF_CACHE.mkdir(parents=True, exist_ok=True)
    cache = REF_CACHE / f"{slug(name)}.json"
    if cache.exists():
        return json.loads(cache.read_text())
    q = urllib.parse.quote(name)
    card = None
    urls = [f"{SCRYFALL}/cards/named?exact={q}&set=isd"]
    # The printing the oracle cache was built from, so a core card gets
    # its classic art rather than whatever crossover printed last.
    cached = json.loads(ORACLE_CACHE.read_text())["cards"].get(name, {})
    m = re.search(r"scryfall\.com/card/([a-z0-9]+)/", cached.get("source_url", ""))
    if m and m.group(1) != "isd":
        urls.append(f"{SCRYFALL}/cards/named?exact={q}&set={m.group(1)}")
    urls.append(f"{SCRYFALL}/cards/named?exact={q}")
    for url in urls:
        try:
            card = http_json(url)
            break
        except RuntimeError as e:
            if "HTTP 404" not in str(e):
                raise
    if card is None:
        raise RuntimeError(f"Scryfall does not know {name!r}")
    face = card
    for f in card.get("card_faces", []):
        if f.get("name") == name:
            face = f
            break
    art = (face.get("image_uris") or card.get("image_uris") or {}).get("art_crop")
    rec = {
        "name": name,
        "card_name": card.get("name"),
        "set": card.get("set"),
        "type_line": face.get("type_line") or card.get("type_line", ""),
        "colors": face.get("colors") if face.get("colors") is not None else card.get("colors", []),
        "flavor_text": face.get("flavor_text") or "",
        "oracle_text": face.get("oracle_text") or card.get("oracle_text", ""),
        "art_crop": art,
        "artist": face.get("artist") or card.get("artist", ""),
        "produced_mana": card.get("produced_mana", []),
    }
    cache.write_text(json.dumps(rec, indent=1))
    time.sleep(0.1)  # Scryfall asks for ~10 requests/second at most.
    return rec


def reference_png_b64(rec: dict, w: int, h: int, enhance: bool = False) -> str | None:
    """The card's art crop, downloaded once, resized to the art window.

    `enhance` stretches the contrast and lifts the brightness first:
    Innistrad art is dark and painterly, and at 64x48 an unaltered crop
    is a brown smear the model faithfully reproduces.
    """
    if not rec.get("art_crop"):
        return None
    from PIL import Image, ImageEnhance, ImageOps
    # Keyed by printing, so a re-fetch that picks a different printing
    # does not reuse the old crop.
    path = REF_CACHE / f"{slug(rec['name'])}-{rec.get('set', 'x')}.jpg"
    if not path.exists():
        req = urllib.request.Request(rec["art_crop"], headers={"User-Agent": "mtg-imsaho-art/1.0"})
        with urllib.request.urlopen(req, timeout=60) as resp:
            path.write_bytes(resp.read())
        time.sleep(0.1)
    im = Image.open(path).convert("RGB")
    # Cover-fit: scale so the whole window is filled, then centre-crop.
    scale = max(w / im.width, h / im.height)
    im = im.resize((max(w, round(im.width * scale)), max(h, round(im.height * scale))), Image.LANCZOS)
    left, top = (im.width - w) // 2, (im.height - h) // 2
    im = im.crop((left, top, left + w, top + h))
    if enhance:
        im = ImageOps.autocontrast(im, cutoff=2)
        im = ImageEnhance.Brightness(im).enhance(1.25)
        im = ImageEnhance.Color(im).enhance(1.3)
    buf = io.BytesIO()
    im.save(buf, "PNG")
    return base64.b64encode(buf.getvalue()).decode()


# --------------------------------------------------------------- prompts


def clean_flavor(text: str) -> str:
    """Flavour text without its attribution line, on one line, capped."""
    lines = [l.strip() for l in text.splitlines() if l.strip()]
    lines = [l for l in lines if not l.startswith("—")]
    out = " ".join(lines).replace('"', "")
    return out[:220]


def card_prompt(name: str, rec: dict) -> str:
    type_line = rec.get("type_line", "")
    main, _, sub = type_line.partition("—")
    types = main.strip().lower()
    subtypes = sub.strip()
    colors = rec.get("colors") or []
    if "land" in types.lower():
        colors = rec.get("produced_mana") or colors
    palette = ", ".join(COLOR_WORDS[c] for c in colors if c in COLOR_WORDS) or "grey stone and bone, colourless"
    flavor = clean_flavor(rec.get("flavor_text", "")).rstrip(".")

    if "creature" in types:
        subject = f"{name}, a {subtypes.lower() or 'creature'}, full figure in a dramatic pose"
    elif "land" in types:
        subject = f"a landscape: {name}, {subtypes.lower() or 'wild land'}, wide view, no characters"
    elif "planeswalker" in types:
        subject = f"{name}, a powerful {subtypes.lower()} planeswalker, portrait"
    elif "aura" in subtypes.lower():
        subject = f"{name}, a magical enchantment taking hold of a figure, glowing runes"
    elif "equipment" in subtypes.lower():
        subject = f"{name}, a single gleaming weapon or piece of armour, close up"
    elif "artifact" in types:
        subject = f"{name}, a strange arcane artifact object, close up"
    elif "enchantment" in types:
        subject = f"{name}, a lingering magical presence over a scene"
    else:  # instant, sorcery
        subject = f"a dramatic moment of magic: {name}, the spell in action"

    parts = [subject]
    if flavor:
        parts.append(flavor)
    parts.append(palette)
    parts.append(SET_FLAVOR)
    return ". ".join(parts)


def token_prompt(name: str, color: str, desc: str) -> str:
    return f"{name} creature token: {desc}. {COLOR_WORDS[color]}. {SET_FLAVOR}"


# ----------------------------------------------------------- retro diffusion


class RetroDiffusion:
    def __init__(self, key: str):
        self.headers = {"X-RD-Token": key}

    def balance(self) -> dict:
        return http_json(f"{RD_BASE}/inferences/credits", self.headers)

    def generate(self, body: dict) -> dict:
        """Submit and wait. Returns the completed task record."""
        accepted = http_json(f"{RD_BASE}/inferences", self.headers, body)
        if body.get("check_cost"):
            return accepted
        task_id = accepted.get("task_id")
        if not task_id:
            # Some replies are synchronous.
            if accepted.get("base64_images"):
                return accepted
            raise RuntimeError(f"no task id in {json.dumps(accepted)[:300]}")
        deadline = time.time() + 600
        delay = 2.0
        while time.time() < deadline:
            time.sleep(delay)
            task = http_json(f"{RD_BASE}/inferences/tasks/{task_id}", self.headers)
            status = str(task.get("status", "")).lower()
            # A finished task is `{"status": "succeeded", "result": {...}}`
            # with the images and the cost inside `result`.
            result = task.get("result") or {}
            if result.get("base64_images"):
                return result
            if task.get("base64_images"):
                return task
            if status in ("succeeded", "failed", "error", "cancelled"):
                raise RuntimeError(f"task {task_id} {status} without images: {json.dumps(task)[:300]}")
            delay = min(delay * 1.5, 10)
        raise RuntimeError(f"task {task_id} timed out")


# ----------------------------------------------------------------- driver


class Job:
    def __init__(self, kind: str, name: str, prompt: str, style: str, w: int, h: int,
                 ref_b64: str | None, strength: float, seed: int):
        self.kind, self.name, self.prompt, self.style = kind, name, prompt, style
        self.w, self.h, self.ref_b64, self.strength, self.seed = w, h, ref_b64, strength, seed

    @property
    def out(self) -> Path:
        return OUT_ROOT / self.kind / f"{slug(self.name)}.png"

    def body(self, check_cost: bool = False) -> dict:
        b = {
            "prompt": self.prompt,
            "prompt_style": self.style,
            "width": self.w,
            "height": self.h,
            "num_images": 1,
            "seed": self.seed,
        }
        if self.ref_b64:
            b["input_image"] = self.ref_b64
            b["strength"] = self.strength
        if check_cost:
            b["check_cost"] = True
        return b


def load_manifest() -> dict:
    if MANIFEST.exists():
        return json.loads(MANIFEST.read_text())
    return {"art_size": [ART_W, ART_H], "images": {}}


def save_manifest(m: dict) -> None:
    OUT_ROOT.mkdir(parents=True, exist_ok=True)
    m["images"] = dict(sorted(m["images"].items()))
    MANIFEST.write_text(json.dumps(m, indent=1, ensure_ascii=False) + "\n")


def seed_for(name: str) -> int:
    # Stable per name so a re-run reproduces, and different per card.
    return sum((i + 1) * ord(c) for i, c in enumerate(name)) % 1_000_000


def build_jobs(args) -> list[Job]:
    jobs: list[Job] = []
    only = {n.lower() for n in args.only} if args.only else None
    if "cards" in args.what:
        for name in registry_faces():
            if only and name.lower() not in only:
                continue
            rec = scryfall_face(name)
            ref = None if args.mode == "prompt" else reference_png_b64(
                rec, args.width, args.height, enhance=args.enhance_ref)
            jobs.append(Job("cards", name, card_prompt(name, rec), args.style,
                            args.width, args.height, ref, args.strength, seed_for(name)))
    if "tokens" in args.what:
        for name, color, desc in TOKENS:
            if only and name.lower() not in only:
                continue
            jobs.append(Job("tokens", name, token_prompt(name, color, desc), args.style,
                            args.width, args.height, None, args.strength, seed_for("token " + name)))
    if "ui" in args.what:
        for name, style, w, h, prompt in UI_PIECES:
            if only and name.lower() not in only:
                continue
            jobs.append(Job("ui", name, prompt, style, w, h, None, args.strength, seed_for("ui " + name)))
    return jobs


def run_job(rd: RetroDiffusion, job: Job, manifest: dict, lock) -> tuple[str, float]:
    task = rd.generate(job.body())
    images = task.get("base64_images") or []
    if not images:
        raise RuntimeError(f"{job.name}: no image in {json.dumps(task)[:200]}")
    job.out.parent.mkdir(parents=True, exist_ok=True)
    job.out.write_bytes(base64.b64decode(images[0]))
    cost = float(task.get("balance_cost") or 0)
    with lock:
        manifest["images"][f"{job.kind}/{slug(job.name)}"] = {
            "name": job.name,
            "kind": job.kind,
            "file": str(job.out.relative_to(OUT_ROOT)),
            "prompt": job.prompt,
            "style": job.style,
            "size": [job.w, job.h],
            "seed": job.seed,
            "img2img_strength": job.strength if job.ref_b64 else None,
            "enhanced_ref": bool(job.ref_b64) and ENHANCE_REF,
            "cost_usd": cost,
        }
        save_manifest(manifest)
    return job.name, cost


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("what", nargs="*", default=["cards"], choices=["cards", "tokens", "ui"])
    ap.add_argument("--only", nargs="*", help="only these names")
    ap.add_argument("--style", default=DEFAULT_STYLE)
    ap.add_argument("--width", type=int, default=ART_W)
    ap.add_argument("--height", type=int, default=ART_H)
    ap.add_argument("--mode", choices=["img2img", "prompt"], default="img2img",
                    help="img2img feeds the printed art in as the source image")
    ap.add_argument("--strength", type=float, default=DEFAULT_STRENGTH,
                    help="img2img strength: 0 keeps the source, 1 ignores it")
    ap.add_argument("--no-enhance-ref", dest="enhance_ref", action="store_false",
                    help="feed the reference in as is, without the contrast and brightness lift")
    ap.add_argument("--force", action="store_true", help="regenerate images that exist")
    ap.add_argument("--dry-run", action="store_true", help="list jobs and the estimated cost; spend nothing")
    ap.add_argument("--jobs", type=int, default=3, help="parallel requests")
    ap.add_argument("--print-prompts", action="store_true")
    ap.add_argument("--out", help="write images and manifest here instead of mtg-gui/assets/art")
    args = ap.parse_args()
    global ENHANCE_REF
    ENHANCE_REF = args.enhance_ref
    if args.out:
        global OUT_ROOT, MANIFEST
        OUT_ROOT = Path(args.out).resolve()
        MANIFEST = OUT_ROOT / "manifest.json"

    key = os.environ.get("RETRO_DIFFUSION_API_KEY", "")
    if not key.startswith("rdpk-"):
        print("RETRO_DIFFUSION_API_KEY is not set (keys start with rdpk-)", file=sys.stderr)
        return 2
    rd = RetroDiffusion(key)

    jobs = build_jobs(args)
    if not args.force:
        jobs = [j for j in jobs if not j.out.exists()]
    if args.print_prompts:
        for j in jobs:
            print(f"[{j.kind}] {j.name}: {j.prompt}")
    if not jobs:
        print("nothing to do")
        return 0

    bal = rd.balance()
    print(f"balance: ${bal.get('balance')}  credits: {bal.get('credits')}")
    if args.dry_run:
        est = rd.generate(jobs[0].body(check_cost=True))
        per = est.get("balance_cost") or est.get("cost") or est
        print(f"{len(jobs)} images; first job estimate: {per}")
        for j in jobs:
            print(f"  [{j.kind}] {j.name} ({j.style} {j.w}x{j.h})")
        return 0

    import threading
    lock = threading.Lock()
    manifest = load_manifest()
    manifest["art_size"] = [args.width, args.height]
    total = 0.0
    failures = []
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(run_job, rd, j, manifest, lock): j for j in jobs}
        for i, fut in enumerate(futures, 1):
            job = futures[fut]
            try:
                name, cost = fut.result()
                total += cost
                print(f"[{i}/{len(jobs)}] {name}  ${cost:.3f}")
            except Exception as e:  # noqa: BLE001 — report and keep going
                failures.append((job.name, str(e)))
                print(f"[{i}/{len(jobs)}] {job.name}  FAILED: {e}", file=sys.stderr)
    print(f"spent ${total:.2f} on {len(jobs) - len(failures)} images; {len(failures)} failed")
    for name, err in failures:
        print(f"  {name}: {err}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
