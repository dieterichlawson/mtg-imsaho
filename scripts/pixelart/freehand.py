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

# ══ HORROR PASS ══════════════════════════════════════════════════════
# The previous figures read as gummy worms because they were built the wrong
# way round: mid-tone masses, light keylines, and fully drawn faces. The
# references all say the opposite —
#
#   "muted blacks and deep greys, sharp silhouettes, and minimal but
#    high-contrast highlights ... highlights use a colder tint"
#   "facial details are minimal; expression comes from posture and
#    silhouette rather than explicit features"
#   "rely on silhouettes, contrast and suggestion rather than detail"
#
# So the rules for this pass:
#   1. The body is NEAR-BLACK. It is a hole in the picture, not an object.
#   2. Light touches only the edge facing the source (upper left). Two steps
#      of it at most: 5 fog, then 7 bone on the single brightest pixel.
#   3. NO drawn eyes, nose or mouth. The face is a void. At most one cold
#      glint or one blood pixel. A face you can read is a face you pity.
#   4. Silhouettes are angular. No rounded shoulders, no smooth hems.
#      Every outline changes direction sharply at least four times.
#   5. One saturated accent per figure and no more (b blood, or 8 candle).
#   6. Dither the light/dark boundary instead of using a mid-tone fill.

sprite("soldier_h")([
 ".....00000........",
 "....05222210......",
 "...0522222210.....",
 "...052000002 0....",
 "...052000002 0....",
 "....0520000210....",
 ".....05200210.....",
 ".....0522210......",
 "...0005222100.....",
 "..05222222221 0...",
 ".0572222222222 0..",
 ".057 222222222 0..",
 ".052 2222222222 0.",
 ".052 22222222222 0",
 ".057 2121212222 0.",
 ".052 2222222222 0.",
 ".052 2222222222 0.",
 "..05 22222222220..",
 "..05 2222222220...",
 "..00 222222222 0..",
 "....05222222220...",
 "....00122222100...",
 "....051200021 0...",
 "....051 0.0 210...",
 "...0512 0.0 210...",
 "...051 0...0 10...",
 "...051 0...0 10...",
 "..00510....0210...",
 "..051210...02100..",
 "..0512100..051100.",
 "..0000000..0000000",
])

sprite("zombie_h")([
 "...0000...........",
 "..0511 0..........",
 "..05 000..........",
 "..0 0000..........",
 "..05 0 0..........",
 "...0 00...........",
 "....0 0...........",
 "...00 100.........",
 "..05 1111 0.......",
 ".05 111111 0......",
 ".05 1 111111 0....",
 "05 1  1111111 0...",
 "05 1 5 111111 0...",
 "05 1  5 11111 0...",
 "05 1 5 5 1111 0...",
 "05   5 5 1111 00..",
 "051  5 5 111 0 0..",
 "05 1  5 5 11 0 0..",
 ".05 1  5 111 0 0..",
 ".05 1  5 11 0  0..",
 "..05 1  5 1 0  0..",
 "..05 1  5 0 0  0..",
 "...05 1  0 0  00..",
 "...00 1 0 0  05...",
 "......0 0 0 051...",
 ".....05 0 0 0 0...",
 ".....0 0.0 0..0...",
 "....05 0..0 0.....",
 "....0 0...05 0....",
 "...05 0...0 10....",
 "...0 10...05 0....",
 "...0000...0000....",
])

sprite("vampire_h")([
 "......00000.......",
 ".....0511150......",
 "....05100015 0....",
 "....0500b0b0 0....",
 "....05100001 0....",
 ".....0510010......",
 "......05110.......",
 "...00.0511.00.....",
 "..0510051100150...",
 ".05110051100 150..",
 ".0511005110000150.",
 "05 110051100000150",
 "05 1100511000001 0",
 "05  110511000000 0",
 "05  1105110000000 ",
 "0 5  110511000000 ",
 "0 5  11051100000 .",
 ".0 5  1105110000 .",
 ".0 5  110511000 ..",
 "..0 5 1105110 0 ..",
 "..0 5 11051100 ...",
 "...0 5110511 0 ...",
 "...0 511051 0 ....",
 "....0 51051 0.....",
 "....0 5105 0......",
 ".....0 500 0......",
 ".....00 0 00......",
 "....051 0 150.....",
 "....0 10.01 0.....",
 "....0110..0110....",
 "....0000..0000....",
])

