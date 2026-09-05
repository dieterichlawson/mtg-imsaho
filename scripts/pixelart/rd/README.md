# Retro Diffusion pipeline

Generates the 32 card illustrations img2img from the cached Scryfall art.

## Original art, not pixelated paintings

The default is TEXT-TO-IMAGE. Every card has a hand-written subject
description in `PROMPTS`, taken from actually looking at that card's source
painting earlier in this project — so the model is told what the card IS and
composes it freshly. Chapel Geist is "an empty billowing pale robe with no
body inside it, a hanging chain and pendant"; Invisible Stalker is "an empty
coat and hat hanging in the air in the rain, nobody inside them". Identity
comes from the description; the composition is the generator's own.

Cohesion comes from two things instead: a shared style clause appended to all
32 prompts, and any images dropped in `rd/style/`, which are passed as
`reference_images` (up to 9) so every card inherits one house look without
inheriting any card's composition.

`input_palette` constrains output to the 30-colour ramp the frames already
use, so results drop into the existing compositor untouched.

There is an opt-in middle setting, `RD_SEED_IMAGE=1`, which additionally
seeds from the original painting at strength 0.9 — loose enough to inform
the composition without the result being a filter over the painting. Lower
strengths approach pixelation and are deliberately not the default.

## Run

    export RD_KEY=rdpk-...            # retrodiffusion.ai -> Developer Tools
    python3 rd/rdgen.py --balance     # check credit
    python3 rd/rdgen.py --styles      # list the 90+ styles
    python3 rd/rdgen.py --estimate    # free dry run, prints per-card cost
    python3 rd/rdgen.py --pilot       # 4 cards, one from each problem class
    python3 rd/rdgen.py               # all 32 (resumes; skips existing)

Tunable by env: `RD_STYLE` (default `rd_plus__default`), `RD_SEED_IMAGE=1`
to seed from the painting, `RD_STRENGTH` (default 0.9; lower means closer to
the original, which is the thing to avoid), `RD_SCALE`
(default 3, so generation happens at 126x102 and downsamples to the 42x34
art window on clean integer boundaries).

Output lands in `rd/out/<slug>.png` at 42x34, plus `<slug>_raw.png` at full
generation size. `render.py` picks these up automatically and falls back to
the hand-drawn `art.py` illustration wherever a file is missing, so the two
can be compared card by card on the same sheet.

## Cost

~$0.015/image (RD Fast) to $0.18 (RD Pro). All 32 at RD Plus with a few
iterations lands around $5-15. Prepaid, no subscription. `--estimate` is free.

## Rights

Text-to-image output is original work. If `RD_SEED_IMAGE=1` is used, the
result is derived from Wizards' painting — fine for a personal prototype,
but do not distribute those.
