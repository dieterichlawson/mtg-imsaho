"""The 32 card illustrations. Each returns a 42x34 grid of palette chars.

Style rules enforced across the set:
  1. Silhouette first — subject reads as a shape at 1x.
  2. Subject occupies 55-80% of art height, with a hard dark keyline.
  3. One dominant light source, backlighting the subject.
  4. One dominant hue per card so the set reads varied at board scale.
  5. Innistrad palette: moonlit violet nights, bone, candle gold, blood.
"""
from engine import Canvas
import math

ART = {}
def card(name):
    def deco(fn): ART[name] = fn; return fn
    return deco

def moon(cv, cx, cy, r, disc='7'):
    cv.disc(cx, cy, r+4, '2'); cv.disc(cx, cy, r+2, '3')
    cv.disc(cx, cy, r+1, '4'); cv.disc(cx, cy, r, disc)

def night(cv, a='1', b='2', c='3'):
    cv.gradient_sky([(0,a),(0.42,b),(0.76,c)])

def treeline(cv, y, c='0', seed=1, h=6):
    import random
    rng = random.Random(seed)
    for x in range(cv.w):
        cv.vline(x, y - rng.randrange(1, h), cv.h-1, c)

# ══ WHITE ════════════════════════════════════════════════════════════
@card("Geist of Saint Traft")
def _():
    cv = Canvas(fill='0')
    cv.gradient_sky([(0,'0'),(0.25,'1'),(0.85,'1')])
    cv.disc(21, 20, 16, '2'); cv.disc(21, 20, 12, '1')   # pooled crypt glow
    # seated hooded spirit, translucent, dissolving at the hem
    cv.stamp(13, 4, [
      ".....0000.....","....0ffff0....","...0fg66gf0...","..0fg6111g f..",
      "..0f611111 f..","..0f61g1g16f..","..0fg611116f..","...0fg6116f...",
      "..0ffg6666gf..",".0fgg666666gf.",".0fg66g88g66gf","0fg66g8888g66g",
      "0fg666g88g666g","0fgg66666666gf",".0fg66666666f.",".0fg6666666gf.",
      "..0fg666666f..","..0fg66666gf..","..0f g6666f...","...0f g66 f...",
      "...0f g6 f0...","....f .6. f...","....0 .g. .0..","...f . . . f..",
    ])
    # the field of floating candles — varied size and depth is the whole card
    cands = [(3,24,1),(8,16,1),(5,29,1),(15,27,1),(2,11,0),(36,15,1),
             (39,25,1),(31,29,1),(28,9,0),(37,7,0),(10,8,0),(33,20,1),
             (19,31,1),(25,32,1),(41,18,0),(6,20,0),(12,12,0),(35,31,1)]
    for (x, y, big) in cands:
        cv.disc(x, y, 3.0 if big else 2.0, '2')
        cv.disc(x, y, 1.6 if big else 1.1, '3')
        cv.vline(x, y+1, y+(4 if big else 2), '6')
        cv.set(x, y, '8'); cv.set(x, y-1, '7')
        if big: cv.set(x, y+1, '9')
    return cv