sprite("scholar_h")([
 "......00000.......",
 ".....0511150......",
 ".....5100001 0....",
 ".....50 0 001 0...",
 ".....5100001 0....",
 "......510010......",
 "......051100......",
 ".....0051100......",
 "....0511111100....",
 "...05111111110....",
 "..05 111111111 0..",
 "..05  11111111 0..",
 "..051 11888111 0..",
 ".05 1 18777811 0..",
 ".05 1 18787811 0..",
 ".05 1 18777811  0.",
 ".05 1 11888111  0.",
 ".05 1 111111111 0.",
 ".05 11111111111 0.",
 "05  11111111111 0.",
 "05 111111111111 0.",
 "05 1111111111111 0",
 "05 1111111111111 0",
 "05  111111111111 0",
 "05 1111111111111 0",
 "05 1111111111111 0",
 "05  111111111111 0",
 "05 1111111111111 0",
 "005111111111111 0.",
 ".00000000000000 0.",
])

# ── HORROR PASS 2 ────────────────────────────────────────────────────
# Pass 1 got the value structure right and threw away the silhouette: four
# dark eggs you couldn't tell apart. But the references are explicit that in
# this style the silhouette does ALL the work, which means the silhouette has
# to be SHAPED. Rule 4 said "every outline changes direction sharply at least
# four times" and pass 1 simply didn't obey it.
#
# So: keep near-black bodies, void faces, cold rim on the lit edge only — and
# give each outline hard angles that name the character. Jutting pauldrons and
# a spear for the soldier. A pointed hood and a V hem for the scholar. Collar
# spikes and a torn cape for the vampire. A dropped shoulder and splayed ribs
# for the zombie. Three internal values, not one, so the mass has form.

sprite("soldier_h2")([
 "........000.......",
 "......005550......",
 ".....0577750......",
 "....05770007 0....",
 "....0570000 20....",
 "....0570000 20....",
 ".....057000 0.....",
 ".....05700 20.....",
 "......0522 0......",
 "...00005220000....",
 "..05777222222750..",
 ".057722222222270..",
 "0577022222222207 0",
 "057 022222222207 0",
 "057 0 2222222 07 0",
 "057 0 1111111 07 0",
 "057 0 2222222 07 0",
 "057 0 2222222 07 0",
 ".07 0 2222222 07 0",
 "..0 0 2222222 07 0",
 "..0 0 22222220 7 0",
 "......0222222007 0",
 "......02222220 7 0",
 ".......022220..7 0",
 ".......02 220..7 0",
 "......05 0 20..7 0",
 "......05 0 20..7 0",
 ".....057 0 20..7 0",
 ".....05 0.05 0.7 0",
 "....0570..0570.7 0",
 "....0000..0000.000",
])

sprite("zombie_h2")([
 "....0000..........",
 "...05770..........",
 "...07000..........",
 "...0b0 0..........",
 "...07 00..........",
 "....0 20..........",
 "....052 0.........",
 "...0057200........",
 "..057022 200......",
 ".0570 2 2 200.....",
 ".057 0 2 2 20.....",
 "057 0 7 2 2 20....",
 "057 0 7 2 2 200...",
 "05  0 7 2 2 2 0...",
 "05  0 7 2 2 2 0...",
 "057 0 7 2 2 2 00..",
 "05  0 7 2 2 2 0 0.",
 "05  0 7 2 22  0 0.",
 ".05 0 7 2 22  0 0.",
 ".05 0 7 2 2   0 0.",
 "..05 0 7 22   0 0.",
 "..05 0 7 2 0  0 0.",
 "...05 0 2 0  00 0.",
 "...0 0 2 0 0 05 0.",
 "....0 2 0 0 0570..",
 "....02 0 0 05 0...",
 "....0 0.0 0 0.....",
 "...05 0..0 20.....",
 "...0 20..05 0.....",
 "..057 0..0 20.....",
 "..0000...0000.....",
])

