# Pixel-art card renderer

Hand-authored pixel art for Innistrad cards, plus a board compositor that
shows what a game in progress looks like.

The design split: a **card face carries identity only** — artwork, colour,
frame. Live game state (P/T, tapped, damage, summoning sickness) is drawn as
an *overlay* by the board renderer and is never baked into the card. That is
what makes "glance at the board, hover for the numbers" work.

## Layout

| file | what it is |
|---|---|
| `engine.py` | drawing DSL — canvas, primitives, the 31-colour gothic palette |
| `font.py` | 3x5 pixel font, plus the "greeked" illegible-text renderer |
| `card.py` | card frame compositor (48x56, with a 42x34 art window) |
| `art.py` | the 32 illustrations, one function per card |
| `board.py` | board mockup: rows, tapped rotation, state overlays |
| `fetch.py` | pulls reference art + metadata from Scryfall into `refs/` |
| `render.py` / `allsheet.py` / `zoom.py` / `scene.py` | render entry points |
| `out/` | rendered sheets and board mockups |

## Usage

```sh
python3 fetch.py                 # populate refs/ (needs network)
python3 allsheet.py              # out/set1.png, out/set2.png  (greeked titles)
python3 allsheet.py --named      # same, with real names in a 3x5 font
python3 scene.py out/board.png   # a turn-7 board mockup
python3 zoom.py z.png "Garruk Relentless"   # one card, 12x, for pixel work
```

## Style rules

Learned the hard way, in this order:

1. **Silhouette first** — the subject must read as a shape at 1x.
2. **No flat figures** — every figure needs 3+ tones and a material-specific
   hue. One flat colour makes a soldier, a ghost and a swordsman identical.
3. **No primitives for organic forms** — hands, heads and bodies are drawn
   pixel by pixel. Circles are for moons and orbs, lines for spears and bars.
4. **No bilateral symmetry** — a symmetric winged creature reads as a moth,
   and a symmetric figure reads as a game sprite, not gothic horror.
5. **Shoulders wider than the head**, arms 2px minimum, every arm ending in a
   hand that touches what it holds.
6. **Backlight the silhouette** — a dark subject needs a lit ground behind it.
7. **One named light source per card**, and rim-light the subject from it.