@card("Doomed Traveler")
def _():
    cv = Canvas(); night(cv, '1','1','2')
    moon(cv, 33, 6, 5)
    cv.stars(20, '3', seed=4, maxy=16)
    # the village on the ridge, warm windows
    cv.poly([(21,19),(42,11),(42,24),(21,24)], '0')
    for (x, y, w) in ((24,15,5),(30,12,6),(37,13,5)):
        cv.rect(x, y, x+w, y+6, '0')
        cv.poly([(x-1,y),(x+w//2,y-3),(x+w+1,y)], '0')
        cv.set(x+1, y+2, '8'); cv.set(x+w-1, y+3, '9'); cv.set(x+2, y+4, '8')
    # cobbled road sweeping up to it
    cv.poly([(0,33),(12,22),(23,20),(28,22),(16,33)], '3')
    cv.poly([(0,33),(11,24),(21,22),(24,23),(13,33)], '4')
    cv.noise(0, 22, 28, 33, '5', 0.10, seed=7)
    cv.noise(0, 22, 28, 33, '3', 0.10, seed=11)
    # the traveler — tall spear, pack, cloak. Reads as a lone silhouette.
    cv.line(6, 2, 9, 25, '0'); cv.line(7, 2, 10, 25, '6')
    cv.poly([(6,2),(9,0),(8,5)], '6')                     # spearhead
    cv.stamp(6, 11, [
      "...0000...","..0tts t0.","..0tnnot0.","..0snnos0.","...0nno0..",
      "..0kllk0..",".0khiihk0.","0khiiiihk0","0khiiiihk0","0kkhiihkk0",
      "0k khiih k","0k 0hiih0 k",".0 0hiih0 .","...0hh h0.","...0k. k0.",
      "...0k. k0.","...0k. k0.","..00k. k00",
    ])
    return cv

@card("Chapel Geist")
def _():
    cv = Canvas(fill='1')
    for x in (5, 33):                                           # narrower, offset
        cv.rect(x-3, 4, x+3, 27, '2'); cv.disc(x, 5, 3.2, '2')
        cv.vline(x, 4, 27, '2')
    cv.rect(0, 28, 41, 33, '1')
    # A robed geist banked left-to-right. Mass pushed left; nothing symmetric.
    cv.stamp(17, 3, ["00000","06660","6060 6","06560","0 60 ",".060."])
    cv.hline(9, 12, 25, '7')                                    # 14px shoulder row
    cv.stamp(10, 10, [
      "0777777777777 0","07777777777777 ","077777777777770","0777777777777 0",
      "07777777777760.","0777777777760..",".0777777776 0..",".077777777 0...",
      "..077777777 0..","..07777777 0...","...0777777 0...","...077777 0....",
      "..07777 76 0...","..0777 7 6 0...",".077 7 . 6 0...",".07 . . . 0....",
      "0 7 . . . 0....","0 . . ... .....",
    ])
    cv.limb(13, 12, 8, 17, '7'); cv.hand(6, 17, '6')            # left arm, out low
    cv.limb(24, 12, 29, 15, '7'); cv.hand(29, 15, '6')          # right arm, out high
    for i, y in enumerate(range(19, 26)):                       # chain to reliquary
        cv.set(31 + i//3, y, '9')
    cv.stamp(32, 26, ["0990","98 9","9889","0990"])             # the reliquary
    for y in range(11, 24): cv.set(10, y, '6')                  # rim on the lit edge
    return cv

@card("Elite Inquisitor")
def _():
    cv = Canvas(fill='1')
    # Light source: the cold windows BEHIND him. So he is backlit, rim on both
    # edges, and the coat's interior stays dark.
    for x in (6, 20, 34):
        cv.poly([(x-5,29),(x-5,10),(x,3),(x+5,10),(x+5,29)], '3')
        cv.poly([(x-4,28),(x-4,10),(x,5),(x+4,10),(x+4,28)], '5')
        cv.vline(x, 5, 28, '3'); cv.hline(15, x-4, x+4, '3'); cv.hline(22, x-4, x+4, '3')
    cv.rect(0, 29, 41, 33, '2'); cv.hline(29, 0, 41, '3')
    # rapier: up and to the right, so the pose is asymmetric
    cv.line(38, 1, 25, 12, '0'); cv.line(38, 2, 25, 13, '7'); cv.line(37, 2, 24, 13, '6')
    cv.poly([(24,12),(26,11),(27,14),(25,15)], '9')            # crossguard
    # head (5 wide) then a 13-wide shoulder row — the ratio that kills "blob"
    cv.stamp(16, 8, ["00000","0nnn0","n0n0n","0nnn0","0mmn0",".0n0."])
    cv.hline(6, 13, 25, 'k'); cv.hline(7, 12, 26, 'k')          # wide-brim hat
    cv.hline(5, 15, 23, 'l'); cv.hline(4, 16, 22, 'k')
    cv.hline(14, 12, 28, '7')                                   # THE shoulder row
    cv.stamp(13, 15, [
      "0666666666660","06777777777 0","0677777777760","0677711177760",
      "06777111177 0","0677711117760","06777777777 0",".067777777760",
      ".067777777760",".06777777 760",".0677777 7760","..067777 76 0",
      "..06777. 7760","..0666. .6660","..0222. .2220",
    ])
    cv.limb(15, 15, 11, 24, '6'); cv.hand(9, 24, '6')          # left arm, low
    cv.limb(26, 15, 25, 13, '6'); cv.hand(24, 12, 'n')         # right arm, on the hilt
    for y in range(15, 28): cv.set(12, y, '5')                 # rim from the window
    return cv

@card("Midnight Haunting")
def _():
    cv = Canvas(fill='0')
    cv.gradient_sky([(0,'0'),(0.35,'1'),(0.9,'1')])
    # the window: hot orange interior behind cold stone tracery
    cv.rect(4, 13, 22, 31, '3'); cv.rect(5, 14, 21, 30, '2')
    cv.rect(6, 15, 20, 29, 'a'); cv.rect(7, 16, 19, 28, 'c')
    cv.noise(7, 16, 19, 28, 'd', 0.34, seed=3)
    cv.noise(8, 17, 18, 27, '8', 0.14, seed=9)
    cv.vline(13, 14, 30, '3'); cv.hline(22, 5, 21, '3'); cv.hline(16, 5, 21, '3')
    # one coherent ribbon of spirits sweeping up and out of the frame
    path = [(16,24),(12,20),(14,14),(20,11),(27,10),(33,7),(38,3),(41,0)]
    cv.stroke(path, [5,4.5,4,3.6,3.2,2.8,2.4,2], '5')
    cv.stroke(path, [3.4,3,2.8,2.4,2.2,1.8,1.6,1.2], '6')
    cv.stroke(path, [1.8,1.6,1.4,1.2,1.0,0.9,0.8,0.6], '7')
    for (hx, hy, r) in ((14,18,2.6),(23,11,2.2),(34,6,1.8)):   # faces, barely there
        cv.disc(hx, hy, r, '7'); cv.disc(hx, hy-r*0.35, r*0.55, '5')
        cv.set(int(hx), int(hy+r*0.5), '5')
    return cv

# ══ BLUE ═════════════════════════════════════════════════════════════
@card("Snapcaster Mage")
def _():
    cv = Canvas(fill='1')
    cv.gradient_sky([(0,'1'),(0.5,'2'),(0.82,'2')])
    cv.poly([(27,29),(27,9),(34,3),(41,9),(41,29)], '2')       # arch, offset right
    cv.poly([(29,29),(29,10),(34,5),(39,10),(39,29)], '1')
    cv.vline(34, 5, 29, '2'); cv.rect(0, 29, 41, 33, '1')
    cv.stamp(2, 15, ["0.0..","03330","0333.",".000."])
    # Built shoulders-first: head 6px at x18-23, shoulder row 16px at x15-30.
    cv.stamp(18, 6, ["0kkkk0","0knnk0","0n00n0","0nnnn0","0mnnm0",".0nn0."])
    cv.stamp(15, 12, [
      "0tttttttttttttt0","0tsssssssssssst0","0tseeeeeeeeeest0",
      "0seeeeeeeeeeees0","0seeeeeeeeeees0.","0tseeeeeeeeest0.",
      "0seeeeeeeeees0..","0seeeeeeeeas0...","0seeeeeeeas0....",
      "0seeeeeeas0.....","0tseeeeas0......","0seeeeas0.......",
      "0seeas0.........","002220..........",
    ])
    cv.limb(17, 14, 11, 21, 's'); cv.hand(9, 22, 'n')          # left, onto the flask
    cv.limb(28, 14, 31, 22, 's'); cv.hand(30, 23, 'n')         # right, hanging low
    cv.disc(10, 24, 4.2, 'e'); cv.disc(10, 24, 2.8, 'f')       # the arm-lantern
    cv.disc(10, 24, 1.5, 'g'); cv.set(9, 23, '7'); cv.ring(10, 24, 5.2, 'e')
    for y in range(13, 26): cv.set(14, y, 'f')                 # lit by his own flask
    return cv

@card("Delver of Secrets")
def _():
    cv = Canvas(fill='1')
    cv.rect(0, 0, 41, 33, '2'); cv.rect(0, 25, 41, 33, '1')
    cv.rect(1, 16, 8, 31, '1'); cv.rect(2, 17, 7, 30, 'e')      # jars, NOT mirrored
    cv.rect(2, 22, 7, 30, 'h'); cv.hline(16, 1, 8, '3')
    cv.rect(30, 20, 41, 31, '1'); cv.rect(31, 21, 40, 30, 'e')
    cv.rect(31, 25, 40, 30, 'h'); cv.hline(20, 30, 41, '3')
    for x in range(33, 41, 2): cv.vline(x, 3, 14, '3')          # birdcage
    cv.hline(3, 32, 41, '3'); cv.hline(14, 32, 41, '3')
    cv.stamp(15, 5, ["00000","0ooo0","o0o0o","0ooo0","0omo0",".0o0."])  # bald head
    cv.hline(11, 11, 26, 'p')                                    # 16px shoulder row
    cv.stamp(11, 11, [
      "0pppppppppppppp0","0pepppppppppppe0","0peppppppppppp 0",
      "0pepppppppppp 0.","0peppppppppp 0..","0pepppppppp 0...",
      "0peppppppppp0...","0pepppppppp0....",".0pppppppp0.....",
      ".0pppppppp0.....",".0pppppppp0.....","..00000000......",
    ])
    cv.limb(13, 12, 9, 19, 'p'); cv.hand(7, 19, 'o')            # left hand, low
    cv.limb(24, 12, 29, 6, 'p'); cv.hand(29, 3, 'o')            # right hand, raised
    cv.disc(31, 2, 3.4, '2'); cv.disc(31, 2, 2.0, '9')          # the burning specimen
    cv.stamp(28, 0, ["0d.d0","d8d8d",".d8d.","0d.d0"])
    for y in range(11, 22): cv.set(26, y, '9')                  # lit from the moth
    return cv

@card("Invisible Stalker")
def _():
    cv = Canvas(fill='2')
    cv.gradient_sky([(0,'2'),(0.4,'3'),(0.8,'3')])
    for y in range(2, 24, 3):
        cv.hline(y, 0, 41, '3')
        for x in range(((y//3) % 2)*4, 42, 8): cv.vline(x, y, y+2, '3')
    cv.rect(0, 24, 41, 33, '3'); cv.hline(24, 0, 41, '2')
    cv.hline(8, 13, 27, 'k'); cv.hline(7, 14, 26, 'l')          # brim
    cv.hline(6, 16, 24, 'k'); cv.hline(5, 17, 23, 'l'); cv.hline(4, 18, 22, 'k')
    # the coat: collar, shoulder row, taper, flare, torn hem. Khaki, not brass.
    COAT = [(11,16,24),(12,14,26),(13,13,27),(14,13,27),(15,14,26),(16,14,26),
            (17,15,25),(18,15,25),(19,15,25),(20,14,26),(21,14,26),(22,13,27),
            (23,13,27),(24,12,28),(25,12,28),(26,11,29),(27,11,29)]
    for (y, x0, x1) in COAT:
        cv.hline(y, x0, x1, '0'); cv.hline(y, x0+1, x1-1, 'k'); cv.hline(y, x0+2, x1-2, 'l')
    for x in range(12, 30, 3): cv.set(x, 28, 'k'); cv.set(x+1, 29, '0')  # torn hem
    for y in range(11, 15): cv.hline(y, 18, 22, '1')            # empty collar only
    for y in range(12, 27): cv.set(20, y, '0')                  # the open front
    cv.limb(26, 13, 33, 18, 'l', w=3); cv.stamp(32, 19, ["0kk0","k11k","k11k","0kk0"])
    for y in range(12, 28): cv.set(13, y, '5')                  # wet rim-light
    for i in range(22):                                          # rain, restrained
        x = (i*17) % 8 + (0 if i % 3 == 0 else (34 if i % 3 == 1 else 4))
        cv.line(x, (i*11) % 30, x-2, (i*11) % 30 + 3, '4')
    return cv

@card("Laboratory Maniac")
def _():
    cv = Canvas(fill='0')
    cv.rect(0, 0, 41, 33, '1')
    cv.poly([(0,0),(14,0),(4,33),(0,33)], '2')     # the machine, left
    cv.rect(0, 4, 9, 26, '1'); cv.rect(1, 5, 8, 25, '2')
    for y in (8, 13, 18, 23): cv.hline(y, 1, 8, '3')
    for (x, y) in ((2,9),(5,14),(2,19),(6,21)):    # green reagent vials
        cv.rect(x, y, x+2, y+3, '0'); cv.rect(x, y+1, x+2, y+3, 'i')
        cv.set(x+1, y+1, 'j')
    for i in range(30, 42, 3): cv.vline(i, 4, 28, '2')
    # arcs of blue lightning above the raised arm
    for pts in ([(20,2),(24,6),(21,9),(26,13)], [(30,3),(27,7),(31,10),(28,14)],
                [(24,1),(30,5),(26,8),(33,11)]):
        for i in range(len(pts)-1): cv.line(*pts[i], *pts[i+1], 'g')
        for i in range(len(pts)-1): cv.line(pts[i][0]+1, pts[i][1], pts[i+1][0]+1, pts[i+1][1], 'f')
    # the maniac: arms flung up, apron, goggles
    cv.stamp(17, 11, [
      "..0000000..",".0nnnnnnn0.",".0n2nnn2n0.",".0nnnnnnn0.","..0nnnnn0..",
      "0.0ekkke0.0","0e0eeeee0e0","0ee0eee0ee0","0eeekkkeee0",".0eekkkee0.",
      ".0e6kkk6e0.",".0e6kkk6e0.","..06kkk60..","..0ekke0...","..0e.ke0...",
      "..0e.ke0...","..00.00....",
    ])
    cv.line(17, 12, 13, 4, 'e'); cv.line(28, 12, 32, 4, 'e')
    cv.line(18, 12, 14, 4, 'n'); cv.line(27, 12, 31, 4, 'n')
    return cv

@card("Stitched Drake")
def _():
    cv = Canvas(fill='2')
    cv.gradient_sky([(0,'2'),(0.34,'3'),(0.72,'4')])
    moon(cv, 32, 7, 5, disc='6')
    cv.poly([(0,34),(0,26),(7,20),(14,27),(22,24),(29,29),(36,22),(41,27),(41,34)], '1')
    for x in (11, 34): cv.poly([(x-2,28),(x,19),(x+2,28)], '0')
    # near wing forward and large; far wing small and behind. Never symmetric.
    cv.poly([(19,16),(7,3),(0,2),(0,11),(5,20),(13,21)], '0')
    cv.poly([(19,16),(8,5),(2,5),(2,10),(7,18),(14,20)], 'a')
    cv.poly([(22,16),(31,10),(38,10),(37,15),(31,19),(26,20)], '0')
    cv.poly([(22,16),(31,11),(36,11),(35,15),(30,18),(26,19)], 'a')
    for (wx, wy) in ((0,3),(3,10),(8,17),(37,11),(33,16)):   # finger struts
        cv.line(20, 16, wx, wy, '0')
    cv.stamp(16, 13, ["..0ss0..",".0s66s0.","0s6006s0","0s6006s0",
                      "0s6006s0",".0s66s0.","..0ss0..","..0ss0.."])
    for i, (nx, ny) in enumerate([(18,13),(16,11),(13,9),(10,8),(8,9)]):
        cv.disc(nx, ny, 1.7-i*0.12, '0')
    cv.stamp(4, 6, ["00ss0","0s66s","s600b","0s660","..000"])   # skull, jaw, eye
    for y in (15, 18, 21):
        cv.hline(y, 16, 23, '0')
        for x in range(16, 24, 2): cv.set(x, y-1, '6')
    cv.line(20, 25, 26, 31, 's'); cv.line(26, 31, 21, 33, 's')
    return cv

@card("Liliana of the Veil")
def _():
    cv = Canvas(fill='1')
    cv.gradient_sky([(0,'1'),(0.3,'2'),(0.75,'2')])
    for (x, w) in ((0,3),(6,2),(11,4),(18,2),(23,3),(29,5),(36,2)):   # irregular drape
        cv.rect(x, 0, x+w, 33, '9'); cv.vline(x, 0, 33, '8')
        cv.rect(x+w+1, 0, x+w+2, 33, '2')
    cv.noise(0, 0, 41, 33, '2', 0.10, seed=5)
    cv.stamp(17, 2, ["0kkkk0","kkkkkk","k0kk0k","0knnk0","0nmmn0",".0nn0."])
    cv.hline(9, 14, 28, 'k')                                     # hair over shoulders
    cv.hline(10, 13, 29, 'p')                                    # 17px shoulder row
    cv.stamp(13, 10, [
      "0ppppppppppppppp0","0pkppppppppppkp0",".0kpppppppppppk0",
      ".0k7ppppppppp7k0",".0kp7pppppppk 0.","..0kppppppppk0..",
      "..0kpppppppk 0..","..0appppppa0....","..0aaaaaaaa0....",
      ".0aaaaaaaaaa0...",".0aaaaaaaaaa0...","0aaaaaaaaaaaa0..",
      "0aaaaaaaaaaaa0..","0aaaaaaaaaaaa0..",".0aaaaaaaaaa0...",
      "..0000000000....",
    ])
    cv.limb(14, 12, 8, 18, 'o'); cv.hand(6, 19, 'o')            # both arms end in
    cv.limb(27, 12, 32, 16, 'o'); cv.hand(32, 17, 'o')          # hands the flame sits in
    for (cx, cy) in ((7, 21), (33, 19)):                        # flame, touching the hand
        cv.disc(cx, cy, 3.6, 'a'); cv.disc(cx, cy, 2.5, 'c')
        cv.disc(cx, cy, 1.4, 'd'); cv.set(cx, cy-1, '8')
        cv.set(cx, cy-4, 'c'); cv.set(cx+1, cy-5, 'd')
    for y in range(10, 25): cv.set(12, y, 'c')                  # lit by her own flame
    return cv

@card("Diregraf Ghoul")
def _():
    cv = Canvas(fill='0')
    cv.gradient_sky([(0,'0'),(0.3,'1'),(0.8,'r')])
    treeline(cv, 12, '1', seed=5, h=9)
    cv.rect(0, 26, 41, 33, '0')
    # the long rusty scythe, dragged on the diagonal
    cv.line(4, 31, 33, 9, '0'); cv.line(5, 31, 34, 9, 'k'); cv.line(5, 30, 34, 8, 'l')
    cv.poly([(33,9),(39,4),(41,9),(36,11)], 's'); cv.poly([(34,9),(39,6),(40,9)], 't')
    # hunched pale zombie, tattered wrappings, head lolling
    cv.stamp(11, 8, [
      "....00000...","...06n6n60..","..06n222n60.","..0n2 . 2n0.","..0nn222nn0.",
      "..0nnnnnn0..","...0m6mm0...","..06mm6mm60.",".06m6mm6m60.","06m6mmmm6m60",
      "06mmmmmmmm60","0m6mmmmmm6m0","0m mmmmmm m0",".0 mmmmmm 0.",".0 mmmmmm 0.",
      "..0mm..mm0..","..0m6..6m0..","..0m6..6m0..","..0k0..0k0..","..00....00..",
    ])
    cv.line(16, 18, 8, 27, 'm'); cv.line(17, 18, 9, 27, '6')  # arm to the haft
    return cv

@card("Unburial Rites")
def _():
    cv = Canvas(fill='1')
    cv.rect(0, 0, 41, 33, '2')
    for x in (7, 22, 37):                                   # cold crypt windows
        cv.poly([(x-4,20),(x-4,7),(x,2),(x+4,7),(x+4,20)], '3')
        cv.poly([(x-3,19),(x-3,7),(x,4),(x+3,7),(x+3,19)], 'e')
        cv.vline(x, 4, 19, '3'); cv.hline(12, x-3, x+3, '3')
    cv.rect(0, 20, 41, 33, '2')
    for y in range(21, 34, 4): cv.hline(y, 0, 41, '1')
    # the sarcophagus, its carved face lid sliding open
    cv.rect(5, 23, 36, 32, '3'); cv.rect(6, 24, 35, 31, '4')
    cv.rect(4, 21, 30, 25, '3'); cv.rect(5, 22, 29, 24, '5')
    cv.stamp(13, 21, ["0000000","04.4.40","0444440","04 4 40","0000000"])
    # the figure leaning in, in a lavender robe, one arm outstretched
    cv.stamp(24, 6, [
      "..00000..",".0666660.",".06pp660.",".0p2pp60.","..0pppp0.","..0pppp0.",
      ".0pqqqp0.","0pqqqqqp0","0pqqqqqp0","0pqqqqqq0","0pqqqqqq0",".0qqqqq0.",
      ".0qqqqq0.",".0qqqqq0.","..00000..",
    ])
    cv.line(24, 14, 16, 20, 'q'); cv.line(24, 15, 16, 21, 'p')
    return cv

@card("Bloodline Keeper")
def _():
    cv = Canvas(fill='1')
    cv.gradient_sky([(0,'1'),(0.4,'2'),(0.8,'2')])
    cv.disc(16, 13, 13, '2'); cv.disc(16, 13, 11, 'h')      # huge sick-green moon
    cv.disc(16, 13, 10, 'i'); cv.disc(14, 11, 2, 'h'); cv.disc(19, 16, 1.5, 'h')
    cv.poly([(32,33),(32,12),(36,4),(40,12),(40,33)], '0')  # clocktower
    cv.disc(36, 14, 2.4, '2'); cv.set(36, 14, '8')
    treeline(cv, 30, '0', seed=9, h=5)
    # the vampire, cloak flaring into two great wings of cloth
    cv.poly([(20,10),(6,14),(2,24),(12,22),(19,18)], '0')
    cv.poly([(22,10),(36,14),(40,24),(30,22),(23,18)], '0')
    cv.poly([(20,11),(9,15),(6,22),(13,21),(19,17)], '1')
    cv.poly([(22,11),(33,15),(36,22),(29,21),(23,17)], '1')
    cv.stamp(17, 4, [
      "..0000..",".0kkkk0.","0kooook0","0ko2 2o0","0koooo k",".0koook0.",
      "..0oo0..",".0k11k0.","0k1111k0","011111 0","01111110","01111110",
      ".0111110",".011111 ","..011110","..01111.","...0110.",
    ])
    for (bx, by) in ((4,6),(9,3),(28,5),(35,2),(24,2),(13,7)):   # bats
        cv.stamp(bx, by, ["0.0","000",".0."])
    return cv

@card("Grimgrin, Corpse-Born")
def _():
    cv = Canvas(fill='1')
    cv.gradient_sky([(0,'1'),(0.35,'2'),(0.75,'r')])
    treeline(cv, 10, '0', seed=13, h=10)
    cv.rect(0, 27, 41, 33, '0')
    # a hulking stitched brute: broad plates, sagging gut, chain-hook arm
    cv.stamp(9, 3, [
      ".....00000.....","....0ssssss0...","...0s666666s0..","..0s61 11 6s0..",
      "..0s6111116s0..","...0s611116s0..","....0s6666s0...","...0ss6666ss0..",
      "..0sst6666tss0.",".0sstt6666ttss0","0sst nnnnnn tss","0st nnnnnnnn ts",
      "0s nnnnnnnnnn s","0 nnn0nnnn0nnn ","0 nnnn0nn0nnnn ","0 nnnnn00nnnnn ",
      "0 nnnn0nn0nnnn ",".0 nnnnnnnnnn 0",".0 nnnnnnnnnn 0","..0 nnnnnnnn 0.",
      "..0 nnn00nnn 0.","..0 nn0..0nn 0.","..0k n0..0n k0.","...0 0....0 0..",
      "...00......00..",
    ])
    for i, y in enumerate(range(15, 31, 3)):                # the dragging chain
        cv.disc(6 - i//2, y, 1.4, 's'); cv.set(6 - i//2, y, '3')
    cv.set(15, 8, 'b'); cv.set(19, 8, 'b')                  # eye glints
    return cv

# ══ RED ══════════════════════════════════════════════════════════════
@card("Brimstone Volley")
def _():
    cv = Canvas(fill='0')
    cv.gradient_sky([(0,'1'),(0.3,'2'),(0.55,'a')])
    cv.poly([(0,33),(0,22),(9,16),(20,21),(30,15),(41,20),(41,33)], '0')
    cv.poly([(6,24),(14,20),(24,24),(34,19),(41,23),(41,28),(0,28)], 'a')
    cv.poly([(8,26),(18,23),(28,26),(38,22),(41,25),(41,29),(2,29)], 'b')
    cv.poly([(12,28),(22,26),(32,28),(41,26),(41,31),(6,31)], 'c')
    cv.noise(0, 26, 41, 33, 'd', 0.22, seed=5)
    cv.noise(0, 28, 41, 33, '8', 0.10, seed=8)
    for x in (5, 16, 27, 36):                          # burnt trees against the fire
        cv.vline(x, 18, 28, '0'); cv.line(x, 21, x-3, 17, '0'); cv.line(x, 23, x+3, 19, '0')
    # the volley: flaming rocks on a hard diagonal, brightest at the head
    for (sx, sy, L) in ((41,1,13),(35,0,16),(28,2,11),(38,8,9),(24,0,8),(31,7,7)):
        for i in range(L):
            x, y = sx-i, sy+i
            cv.set(x, y, 'd' if i > L-4 else ('c' if i > L-8 else 'a'))
            if i > L-4: cv.set(x, y+1, 'c'); cv.set(x+1, y, '8')
        cv.disc(sx-L, sy+L, 1.6, 'd'); cv.disc(sx-L, sy+L, 0.9, '8')
    return cv

@card("Devil's Play")
def _():
    cv = Canvas(fill='0')
    cv.gradient_sky([(0,'0'),(0.4,'1'),(0.72,'a')])
    cv.poly([(0,4),(41,14),(41,21),(0,11)], 'k')
    cv.poly([(0,5),(41,15),(41,18),(0,8)], 'l')
    cv.poly([(0,33),(6,27),(16,31),(26,26),(34,30),(41,27),(41,33)], 'a')
    cv.poly([(2,33),(9,29),(18,33),(27,28),(35,32),(41,29),(41,33)], 'b')
    cv.poly([(6,33),(13,31),(21,33),(30,30),(38,33),(41,31),(41,33)], 'c')
    cv.noise(0, 28, 41, 33, 'd', 0.28, seed=2)
    # the pour: tapers and brightens toward the fire, with a hot core
    for i in range(80):
        t = i/79.0
        x = 15 + 11*t + 2.5*math.sin(t*5); y = 11 + 19*t
        cv.disc(x, y, 2.4-1.1*t, 'a'); cv.disc(x, y, 1.5-0.6*t, 'c')
        cv.disc(x, y, 0.7-0.2*t, 'd')
    # the devil: crouched, asymmetric, arms that reach down to the pour
    cv.stamp(9, 0, [
      "0b0.....0b0","0bb0...0bb0",".0bbbbbbb0.","0bb8bbb8bb0","0bbbbbbbbb0",
      "0bb00b00bb0",".0bbbbbbb0.","0abbbbbbba0","0abbbbbbba0","0ab0bbb0ba0",
      ".0b0bbb0b0.","..0bbbbb0..","..0b0.0b0..",
    ])
    cv.limb(11, 8, 15, 12, 'b'); cv.hand(14, 12, 'b', 'a')      # arms onto the pour
    cv.limb(19, 8, 17, 12, 'b'); cv.hand(16, 12, 'b', 'a')
    cv.line(20, 7, 26, 4, 'b'); cv.line(26, 4, 29, 9, 'b')      # tail
    cv.line(20, 8, 26, 5, 'a')
    for y in range(1, 12): cv.set(8, y, 'c')                    # lit from below
    return cv

@card("Balefire Dragon")
def _():
    """Three attempts at a full flying dragon all read as a moth. At 42x34 a
    head-and-neck portrait crop carries far more menace than a whole body —
    so this one breaks the set's wide-shot habit on purpose."""
    cv = Canvas(fill='b')
    cv.gradient_sky([(0,'a'),(0.18,'b'),(0.55,'b'),(0.80,'a')])
    cv.disc(24, 14, 15, 'b')
    for x in (2, 9, 34, 40):
        cv.vline(x, 25, 33, '0'); cv.hline(28, x-2, x+2, '0')
    cv.rect(0, 31, 41, 33, '0')
    # skull and upper jaw, snout to the left, brow ridge heavy over the eye
    UPPER = {5:(24,34),6:(20,35),7:(16,36),8:(12,36),9:(9,35),
             10:(7,34),11:(5,32),12:(4,30),13:(5,27)}
    for y,(x0,x1) in UPPER.items():
        cv.hline(y, x0, x1, '0'); cv.hline(y, x0+1, x1-1, '1') if y in (7,9) else None
        cv.set(x0, y, 'a')
    for y,(x0,x1) in UPPER.items(): cv.hline(y, x0+2, x1-2, '0')
    # lower jaw, dropped open — the gap between the two is where the fire is
    LOWER = {17:(7,29),18:(9,30),19:(12,31),20:(16,32),21:(20,33)}
    for y,(x0,x1) in LOWER.items():
        cv.hline(y, x0, x1, '0'); cv.set(x0, y, 'a')
    for x in range(9, 26, 4):                                    # teeth, upper + lower
        cv.set(x, 14, '6'); cv.set(x+2, 16, '6')
    # swept-back horns
    for (hx, hy) in ((30,4),(34,3)):
        cv.limb(hx, hy, hx+7, hy-3, '0', 'a', w=2)
    cv.set(27, 9, 'd'); cv.set(28, 9, 'd'); cv.set(27, 8, '8')   # the eye
    cv.hline(7, 25, 31, '0')                                     # brow ridge over it
    # neck, receding down-right so the head is unmistakably the subject
    for i, (nx, ny) in enumerate([(33,14),(35,18),(37,23),(39,28)]):
        cv.limb(nx, ny, nx+2, ny+5, '0', 'a', w=4+i)
    # the balefire, pouring out of the open jaw to the left
    for i in range(16):
        t = i/15.0
        x = 8 - i*0.55; y = 15 + i*0.30
        cv.disc(x, y, 1.6+i*0.26, 'a'); cv.disc(x, y, 1.0+i*0.19, 'c')
        cv.disc(x, y, 0.5+i*0.11, 'd')
        if i > 9: cv.disc(x, y, 0.3+i*0.05, '8')
    return cv

@card("Instigator Gang")
def _():
    cv = Canvas(fill='9')
    cv.gradient_sky([(0,'8'),(0.32,'9'),(0.66,'l'),(0.88,'k')])
    cv.noise(0, 0, 41, 33, 'l', 0.06, seed=6)
    # two in the back, small and dark, so the leader reads first
    for (x, y) in ((0, 17), (31, 19)):
        cv.stamp(x, y, [
          "..0000..",".0knnk0.","0kn00nk0","0knnnnk0",".0knnk0.","..0kk0..",
          "0kkkkkk0","0kkkkkk0","0kkkkkk0",".0kkkk0.",".0kkkk0.",".0k00k0.",
        ])
    # the leader: head, jaw, neck, shoulders, torso — pixel by pixel
    cv.stamp(12, 8, [
      "...000000...","..0kkkkkk0..",".0knnnnnnk0.",".0nnnnnnnn0.",
      ".0n0nn0nn0n0",".0nnnnnnnn0.","..0nn00nn0..","..0nnnnnn0..",
      "...0nkkn0...","....0nn0....","..00llll00..",".0lllllllll0",
      "0llllllllll0","0lllllllllll","0llllllllll0","0llllllllll0",
      ".0lllllllll0",".0llllllll0.",".0llllllll0.",".0lllll.ll0.",
      ".0llll0.ll0.","..0ll0..0l0.",
    ])
    # arms up and out; the fists are 4px wide, half the head, so they read as hands
    FIST    = ["0nn0","onno","onno","0nn0"]
    for (sx, sy, fx, fy, up) in ((13,20,4,4,1),(25,20,34,3,1)):
        for k in range(4):
            cv.line(sx+k, sy, fx+k, fy+4, '0' if k in (0,3) else ('n' if k==1 else 'o'))
        cv.stamp(fx, fy, FIST)
    for (fx, fy) in ((1,13),(9,10),(29,12),(38,14)):     # the mob's fists behind
        cv.stamp(fx, fy, ["0kk0","knnk","knnk","0kk0"])
    return cv

@card("Blasphemous Act")
def _():
    cv = Canvas(fill='0')
    cv.rect(0, 0, 41, 33, '1')
    # a POINTED gothic arch: narrow apex, walls curving in, not a barn pentagon
    ARCH = [(2,20),(3,18),(4,16),(5,14),(6,12),(7,10),(8,8),(9,6),(10,5),(11,4)]
    for (dy, halfw) in enumerate([3,4,5,6,7,8,9,10,11,12,12,13,13,13]):
        y = 3 + dy
        cv.hline(y, 21-halfw, 21+halfw, '2')
    cv.rect(8, 17, 34, 33, '2')
    for (dy, halfw) in enumerate([2,3,4,5,6,7,8,9,10,11,11,12,12,12]):
        cv.hline(4+dy, 21-halfw, 21+halfw, 'a')
    cv.rect(9, 18, 33, 33, 'a')
    for (dy, halfw) in enumerate([1,2,3,4,5,6,7,8,9,10,10,11,11,11]):
        cv.hline(5+dy, 21-halfw, 21+halfw, 'c')
    cv.rect(10, 19, 32, 33, 'c')
    cv.noise(10, 8, 32, 33, 'd', 0.20, seed=4)
    cv.noise(11, 14, 31, 33, '8', 0.09, seed=7)
    for x in (3, 38):                                            # red banners
        cv.rect(x-2, 1, x+2, 21, 'a'); cv.rect(x-1, 1, x+1, 19, 'b')
        cv.poly([(x-2,21),(x,24),(x+2,21)], 'b')
    for y in range(24, 34, 3): cv.hline(y, 0, 41, '1')
    # the survivor: black silhouette against the fire, arms and a lowered sword
    cv.stamp(18, 12, ["00000","0mmm0","m0m0m","0mmm0",".0m0."])
    cv.hline(17, 15, 27, '0')
    cv.stamp(15, 17, ["0000000000 0","000000000 0.","00000000 0..",
                      "0000000 0...","000000 0....","00000 0.....",
                      "0000 0......","000.0.......","00..........",])
    cv.limb(17, 18, 13, 25, '0', '1'); cv.hand(11, 25, 'm', '1')
    cv.limb(25, 18, 29, 24, '0', '1'); cv.hand(29, 25, 'm', '1')
    cv.line(30, 26, 37, 31, 's'); cv.line(30, 27, 37, 32, '0')   # sword, point down
    for (bx, by) in ((3,28),(12,31),(27,30),(35,27)):            # bodies, not loaves
        cv.stamp(bx, by, ["00m00.","0mmmm0","0m00m0"])
    for y in range(12, 26): cv.set(14, y, 'c')                   # rim from the fire
    return cv

@card("Mayor of Avabruck")
def _():
    cv = Canvas(fill='0')
    cv.rect(0, 0, 41, 33, '1')
    cv.rect(0, 0, 14, 33, '2')                          # panelled study wall
    for y in range(2, 33, 6): cv.hline(y, 0, 13, '1')
    cv.rect(2, 6, 6, 11, 'k'); cv.rect(3, 7, 5, 10, '9')   # portrait
    cv.rect(30, 2, 41, 26, 'e'); cv.rect(31, 3, 40, 25, 'f')  # cold window
    for x in range(31, 41, 3): cv.vline(x, 3, 25, 'e')
    cv.rect(0, 27, 41, 33, 'k')
    cv.disc(8, 18, 5, '9'); cv.disc(8, 18, 3, '8')      # lamp glow on the desk
    cv.rect(0, 22, 20, 27, 'k'); cv.rect(0, 21, 20, 22, 'l')
    # the mayor, leaning on the desk, and the black hound at his knee
    cv.stamp(17, 5, [
      "...0000...","..0nnnn0..",".0n2nn2n0.",".0nnnnnn0.","..0nnnn0..",
      "..0hhhh0..",".0ehhhhe0.","0eehhhhee0","0ehhhhhhe0","0ehhhhhhe0",
      "0ehhhhhhh0",".0hhhhhh0.",".0hhhhhh0.",".0hhhhhh0.",".0hhh.hh0.",
      ".0hh0.0hh0",".0k0...0k0",
    ])
    cv.limb(17, 14, 12, 20, 'h', '0'); cv.hand(10, 21, 'n')   # a hand on the desk
    cv.stamp(21, 24, [                                   # the hound
      "0000.......","0k11k0.....","01 1 10....","00111000000","0111111111 0",
      "0111111111 0","0k1k...0k1k",
    ])
    return cv

@card("Kessig Cagebreakers")
def _():
    cv = Canvas(fill='0')
    cv.gradient_sky([(0,'1'),(0.4,'2'),(0.75,'2')])
    cv.rect(5, 2, 34, 31, '1'); cv.rect(6, 3, 33, 30, '0')
    cv.disc(20, 14, 9, '2'); cv.disc(20, 14, 7, '4')            # LIT so the wolf reads
    cv.stamp(14, 7, [
      "0h0.....0h0","0hh0...0hh0",".0hhhhhhh0.","0h5hhhhh5h0","0hh5hhh5hhh",
      "0hhhhhhhhhh",".0hh777hh0.","..0h777h0..","...0h7h0...",".0hh...hh0.",
    ])
    cv.set(17, 10, 'j'); cv.set(22, 10, 'j')
    for x in range(5, 35, 5): cv.vline(x, 2, 31, 's'); cv.vline(x+1, 2, 31, '3')
    cv.hline(2, 5, 34, 's'); cv.hline(31, 5, 34, 's')
    cv.rect(0, 31, 41, 33, '1')
    # the cagebreakers: actual heads under the hats, arms, hands on the bars
    for (x, flip) in ((0, 0), (35, 1)):
        cv.stamp(x, 10, ["0kkkkk0","kkkkkkk","0.....0"])        # tricorn
        cv.stamp(x+1, 13, ["00000","0nnn0","n0n0n","0nnn0",".0n0."])
        cv.hline(18, x, x+6, 'e')
        cv.stamp(x, 18, ["0eeeeee0","0eeeeee0","0eeeeee0","0eeeeee0",
                         ".0eeee0.",".0eeee0.",".0e00e0.",".00..00."])
        sx = x+6 if not flip else x
        cv.limb(sx, 19, 20 if not flip else 21, 22, 'e'); cv.hand(
            (18 if not flip else 22), 22, 'n')
    cv.disc(38, 3, 2.4, 'a'); cv.disc(38, 3, 1.4, 'd'); cv.set(38, 2, '8')
    return cv

@card("Garruk Relentless")
def _():
    cv = Canvas(fill='h')
    cv.gradient_sky([(0,'5'),(0.22,'4'),(0.5,'3'),(0.78,'h')])
    for (x, w, c) in ((1,2,'2'),(7,1,'3'),(13,2,'2'),(20,1,'3'),
                      (26,2,'2'),(32,1,'3'),(38,2,'2')):
        cv.rect(x, 0, x+w, 31, c)
        for b in range(3, 28, 8):
            cv.line(x, b, x-4 if x > 20 else x+4, b-4, c)
    cv.noise(0, 0, 41, 31, '4', 0.04, seed=3)
    cv.poly([(0,33),(0,29),(41,28),(41,33)], 'r')
    # The axe is genuinely geometric — haft as lines, head as a poly.
    cv.line(4, 32, 34, 4, '0'); cv.line(5, 32, 35, 4, 'k')
    cv.line(6, 32, 36, 4, 'l'); cv.line(7, 32, 37, 4, '0')
    cv.poly([(32,0),(41,2),(39,11),(31,7)], '0')
    cv.poly([(33,2),(39,4),(37,10),(32,7)], 't')
    cv.poly([(34,3),(37,5),(36,9),(33,7)], 's')
    # Garruk: head, beard, fur mantle, torso, two arms with hands on the haft.
    cv.stamp(12, 3, [
      "......000000......",".....0kllkkk0.....","....0klllkkkk0....",
      "....0kohnnnmk0....","....0oonnnnmm0....","....0o0nn0nmm0....",
      "....0onn00nmm0....","....0onnnnnmm0....",".....0onnnmm0.....",
      ".....0kllkkk0.....",".....0kllkkk0.....","......0kkkk0......",
      "...00kllkkkkk00...","..0klllkkkkkkkk0..",".0kllliihhhhhkkk0.",
      "0klllihhhhhhhhkkk0","0kllihhhhhhhhhhkk0","0kliihhhhhhhhhhhk0",
      "0klihhhhhhhhhhhhk0",".0lihhhhhhhhhhhk0.",".0lihhhhhhhhhhhk0.",
      "..0ihhh0000hhhk0..","..0ihh0llll0hhk0..","..0ihh0llll0hhk0..",
      "..0kll0....0lkk0..","..0kll0....0lkk0..","..0kll0....0lkk0..",
      "..000........000..",
    ])
    # arms: drawn as limbs off the shoulders, each ending in a real hand
    for (x0, y0, x1, y1) in ((16,20,12,25),(27,20,30,14)):
        cv.line(x0, y0, x1, y1, '0'); cv.line(x0+1, y0, x1+1, y1, 'h')
        cv.line(x0+2, y0, x1+2, y1, 'i'); cv.line(x0+3, y0, x1+3, y1, '0')
    for (hx, hy) in ((11,25),(29,13)):
        cv.stamp(hx, hy, ["0nn0","nono","nnnn","0nn0"])
    cv.set(17, 8, 'a'); cv.set(17, 9, 'a')          # the scar, one side only
    return cv

@card("Spider Spawning")
def _():
    cv = Canvas(fill='0')
    cv.gradient_sky([(0,'r'),(0.22,'h'),(0.45,'i'),(0.62,'j')])
    cv.disc(20, 15, 13, 'j'); cv.disc(20, 15, 9, '6')        # pale fog behind
    treeline(cv, 9, '0', seed=21, h=9)
    cv.poly([(0,34),(0,26),(19,21),(41,28),(41,34)], 'h')
    cv.poly([(0,34),(0,30),(18,26),(41,31),(41,34)], 'r')
    def spider(x, y, s):
        for dx in (-1, 1):
            cv.line(x, y, x+dx*s, y-s, '0')
            cv.line(x+dx*s, y-s, x+dx*s*2, y+1, '0')
            cv.line(x, y+1, x+dx*(s+1), y+s-1, '0')
            cv.line(x+dx*(s+1), y+s-1, x+dx*s*2, y+s, '0')
        cv.disc(x, y, s*0.5, '0'); cv.disc(x, y+1, s*0.65, '0')
        cv.set(x-1, y-1, 'j'); cv.set(x+1, y-1, 'j')
    for (x, y, s) in ((7,17,3),(18,13,4),(29,17,3),(38,14,3),(12,23,4),
                      (24,22,3),(34,24,4),(3,26,3),(19,29,3),(30,30,2),(9,31,2)):
        spider(x, y, s)
    return cv

@card("Gatstaf Shepherd")
def _():
    cv = Canvas(fill='4')
    # dusk, not daylight — the set is gothic and this was the one defector
    cv.gradient_sky([(0,'3'),(0.20,'4'),(0.36,'5'),(0.48,'9')])
    cv.hline(15, 0, 41, '8'); cv.hline(16, 0, 41, '9')
    cv.poly([(0,33),(0,18),(12,14),(26,17),(41,13),(41,33)], 'i')
    cv.poly([(0,33),(0,23),(14,19),(28,22),(41,18),(41,33)], 'j')
    cv.noise(0, 16, 41, 33, 'i', 0.10, seed=8)
    for x in (2, 37): cv.poly([(x-2,17),(x-1,8),(x+1,8),(x+2,17)], '4')   # standing stones
    # the flock, then the shepherd with his crook
    for (sx, sy) in ((22,25),(28,23),(33,26),(26,29),(36,22),(31,30),(19,28)):
        cv.stamp(sx, sy, ["06660","67776","0k0k0"])
    cv.line(13, 4, 11, 30, 'k'); cv.line(14, 4, 12, 30, 'l')
    cv.stamp(11, 1, ["00..","0ll0","0l.0",".00."])       # the crook's hook
    cv.stamp(11, 9, [
      "..0000..",".0nnnn0.","0n2nn2n0","0nnnnnn0",".0nnnn0.","..0kk0..",
      ".0hkkkh0","0hhkkkhh0".replace('9',''),"0hhkkkhh","0hhkkkhh",".0hkkkh0",
      ".0kkkkk0",".0kkkkk0",".0kk.kk0",".0k0.0k0",
    ])
    return cv

# ══ LANDS ════════════════════════════════════════════════════════════
@card("Plains")
def _():
    cv = Canvas(fill='2')
    cv.gradient_sky([(0,'2'),(0.2,'3'),(0.34,'4'),(0.44,'5')])
    cv.poly([(4,15),(14,11),(28,12),(38,9),(41,15),(0,15)], '8')   # the sky's one slit
    cv.poly([(8,15),(18,13),(30,14),(41,12),(41,15),(2,15)], '7')
    cv.poly([(0,10),(9,7),(20,9),(32,6),(41,8),(41,0),(0,0)], '3')
    cv.hline(16, 0, 41, '9')
    cv.poly([(0,33),(0,17),(41,16),(41,33)], 'h')                  # olive flats
    cv.poly([(0,33),(0,21),(41,19),(41,33)], 'i')
    cv.noise(0, 17, 41, 33, 'h', 0.14, seed=2)
    cv.noise(0, 24, 41, 33, 'r', 0.10, seed=6)
    for pts in ([(14,33),(17,27),(15,22),(19,17)],):               # a pale stream
        cv.stroke(pts, [3,2.2,1.6,1.0], '5'); cv.stroke(pts, [1.8,1.3,0.9,0.5], '6')
    for (x, y, h) in ((3,20,6),(35,19,5),(24,18,4)):               # bare trees
        cv.vline(x, y-h, y, '0')
        for d in (-2,-1,1,2): cv.line(x, y-h+1, x+d, y-h-1, '0')
    return cv

@card("Island")
def _():
    cv = Canvas(fill='1')
    cv.gradient_sky([(0,'4'),(0.12,'3'),(0.4,'2')])
    cv.poly([(0,0),(13,0),(15,12),(11,24),(13,33),(0,33)], '1')    # gorge walls
    cv.poly([(41,0),(28,0),(26,10),(30,22),(28,33),(41,33)], '1')
    cv.poly([(0,0),(9,0),(11,14),(7,26),(9,33),(0,33)], '2')
    cv.poly([(41,0),(32,0),(30,12),(34,24),(32,33),(41,33)], '2')
    cv.poly([(16,0),(25,0),(24,20),(17,20)], '5')                  # the falling water
    cv.poly([(17,0),(24,0),(23,18),(18,18)], '6')
    cv.poly([(19,0),(22,0),(22,14),(19,14)], '7')
    cv.noise(16, 4, 25, 20, '7', 0.16, seed=5)
    cv.disc(20, 22, 7, '5'); cv.disc(20, 23, 5, '6')               # the churn at its foot
    cv.noise(13, 19, 28, 27, '7', 0.22, seed=9)
    cv.poly([(0,33),(0,26),(41,27),(41,33)], 'e')
    cv.noise(0, 27, 41, 33, 'f', 0.12, seed=3)
    for (x, y, dx, dy) in ((7,25,9,3),(33,24,-8,4),(14,29,7,-2),(30,30,-6,-3)):
        cv.line(x, y, x+dx, y+dy, '0'); cv.line(x, y+1, x+dx, y+dy+1, '1')
    return cv

@card("Swamp")
def _():
    cv = Canvas(fill='1')
    cv.gradient_sky([(0,'1'),(0.25,'2'),(0.5,'3'),(0.66,'5')])
    cv.hline(19, 0, 41, '6'); cv.hline(20, 0, 41, '5')             # a low band of mist
    cv.poly([(0,33),(0,21),(41,20),(41,33)], 'r')
    cv.poly([(0,33),(0,25),(41,24),(41,33)], 'h')
    cv.noise(0, 21, 41, 33, 'r', 0.16, seed=4)
    # the broken fence, running away to the right
    for i, x in enumerate(range(2, 42, 6)):
        cv.vline(x, 22 - i//3, 27 - i//3, '0')
    cv.line(2, 23, 38, 21, '0'); cv.line(2, 25, 38, 23, '0')
    # the dead tree — the whole card's silhouette, set against the mist
    cv.vline(12, 6, 24, '0'); cv.vline(13, 6, 24, '0'); cv.vline(11, 12, 24, '0')
    for (x0,y0,x1,y1) in ((12,10,4,3),(13,9,21,2),(12,13,3,9),(13,12,22,7),
                          (12,7,8,1),(13,8,19,4),(12,16,5,13),(13,15,20,12)):
        cv.line(x0, y0, x1, y1, '0')
        cv.line(x1, y1, x1-2 if x1 < 12 else x1+2, y1-2, '0')
    cv.disc(12, 24, 3.5, '0')
    for (x, y, h) in ((30,22,7),(36,21,5)):                        # smaller dead trees
        cv.vline(x, y-h, y, '0'); cv.line(x, y-h+1, x-3, y-h-1, '0')
        cv.line(x, y-h+2, x+3, y-h, '0')
    return cv

@card("Mountain")
def _():
    cv = Canvas(fill='0')
    SKY = ['2','2','2','2','2','3','3','3','3','3','3','4','4','a','a','a',
           '9','9','9','8','8','8','8','8','8','8','8','8','8','8','8','8','8','8']
    LEFT  = [5,6,5,7,6,5,7,8,7,9,8,7,9,8,10,9,
             8,10,9,11,10,12,11,13,12,14,13,15,14,16,15,17,18,19]
    RIGHT = [4,5,4,6,5,7,6,5,7,6,8,7,6,8,7,9,
             8,7,9,8,10,9,11,10,12,11,13,12,14,13,15,16,17,18]
    for y in range(34):
        cv.hline(y, 0, 41, SKY[y])
        cv.hline(y, 0, LEFT[y]-1, '1')
        cv.hline(y, 0, LEFT[y]-3, '0')
        cv.hline(y, 42-RIGHT[y], 41, '1')
        cv.hline(y, 44-RIGHT[y], 41, '0')
    cv.disc(21, 19, 1.8, 'd'); cv.disc(21, 19, 1.0, '7')      # the small low sun
    for y in range(20, 34):                                    # the valley floor
        w = (y-19)
        cv.hline(y, 21-w, 21+w, '1' if y % 3 else '2')
    cv.noise(10, 22, 32, 33, '0', 0.22, seed=7)
    for (x, y) in ((17,27),(25,29),(21,31)):
        cv.vline(x, y-3, y, '0'); cv.line(x, y-2, x-2, y-4, '0')
    return cv

@card("Forest")
def _():
    cv = Canvas(fill='p')
    cv.gradient_sky([(0,'2'),(0.18,'p'),(0.45,'q'),(0.72,'p')])    # the violet ISD fog
    cv.disc(24, 14, 11, 'q'); cv.disc(24, 14, 7, '5')
    for (x, w, c) in ((3,2,'1'),(9,1,'2'),(15,2,'1'),(21,1,'2'),
                      (28,2,'1'),(34,1,'2'),(39,2,'1')):
        cv.rect(x, 0, x+w, 30, c)
        for b in range(2, 28, 7):
            cv.line(x, b, x-3 if x > 20 else x+3, b-3, c)
    cv.noise(0, 8, 41, 26, 'q', 0.05, seed=12)
    cv.poly([(0,33),(0,27),(41,26),(41,33)], '2')
    cv.poly([(0,33),(0,30),(41,29),(41,33)], '1')
    for i in range(60):                                            # the fallen branch
        t = i/59.0
        cv.set(int(2+38*t), int(31-6*math.sin(t*3.0)), '0')
        cv.set(int(2+38*t), int(32-6*math.sin(t*3.0)), '1')
    return cv

@card("Kessig Wolf Run")
def _():
    cv = Canvas(fill='1')
    cv.gradient_sky([(0,'1'),(0.2,'2'),(0.42,'3'),(0.55,'4')])
    moon(cv, 8, 6, 4, disc='6')
    cv.stars(14, '4', seed=6, maxy=14)
    # the ridge, and the wolf standing on its crest — the whole card
    cv.poly([(0,33),(0,22),(10,18),(22,13),(32,17),(41,15),(41,33)], '0')
    cv.poly([(0,33),(0,26),(12,22),(24,18),(34,21),(41,20),(41,33)], '1')
    cv.stamp(18, 3, [
      "0h0.....0h0","0hh0...0hh0",".0hhhhhhh0.","0hhhhhhhhhh","0h5hhhhh5h0",
      "0hhhhhhhhhh",".0hhhhhhh0.","..0hhhhh0..","...0hhh0...","..0hhhhh0..",
      ".0hh0.0hh0.",
    ])
    for y in range(3, 14):                                         # black it out
        for x in range(18, 29):
            if cv.get(x, y) in ('h','5'): cv.set(x, y, '0')
    cv.set(20, 7, '8'); cv.set(25, 7, '8')                         # its eyes
    cv.poly([(0,33),(8,29),(18,31),(28,28),(41,30),(41,33)], 'e')  # the river below
    cv.noise(0, 29, 41, 33, 'f', 0.10, seed=4)
    for x in (4, 34): cv.vline(x, 17, 27, '0')
    return cv

# ══ ARTIFACT ═════════════════════════════════════════════════════════
@card("Blazing Torch")
def _():
    cv = Canvas(fill='1')
    # dusk gradient rather than three flat bands
    cv.gradient_sky([(0,'2'),(0.14,'3'),(0.26,'a'),(0.36,'9'),(0.46,'8'),(0.56,'9')])
    cv.noise(0, 6, 41, 20, 'a', 0.06, seed=3)
    cv.poly([(0,23),(12,20),(24,22),(34,19),(41,21),(41,27),(0,27)], '3')
    cv.poly([(0,33),(0,26),(41,25),(41,33)], '1')
    for (x, w) in ((2,4),(8,3),(30,4),(37,3)):
        cv.rect(x, 23, x+w, 27, '0'); cv.poly([(x-1,23),(x+w//2,21),(x+w+1,23)], '0')
        cv.set(x+1, 25, '8')
    # the bearer, in silhouette, with an arm that actually holds the torch
    cv.stamp(22, 9, ["00000","01110","01110","01110",".010."])
    cv.hline(14, 19, 30, '1')
    cv.stamp(19, 14, ["0111111111 0","01111111110.","011111111 0.",
                      "0111111110..","01111111 0..","011111110...",
                      "01111111 0..","011111110...","0111.1110...",
                      "011.0.110...","01..0.010...",])
    cv.limb(20, 15, 15, 12, '1'); cv.hand(13, 12, '1', '0')     # arm up to the shaft
    cv.line(14, 13, 15, 23, 'k'); cv.line(15, 13, 16, 23, 'l')  # torch shaft
    cv.disc(14, 8, 5.6, 'a'); cv.disc(14, 8, 4.0, 'c')
    cv.disc(14, 7, 2.6, 'd'); cv.disc(14, 6, 1.2, '8')
    for (fx, fy, r) in ((11,3,1.4),(17,2,1.0),(14,0,0.9)):
        cv.disc(fx, fy, r, 'c'); cv.disc(fx, fy, r*0.5, 'd')
    cv.line(31, 6, 29, 30, 'k')                                  # the spear
    for y in range(14, 26): cv.set(19, y, 'c')                   # rim from his flame
    return cv


# ══ SET GRAMMAR ══════════════════════════════════════════════════════
# Three rules applied across all 32 after each illustration is drawn, so the
# set reads as one art object rather than 32 competent pictures.
#
#   1. APERTURE — every card is seen through something. Two of four edges are
#      bitten by a ragged occluder belonging to that card's own world. Never
#      symmetric, never the same depth twice.
#   2. CANDLES  — a votive flame is the warmest thing on almost every card,
#      and the count means something: 1 = a lone soul, 2 = a duel or bargain,
#      many = mass death, ZERO = the werewolf/moon cards, where the moon
#      takes the candles' place.
#   3. CONTACT  — any figure standing on ground gets a shadow under its feet.
#
# The four client-approved cards are exempt: they already do all of this.
APPROVED = {"Brimstone Volley", "Spider Spawning", "Mountain", "Forest"}

FINISH = {
 # name: (aperture sides, depth, seed, colour, [candles], contact or None)
 "Geist of Saint Traft":  (('L','R'), 3,11, '0',  [], None),
 "Doomed Traveler":       (('B','R'), 3,3, '0',  [(3,26,False)], (8,29,9)),
 "Chapel Geist":          (('T','L'), 4,5, '0',  [(35,24,True),(3,27,False)], None),
 "Elite Inquisitor":      (('L','B'), 4,7, '0',  [(36,20,True)], (16,31,13)),
 "Midnight Haunting":     (('R','B'), 4,9, '0',  [(3,7,False),(6,29,True)], None),
 "Snapcaster Mage":       (('R','B'), 4,13, '0',  [(4,8,False),(38,17,False)], (17,30,13)),
 "Delver of Secrets":     (('T','R'), 3,17, '0',  [(11,29,True),(24,28,False)], (13,30,11)),
 "Invisible Stalker":     (('L','T'), 3,19, '0',  [(37,28,False)], None),
 "Laboratory Maniac":     (('L','B'), 4,23, '0',  [(35,6,False),(38,25,False)], (19,30,11)),
 "Stitched Drake":        (('T','L'), 4,29, '0',  [(5,30,True)], None),
 "Liliana of the Veil":   (('T','B'), 3,31, '0',  [], (16,31,13)),
 "Diregraf Ghoul":        (('R','T'), 4,37, '0',  [(4,28,True)], (12,29,11)),
 "Unburial Rites":        (('L','T'), 4,41, '0',  [(37,26,True),(2,24,False)], None),
 "Bloodline Keeper":      (('L','R'), 4,43, '0',  [], None),
 "Grimgrin, Corpse-Born": (('T','R'), 4,47, '0',  [(3,27,True)], (14,31,15)),
 "Devil's Play":          (('L','T'), 3,53, '0',  [], None),
 "Balefire Dragon":       (('B','L'), 3,59, '0',  [], None),
 "Instigator Gang":       (('L','R'), 3,61, 'k',  [(20,3,False)], (14,31,15)),
 "Blasphemous Act":       (('L','R'), 4,67, '0',  [(6,12,True),(35,12,True),(4,22,False),(37,22,False)], None),
 "Mayor of Avabruck":     (('T','R'), 3,71, '0',  [], (18,29,11)),
 "Kessig Cagebreakers":   (('T','B'), 3,73, '0',  [], None),
 "Garruk Relentless":     (('L','B'), 4,79, 'r',  [], (13,31,15)),
 "Gatstaf Shepherd":      (('T','L'), 3,83, '0',  [], (12,29,9)),
 "Plains":                (('L','T'), 4,89, '0',  [(33,22,False)], None),
 "Island":                (('T','R'), 4,97, '0',  [], None),
 "Swamp":                 (('R','T'), 4,101, '0',  [(6,27,True)], None),
 "Kessig Wolf Run":       (('L','B'), 4,103, '0',  [], None),
 "Blazing Torch":         (('R','T'), 3,107, '0',  [], (24,30,11)),
}

def finish(name, cv):
    """Apply the set grammar. Approved cards pass through untouched."""
    if name in APPROVED or name not in FINISH:
        return cv
    sides, depth, seed, col, candles, contact = FINISH[name]
    cv.aperture(sides, depth, seed, col)
    if contact:
        cv.contact(*contact)
    for (x, y, big) in candles:
        cv.candle(x, y, big)
    return cv