sprite("vampire_h2")([
 "......00000.......",
 ".....0577750......",
 "....057000750.....",
 "....070b0b070.....",
 "....0570007 0.....",
 ".....0570 20......",
 "0.....05 20.....0.",
 "00...0057200...000",
 "070..05722250..070",
 "0070057222225007 0",
 "00700572222250070.",
 "0070 0722222007 0.",
 ".070 0722222070 0.",
 ".0 70072222207 0..",
 ".0 700722222070 0.",
 "..0 70722222 70 0.",
 "..0 7072222207 0..",
 "...0 0722222 0 0..",
 "...0 07222220 0...",
 "....0 722222 0....",
 "....0 0722220 0...",
 ".....0 72222 0....",
 ".....0 0722 00....",
 "......0 722 0.....",
 "......0 72 20.....",
 ".......0 2 0......",
 ".......00 00......",
 "......057 0570....",
 "......0 20.0 20...",
 ".....057 0.057 0..",
 ".....0000..00000..",
])

sprite("scholar_h2")([
 ".......000........",
 "......05750.......",
 ".....0577750......",
 "....057000750.....",
 "....07000007 0....",
 "....0570000 0.....",
 ".....057002 0.....",
 ".....05702 20.....",
 "....005722 200....",
 "...0577222222 0...",
 "..05722222222 20..",
 "..0722222222222 0.",
 "..07 2228882222 0.",
 ".07 2 28777822 20.",
 ".07 2 28787822 20.",
 ".07 2 28777822  0.",
 ".07 2 22888222  0.",
 ".07 22222222222 0.",
 "07 222222222222 0.",
 "07 2222222222222 0",
 "07 2222222222222 0",
 "07 2222222222222 0",
 "07 22222222222222 ",
 "07 22222222222222 ",
 "07 22222222222222 ",
 "07 2222222222222 0",
 "07 222222222222 0.",
 "07 22222222222 0..",
 "0 022222222220 0..",
 "0.0.0.0.0.0.0.0...",
])

# ── HORROR PASS 3: BUSTS ─────────────────────────────────────────────
# Two passes of full figures failed the same way, so the premise is suspect.
# At 42x34 a whole body spends most of its pixels on torso and legs — the
# least characterful parts — and leaves four or five for a face. Horror
# portraiture does the opposite: crop to head and shoulders, let the rest go
# to black, and spend the pixels on the one thing that unsettles.
#
# Also fixing the rim: pass 2 traced light down the entire contour, which
# reads as a glowing outline sticker. Light lands on THREE short segments at
# most — a brow ridge, a cheekbone, a collar edge — and nowhere else.
# Contrast is pushed hard: near-black against bone, almost nothing between.

sprite("bust_vampire")([
 "......0000000.....",
 "....00777777000...",
 "...0770000007700..",
 "..077000000000700.",
 "..07000b000b00070.",
 "..0700000000000700",
 "..070007000700070.",
 "..0070000000007700",
 "...0070000000700..",
 "....00700070000...",
 ".....0007700......",
 "...0000700000.....",
 "..07700000077000..",
 ".0770000000000770.",
 "07700000000000077.",
 "07000000000000007.",
 "0700000000000000..",
 "070000000000000...",
 "07000000000000....",
 "0700000000000.....",
 "070000000000......",
 "00000000000.......",
])

