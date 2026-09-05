# Retro Diffusion pipeline

Generates the 32 card illustrations img2img from the cached Scryfall art.

## Why this and not a text prompt

Three API parameters map onto the three things hand-drawing kept failing:

| parameter | fixes |
|---|---|
| `input_image` + `strength` | composition and card identity come from the real painting, not from recollection |
| `reference_images` (up to 9, RD Pro) | one house style across all 32 |
| `input_palette` | output constrained to the 31-colour ramp the frames already use |

## Run

    export RD_KEY=rdpk-...            # retrodiffusion.ai -> Developer Tools
    python3 rd/rdgen.py --balance     # check credit
    python3 rd/rdgen.py --styles      # list the 90+ styles
    python3 rd/rdgen.py --estimate    # free dry run, prints per-card cost
    python3 rd/rdgen.py --pilot       # 4 cards, one from each problem class
    python3 rd/rdgen.py               # all 32 (resumes; skips existing)

Tunable by env: `RD_STYLE` (default `rd_plus__default`), `RD_STRENGTH`
(default 0.62 — lower keeps more of the original composition), `RD_SCALE`
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

img2img from Wizards' paintings produces derivative works. Fine for a
personal prototype; do not distribute.
