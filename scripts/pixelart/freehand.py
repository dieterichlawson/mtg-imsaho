"""Sprites drawn freehand: every pixel placed by hand, no drawing primitives.

No limb(), no torso(), no profiles, no hline, no disc. Each figure is a
literal character grid. Anything a helper generates carries the helper's
geometry — that is where the quadrilateral bodies and orb heads came from.
The only code here is a validator and a blitter.

  .  transparent      0 keyline / black
  n  skin  o skin-lit  m skin-shadow
  h  cloth mid  i cloth lit  r cloth dark
  k  leather dark  l leather mid  9 leather lit
  s  steel  t steel-lit  3 steel-dark
  6  bone  7 bone-lit  5 grey  4 grey-dark  2 dusk  1 night
  b  blood  c flame  d fire-lit  8 gold  q arcane-lit  p arcane
"""

SPRITES = {}

def sprite(name):
    def deco(rows):
        w = max(len(r) for r in rows)
        SPRITES[name] = [r.ljust(w, '.') for r in rows]
        bad = [i for i, r in enumerate(rows) if len(r) != w]
        if bad: print("  ! %-10s ragged rows %s (padded to %d)" % (name, bad, w))
        return rows
    return deco

def blit(cv, x0, y0, name):
    for dy, row in enumerate(SPRITES[name]):
        for dx, ch in enumerate(row):
            if ch != '.':
                cv.set(x0 + dx, y0 + dy, ch)

# ── SOLDIER ──────────────────────────────────────────────────────────
# Turned to his left, weight on the near leg, spear gripped in the far hand.
# The near arm hangs clear of the body so the outline breaks in two places.
sprite("soldier")([
 ".....00000.......00.",
 "....0sttts0......0t0",
 "...0stttttt0.....0t0",
 "...0tonn0nntm....0t0",
 "...0onnmnnnm.....0l0",
 "....0onnnm0......0l0",
 ".....0nnm0.......0l0",
 ".....0nn0........0l0",
 "...000hh000......0l0",
 "..0stihhhits0....0l0",
 ".0nstihhhhits0...0l0",
 ".0on0ihhhhhi0n0..0l0",
 ".0nn0ihhhhhi0nn0.0l0",
 ".0on0ihhhhhi0on0.0l0",
 ".0nn0ihhhhhi0nn00ol0",
 ".0on0ikkkkki0onoool0",
 ".0nn0ihhhhhi0on0.0l0",
 ".0on0ihhhhhi00n0.0l0",
 "..0o0ihhhhhi0.00.0l0",
 "..0n0ihhhhhi0....0l0",
 "..000ihhhhhi0....0l0",
 "....0ihhhhhi0....0l0",
 "....00kkkkk00....0l0",
 "....0kl00lk0.....0l0",
 "....0kl0.0kl0....0l0",
 "...0kl0..0kl0....0l0",
 "...0kl0..0kl0....0l0",
 "...0kl0...0kl0...0l0",
 "..00kk0...0kl0...0l0",
 "..0kkk0...00kk0..0l0",
 "..00000....0kkk0.0l0",
 "............00000000",
])

# ── ZOMBIE ───────────────────────────────────────────────────────────
# Head lolls to his left, spine curled, right shoulder dropped, left arm
# hanging much longer than the right. Ribs read as bone through torn flesh.
# Nothing about this figure mirrors.
sprite("zombie")([
 "...00000..........",
 "..06666m0.........",
 "..060m0m0.........",
 "..06m0m60.........",
 "..0m6mm60.........",
 "...0mmm0..........",
 "....0m60..........",
 "...00m600.........",
 "..0m6mmm60........",
 ".0m66mmmm60.......",
 ".0m6mmmmmm60......",
 "0m06mmmmmmm0......",
 "0m06mmm6mmm60.....",
 "0m0m6mm66mmm0.....",
 "0m0m6mmm6mmm0.....",
 "0m0.0mm66mmm0.....",
 "0m0.0mmm6mmm00....",
 "066.0mmmm6mm0m0...",
 "066.0mmmmmmm0m0...",
 ".066.0mmmmmm0m0...",
 ".066.0mmmmm00m0...",
 "..066.0mmmm0.m0...",
 "..0660.0mmm0.m0...",
 "...066.0mm0.0m0...",
 "...000.0mm0.066...",
 "......00m00.0660..",
 "......0m0m0..066..",
 ".....0m0.0m0..00..",
 ".....0m0.0m0......",
 "....00m0..0m00....",
 "....0mm0..0mm0....",
 "....0000..0000....",
])

# ── VAMPIRE ──────────────────────────────────────────────────────────
# Cape thrown out to his right only; the near shoulder stays clear of it so
# the silhouette isn't a symmetrical bat. High collar behind the jaw.
sprite("vampire")([
 "......00000.......",
 ".....0000000......",
 "....00nnnnn00.....",
 "....00b0n0b00.....",
 "....0onnnnnm0.....",
 ".....0onnnm0......",
 "......0nnm0.......",
 "...00.0nn0.00.....",
 "..0330233302200...",
 "..03302333022200..",
 ".0330n2333022220..",
 ".0330o2333002222..",
 ".033on23330022220.",
 ".0330o2333000222 0",
 ".0330.2333000222 0",
 ".033..2333002222..",
 ".033..2333002220..",
 "..03..2333002200..",
 "..0...2333002000..",
 "......23330020....",
 "......2333000.....",
 "......233300......",
 "......23330.......",
 ".....023330.......",
 ".....02333........",
 "....00k00k00......",
 "....0k0..0k0......",
 "...00k0..0k00.....",
 "...0kk0..0kk0.....",
 "...0000..0000.....",
])

# ── SCHOLAR ──────────────────────────────────────────────────────────
# Bald, stooped over a codex he holds in both hands. Robe falls unevenly.
sprite("scholar")([
 "......00000.......",
 ".....0nnnnn0......",
 ".....0onnnnm0.....",
 ".....0n0n0nm0.....",
 ".....0onnnnm0.....",
 "......0onnm0......",
 "......00nn0.......",
 ".....00pppp00.....",
 "....0pqppppppp0...",
 "...0pqpppppppp0...",
 "..0nqppppppppp0n..",
 "..0onppppppppon0..",
 "..0pnqppppppnp0...",
 ".0pqnq8888nppp0...",
 ".0pq0q866688p0p0..",
 ".0pq0q866688p0p0..",
 ".0pqq08888880pp0..",
 ".0qppp0pppp0ppp0..",
 ".0qpppppppppppp0..",
 "0qppppppppppppp0..",
 "0qpppppppppppppp0.",
 "0qpppppppppppppp0.",
 "0qppppppppppppppp0",
 "0qppppppppppppppp0",
 "0qppppppppppppppp0",
 "0qpppppppppppppppp",
 "0qpppppppppppppppp",
 "0qpppppppppppppppp",
 "00ppppppppppppppp0",
 ".00000000000000000",
])