sprite("bust_ghoul")([
 ".....00000........",
 "....0777770.......",
 "...077000770......",
 "..07700000770.....",
 "..070b00000700....",
 "..07000000b070....",
 "..0700000000700...",
 "..07000700007700..",
 "...070700000070...",
 "...00707000700....",
 "....0070700700....",
 ".....007070700....",
 "......0070700.....",
 "....00007000......",
 "...077000000......",
 "..07700000000.....",
 ".077000000000.....",
 "07700000000000....",
 "070000000000000...",
 "0700000000000000..",
 "07000000000000....",
 "0000000000000.....",
])

sprite("bust_inquisitor")([
 "..0000000000000...",
 ".07777777777770...",
 "0770000000000770..",
 "0000000000000000..",
 "...0700000070.....",
 "...070000000700...",
 "...07000000000....",
 "...07000000000....",
 "....0700070000....",
 "....07000000700...",
 ".....00700070.....",
 "......0000000.....",
 "....0007700000....",
 "..077000000770....",
 ".07700000000770...",
 "0770000000000770..",
 "0700000000000007..",
 "070000700000000...",
 "07000070000000....",
 "0700000700000.....",
 "070000000000......",
 "00000000000.......",
])

# ── HORROR PASS 4: LIT MASS, NOT LIT OUTLINE ─────────────────────────
# Every failed pass shares one mistake, and it took four to name it: I kept
# putting the bright value on the CONTOUR. A bone-coloured outline around a
# dark fill is a sticker, or a wireframe — never a body. A face lit in
# darkness is the reverse: the flesh is the bright mass, the FEATURES are the
# holes, and the edge falls off into black with nothing outlining it at all.
#
# So here the skin is 6/7, the eye sockets and the hollows under the cheek
# and jaw are 0, and there is no keyline anywhere on the silhouette — the
# form ends because the light ends.

sprite("bust_vampire2")([
 "........00000.....",
 ".....006666660....",
 "....0667777766 0..",
 "...06777777777600.",
 "...0677777777776 0",
 "..0677777777777760",
 "..0670000700000760",
 "..067b00070000b760",
 "..06777007000777 0",
 "..0677770007777760",
 "..0677777077777760",
 "...06777707777760.",
 "...0677700007776 0",
 "...067770000077760",
 "....0677777777760.",
 "....06550000556 0.",
 ".....0550000055 0.",
 ".....0500000005 0.",
 "....055000000055..",
 "...0550000000055..",
 "..05500000000005..",
 "..0500000000000...",
 ".05000000000000...",
 ".0000000000000....",
])

sprite("bust_ghoul2")([
 ".......00000......",
 "......0666660.....",
 ".....06777776 0...",
 "....0677777777 0..",
 "....067777777760..",
 "...06770077007760.",
 "...067b0007000b760",
 "...0677000700007 0",
 "...06777007000776.",
 "...067770007777 0.",
 "...0677770077760..",
 "...06770707076 0..",
 "....0670707070 0..",
 "....06707070706 0.",
 "....066070707660..",
 ".....0660007660...",
 ".....05600066 0...",
 "....0550000055 0..",
 "...05500000005 0..",
 "..0550000000005 0.",
 "..050000000000 0..",
 ".05000000000000...",
 ".000000000000.....",
])

sprite("bust_inquisitor2")([
 "..00000000000000..",
 ".05555555555555 0.",
 "0555555555555555 0",
 "0000000000000000 0",
 "...06666666660....",
 "...0677777777 0...",
 "..067000700077 0..",
 "..06700070000760..",
 "..0677700700776 0.",
 "..06777007007760..",
 "..067770000077 0..",
 "...0677700077760..",
 "...06777777777 0..",
 "....067777777760..",
 "....0655000556 0..",
 "...05500000005 0..",
 "..0550000000005 0.",
 "..05000tt0000005..",
 ".050000tt00000005.",
 ".05000ttt0000000..",
 "05000tt000000000..",
 "0500ttt0000000....",
 "000tt00000000.....",
])

