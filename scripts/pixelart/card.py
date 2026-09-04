"""Card frame compositor: a gothic pixel frame around a 42x34 art grid."""
from engine import Canvas, PAL, AW, AH
import font

CW, CH = 48, 56
ART_X, ART_Y = 3, 11

# (keyline, rail, plate, plate_text, accent) — darker + richer than v1 so the
# art carries the card instead of competing with a pale lavender border.
FRAMES = {
 'W': ('0','5','2','5','8'),
 'U': ('0','e','1','e','g'),
 'B': ('0','2','1','3','b'),
 'R': ('0','a','1','a','c'),
 'G': ('0','h','1','h','8'),
 'M': ('0','9','1','9','7'),
 'A': ('0','3','1','3','t'),
 'L': ('0','k','1','k','6'),
}
PIPS = {'W':'7','U':'f','B':'p','R':'b','G':'i','C':'5','X':'4'}

def make_card(art_rows, color='B', name='', typeline='', cost='', greek=True):
    key, rail, plate, ptxt, accent = FRAMES[color]
    cv = Canvas(CW, CH, fill=rail)
    cv.rect(0, 0, CW-1, CH-1, key)
    cv.rect(1, 1, CW-2, CH-2, rail)
    for (x,y) in ((1,1),(CW-2,1),(1,CH-2),(CW-2,CH-2)): cv.set(x,y,key)

    cv.rect(2, 2, CW-3, 9, plate)                       # title plate
    cv.hline(2, 2, CW-3, key); cv.hline(9, 2, CW-3, key)
    n = name.upper()
    if greek:
        font.draw_greek(cv, 4, 4, 27, n or 'CARD', ptxt)
    else:
        while font.text_w(n) > 27 and ' ' in n: n = n.rsplit(' ', 1)[0]
        font.draw_text(cv, 4, 4, n[:7], ptxt)
    px = CW - 5                                          # mana pips
    for sym in reversed(cost):
        cv.disc(px, 5, 1.7, PIPS.get(sym,'4'), edge=key); px -= 4

    cv.rect(ART_X-1, ART_Y-1, ART_X+AW, ART_Y+AH, key)   # art window
    for dy, row in enumerate(art_rows):
        for dx, ch in enumerate(row):
            if ch != '.': cv.set(ART_X+dx, ART_Y+dy, ch)

    ty = ART_Y+AH+1                                      # type / text plate
    cv.rect(2, ty, CW-3, ty+6, plate)
    cv.hline(ty, 2, CW-3, key); cv.hline(ty+6, 2, CW-3, key)
    if greek:
        font.draw_greek(cv, 4, ty+2, 29, typeline or 'TYPE', ptxt, h=4)
    else:
        t = typeline.upper().split('—')[0].strip()
        font.draw_text(cv, 4, ty+2, t[:7], ptxt)
    cv.set(CW-5, ty+3, accent)
    return cv

def to_image(cv, scale=1, bg=(0x0d,0x0b,0x14)):
    from PIL import Image
    im = Image.new('RGB', (cv.w, cv.h), bg); p = im.load()
    for y in range(cv.h):
        for x in range(cv.w):
            c = PAL.get(cv.px[y][x])
            if c: p[x,y] = c
    return im.resize((cv.w*scale, cv.h*scale), Image.NEAREST) if scale>1 else im
