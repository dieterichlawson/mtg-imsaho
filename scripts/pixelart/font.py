"""A 3x5 pixel font, plus a 'greeked' scribble renderer.

Two title-bar treatments, per the design fork: real names in a tiny
readable face, or illegible scribbles that convey only rhythm and length.
"""
F = {
'A':["010","101","111","101","101"], 'B':["110","101","110","101","110"],
'C':["011","100","100","100","011"], 'D':["110","101","101","101","110"],
'E':["111","100","110","100","111"], 'F':["111","100","110","100","100"],
'G':["011","100","101","101","011"], 'H':["101","101","111","101","101"],
'I':["111","010","010","010","111"], 'J':["001","001","001","101","010"],
'K':["101","101","110","101","101"], 'L':["100","100","100","100","111"],
'M':["101","111","111","101","101"], 'N':["101","111","111","111","101"],
'O':["010","101","101","101","010"], 'P':["110","101","110","100","100"],
'Q':["010","101","101","110","011"], 'R':["110","101","110","101","101"],
'S':["011","100","010","001","110"], 'T':["111","010","010","010","010"],
'U':["101","101","101","101","011"], 'V':["101","101","101","010","010"],
'W':["101","101","111","111","101"], 'X':["101","101","010","101","101"],
'Y':["101","101","010","010","010"], 'Z':["111","001","010","100","111"],
'0':["111","101","101","101","111"], '1':["010","110","010","010","111"],
'2':["110","001","010","100","111"], '3':["110","001","010","001","110"],
'4':["101","101","111","001","001"], '5':["111","100","110","001","110"],
'6':["011","100","110","101","010"], '7':["111","001","010","010","010"],
'8':["010","101","010","101","010"], '9':["010","101","011","001","110"],
"'":["010","010","000","000","000"], '-':["000","000","111","000","000"],
',':["000","000","000","010","100"], '.':["000","000","000","000","010"],
' ':["000","000","000","000","000"],
}
def text_w(s):  return max(0, len(s)*4 - 1)
def draw_text(cv, x, y, s, c):
    for ch in s.upper():
        g = F.get(ch)
        if g:
            for dy,row in enumerate(g):
                for dx,b in enumerate(row):
                    if b == '1': cv.set(x+dx, y+dy, c)
        x += 4
def draw_greek(cv, x, y, w, s, c, seed=0, h=5):
    """Illegible word-shapes with the rhythm of the real string.

    Seeded with a *stable* hash of the text. Python's built-in hash() is
    randomised per process (PYTHONHASHSEED), so seeding from it gave every
    card different greeked handwriting on every render — the same card was
    never twice the same, and diffing two renders showed 50k changed pixels
    with no source change. A card's scribble is part of its identity and
    must be reproducible.
    """
    import random, zlib
    rng = random.Random(zlib.crc32(s.encode()) & 0xffff if seed == 0 else seed)
    cx = x
    for word in s.upper().split():
        wl = max(2, min(len(word)*3, w - (cx - x)))
        if cx - x + wl > w: break
        for i in range(wl):
            asc = rng.random()
            if asc < 0.18:                      # ascender
                cv.vline(cx+i, y, y+h-2, c)
            elif asc < 0.30:                    # descender
                cv.vline(cx+i, y+2, y+h-1, c)
            else:
                cv.vline(cx+i, y+2, y+h-2, c)
        cx += wl + 2
        if cx - x >= w: break