# ── PASS 5: CONTOURS WITH GEOMETRY ───────────────────────────────────
# The blobbiness turned out to be measurable. Every previous sprite's outline
# advanced +/-1 px every row or two with irregular run lengths and almost no
# steps of 2+, which is literally how a scanline algorithm rasterizes an
# ellipse. I had been hand-drawing ovals without meaning to.
#
# These are built from deliberate segments instead: a long flat run, then a
# diagonal whose run lengths are all EQUAL, then a hard corner of 2px or
# more. contour.py scores it, so "does this read as a drawn shape" stops
# being a matter of opinion.

sprite("bust_vampire3")([
 "......00000000....",   # flat crown
 "....007777777700..",
 "...07777777777770.",
 "...07777777777770.",
 "...07777777777770.",
 "...07777777777770.",
 "...07777777777770.",   # 5-row vertical flat: the temple
 ".....0000077000...",   # CORNER: cheek steps in 2
 ".....0b00077000...",
 ".....077007700b0..",
 ".....07700770000..",
 ".....07777777000..",   # 4-row flat: the cheek
 "......077777700...",   # diagonal, run 1
 ".......0777700....",   # diagonal, run 1
 "........07700.....",   # diagonal, run 1 — a clean 45 deg jaw
 "........07700.....",
 "........05500.....",   # neck, flat
 "........05500.....",
 "..0000005500000...",   # CORNER: shoulders jump out 6
 "0055555555555555 0",
 "0500000000000000 0",
 "0500000000000000 0",
 "0500000000000000 0",
 "0500000000000000 0",   # long flat shoulders
])

sprite("bust_ghoul3")([
 ".......000000.....",
 ".....00777777000..",
 "....077777777770..",
 "....077777777770..",
 "....077777777770..",
 "....077777777770..",
 "....077777777770..",   # flat temple
 "......0000770000..",   # CORNER
 "......0b00770b00..",
 "......07007700700.",
 "......07007700700.",
 "......07777777700.",   # flat cheek
 "......00707070700.",   # bared teeth: hard verticals, no curve
 "......00707070700.",
 ".......070707000..",   # diagonal, run 1
 "........0707000...",   # diagonal, run 1
 ".........07000....",   # diagonal, run 1
 ".........05500....",
 "......0000550000..",   # CORNER
 "..00555555555500..",
 "..0500000000000...",
 "..0500000000000...",
 "..0500000000000...",
])

sprite("bust_inquisitor3")([
 "0000000000000000 0",   # hat brim: one long flat, hard ends
 "0555555555555555 0",
 "0000000000000000 0",
 ".....00000000.....",   # CORNER: crown steps in 5
 ".....07777770.....",
 ".....07777770.....",
 ".....07777770.....",   # flat crown
 "....0777777770....",   # CORNER out 1
 "....0700770070....",
 "....0700770070....",
 "....0777777770....",
 "....0770770770....",
 "....0777777770....",   # flat cheek
 ".....07777770.....",   # diagonal, run 1
 "......077770......",   # diagonal, run 1
 ".......0770.......",   # diagonal, run 1
 ".......0550.......",
 "....000055000000..",   # CORNER
 "..0055555555555500",
 "..050000tt00000000",
 "..05000tt000000000",
 "..0500tt0000000000",
 "..050tt00000000000",
])

# ── PASS 6: FACETED, NOT CURVED, NOT BOXED ───────────────────────────
# Blown up, pass 4 is a smooth oval and pass 5 is a literal rectangle. The
# thing a skull actually is sits between them: a POLYGON of five or six
# planes — crown, temple, cheekbone, jaw, chin — each a short straight run at
# a DIFFERENT angle, meeting at corners you can point to. Curve has no
# corners; box has only right angles; a face has neither.
#
# Interior likewise: the brow is a hard shadow band, the eye socket is an
# angled wedge rather than a rectangle, the nose ridge is the one lit edge,
# and the cheek hollow runs on a diagonal.

sprite("bust_vampire4")([
 "......077770......",   # crown: shallow, two steps
 ".....07777770.....",
 "....0777777770....",
 "....0777777770....",
 "....0777777770....",
 "....0777777770....",   # temple: vertical run of 4
 "....0555555550....",   # brow: hard horizontal shadow band
 "....05000000050...",
 "....0b000000b50...",   # eye sockets: wedges, blood at the outer corner
 ".....000770000....",
 ".....06077060.....",   # cheekbone steps in 1 — corner
 "......0677060.....",   # nose ridge is the lit edge
 "......0677600.....",
 "......0655500.....",   # cheek hollow, on the diagonal
 ".......06660......",   # jaw: 1:2 diagonal
 ".......05550......",
 "........050.......",   # chin
 "........050.......",
 ".......05550......",   # neck
 "....000555000.....",
 "..0055555555000...",   # collar: hard corner out
 ".05500000000550...",
 "05500000000005500.",
 "0500000000000005 0",
])

sprite("bust_ghoul4")([
 ".......07770......",
 "......0777770.....",
 ".....077777770....",
 ".....077777770....",
 ".....077777770....",
 ".....077777770....",
 ".....055555550....",   # brow band
 ".....05000000500..",
 ".....0b00000b500..",   # sunken sockets
 "......00007700....",
 "......0607700.....",   # cheekbone corner
 "......06770600....",
 "......0677600.....",
 "......0700600.....",   # nasal void
 ".......07060......",
 ".......00700......",
 "......0707070.....",   # bared teeth: hard verticals
 "......0707070.....",
 ".......05550......",
 "......005550......",
 "....00555550000...",
 "..005555000005500.",
 "..05000000000005..",
 "..0500000000000...",
])

sprite("bust_inquisitor4")([
 "..00000000000000..",   # brim: one flat, hard ends
 ".0555555555555550.",
 "..00000000000000..",
 "......077770......",   # crown corner in
 ".....07777770.....",
 ".....07777770.....",
 ".....07777770.....",   # temple vertical
 ".....055555500....",   # brow band, offset right — the head is turned
 ".....0500000500...",
 ".....0500000500...",   # sockets in shadow under the brim
 "......000770......",
 "......0607700.....",   # cheekbone
 "......06776000....",
 "......0655600.....",
 ".......06660......",   # jaw
 ".......05550......",
 "........050.......",
 "........050.......",
 ".......05550......",
 "....000555000.....",
 "..005555555500....",
 ".05500tt00005500..",   # collar, and the rapier crossing it
 "05000tt000000005..",
 "0000tt0000000000..",
])

# ── PASS 7: KEEP THE MASS ────────────────────────────────────────────
# Pass 6 added facets and then carved so much out of the face that only 1px
# fragments were left — the mass, which was the whole point of pass 4, was
# gone again. And the crown was still a smooth dome.
#
# So the skull is a COFFIN HEXAGON: a flat top, two chamfered upper corners,
# straight vertical sides, two chamfered lower corners, a flat chin. Six
# clean segments, gothic by construction, and crisp because every segment is
# straight. The face stays a solid pale mass; the features are small dark
# marks cut into it, never large enough to break it up.

sprite("v_vampire")([
 "......077770......",
 ".....07777770.....",   # chamfer
 "....0777777770....",   # chamfer
 "....0777777770....",
 "....0777777770....",
 "....0777777770....",   # straight sides
 "....0755555570....",   # brow band
 "....0700770070....",   # eye slots
 "....07b0770b70....",   # blood glint at the outer corner of each
 "....0777667770....",
 "....0777667770....",   # nose shadow, one column
 "....0777667770....",
 "....0776666770....",
 ".....07766770.....",   # chamfer
 "......077770......",   # chamfer
 ".......0550.......",   # chin
 ".......0550.......",
 "......055550......",   # neck
 "....0005555000....",
 "..00555555555500..",   # collar: hard corner
 ".0550000000000550.",
 "05500000000000055.",
 "0500000000000000 0",
 "0500000000000000 0",
])

sprite("v_ghoul")([
 "......077770......",
 ".....07777770.....",
 "....0777777770....",
 "....0777777770....",
 "....0777777770....",
 "....0777777770....",
 "....0755555570....",
 "....0700770070....",
 "....07b0770b70....",
 "....0770660770....",
 "....0770060770....",   # nasal void, narrow
 "....0777667770....",
 "....0700000070....",   # the jaw has come away
 ".....07070700.....",   # teeth: hard verticals
 "......070700......",
 ".......0550.......",
 "......055550......",
 "....00055500000...",
 "..005555550005500.",
 ".05500000000000550",
 "05500000000000005.",
 "0500000000000000..",
 "0500000000000000..",
])

sprite("v_inquisitor")([
 "..00000000000000..",   # hat brim
 ".0555555555555550.",
 "..00000000000000..",
 "......077770......",   # crown, stepped in hard
 ".....07777770.....",
 "....0777777770....",
 "....0777777770....",
 "....0755555570....",   # brow, deep in the brim's shadow
 "....0700770070....",
 "....0700770070....",
 "....0777667770....",
 "....0777667770....",
 "....0776666770....",
 ".....07766770.....",
 "......077770......",
 ".......0550.......",
 "......055550......",
 "....0005555000....",
 "..00555555555500..",
 ".05500tt00000550..",   # the rapier crosses the collar
 "05500tt0000000055.",
 "0500tt0000000000..",
 "050tt00000000000..",
])

# ── PASS 8: TURN THE HEAD ────────────────────────────────────────────
# The coffin hexagon fixed the blobbiness but left a perfectly mirrored face,
# which reads as a mask or an icon rather than a person. Turning the head a
# few degrees breaks it: the far chamfer is longer than the near one, the
# features sit one column off centre, the brow band is shorter on the far
# side, and one shoulder rides higher than the other.

sprite("t_vampire")([
 "......07777 0.....",
 ".....077777770....",   # far chamfer is longer
 "....07777777770...",
 "....07777777770...",
 "....07777777770...",
 "....07777777770...",
 "....075555555 0...",   # brow stops short on the far side
 "....0700770000 0..",
 "....07b077000b 0..",   # near eye reads full, far eye is foreshortened
 "....07776607770...",
 "....07776607770...",
 "....07766607770...",   # nose sits one column right of centre
 "....077666677 0...",
 ".....077666770....",
 "......0777770.....",
 ".......05550......",
 ".......0550.......",
 "......055500......",
 "....000555000.....",
 "..0055555555000...",
 ".05500000000550...",   # near shoulder rides higher
 "05500000000005500.",
 "0500000000000005 0",
 "050000000000000 0.",
])

sprite("t_ghoul")([
 ".....077770.......",
 "....07777770......",
 "....077777770.....",
 "...0777777770.....",
 "...0777777770.....",
 "...0777777770.....",
 "...07555555 0.....",
 "...0700770000.....",
 "...07b077000b0....",
 "...0770660770.....",
 "...0770060770.....",
 "...07766607700....",
 "...0700000070.....",
 "....070707000.....",   # jaw hangs to one side
 ".....0707000......",
 "......05500.......",
 ".....0555000......",
 "...00055500000....",
 "..0555555000550...",
 ".0550000000000550.",
 "05500000000000055.",
 "0500000000000000..",
 "050000000000000...",
])

sprite("t_inquisitor")([
 ".0000000000000000.",   # brim tilted: lower on the near side
 "0555555555555555 0",
 ".00000000000000 0.",
 ".....077770.......",
 "....07777770......",
 "....077777770.....",
 "....077777770.....",
 "....07555555 0....",
 "....0700770000....",
 "....070077000 0...",
 "....07776607770...",
 "....07776607770...",
 "....07766607770...",
 ".....0776667700...",
 "......07777770....",
 ".......055500.....",
 ".......0550.......",
 "......055500......",
 "....000555000.....",
 "..00555555555500..",
 ".05500tt000005500.",
 "05500tt0000000055.",
 "0500tt00000000000.",
])
