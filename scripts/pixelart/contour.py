"""Measure whether a sprite's outline is drawn or merely rasterized.

Blobbiness is measurable. A hand-drawn contour that advances +/-1 px every
row or two, with irregular run lengths and no jumps of 2 or more, IS the
rasterization of an ellipse — that is what a circle looks like when a
scanline algorithm draws it. Crisp pixel art instead uses long flat runs,
diagonals whose run lengths are CONSISTENT (4,4,4,4 or 2,2,2,2), and
deliberate >=2 steps at corners.
"""
import statistics

def edge_runs(rows, side='L'):
    xs = []
    for r in rows:
        ix = [i for i, c in enumerate(r) if c not in '.']
        if ix: xs.append(ix[0] if side == 'L' else ix[-1])
    runs, cur = [], 1
    for i in range(1, len(xs)):
        if xs[i] == xs[i-1]: cur += 1
        else: runs.append(cur); cur = 1
    runs.append(cur)
    steps = [xs[i+1]-xs[i] for i in range(len(xs)-1)]
    return xs, runs, steps

def segments(xs):
    """Split the contour where its direction changes. A shape is a sequence
    of segments; regularity has to be judged inside a segment, not across
    the whole edge — a global stdev punishes exactly the mix of long flats
    and short diagonal steps that a well-drawn shape needs."""
    segs, cur, dirn = [], [xs[0]], 0
    for i in range(1, len(xs)):
        d = (xs[i] > xs[i-1]) - (xs[i] < xs[i-1])
        if d != dirn and d != 0 and dirn != 0:
            segs.append(cur); cur = [xs[i-1]]
        if d: dirn = d
        cur.append(xs[i])
    segs.append(cur)
    return segs

def score(rows, name=''):
    out = {}
    for side in 'LR':
        xs, runs, steps = edge_runs(rows, side)
        nz = [abs(s) for s in steps if s]
        sds = []
        for seg in segments(xs):
            r, cur = [], 1
            for i in range(1, len(seg)):
                if seg[i] == seg[i-1]: cur += 1
                else: r.append(cur); cur = 1
            r.append(cur)
            if len(r) > 2: sds.append(statistics.pstdev(r))
        out[side] = dict(
            maxrun=max(runs),
            seg_sd=(sum(sds)/len(sds)) if sds else 0.0,
            corners=sum(1 for s in nz if s >= 2) / max(1, len(steps)),
        )
    return out

def report(rows, name):
    s = score(rows)
    ok = (min(s['L']['maxrun'], s['R']['maxrun']) >= 5
          and max(s['L']['seg_sd'], s['R']['seg_sd']) <= 0.85
          and min(s['L']['corners'], s['R']['corners']) >= 0.12)
    print("%-18s L(flat%-2d seg-sd%.2f corner%2.0f%%)  R(flat%-2d seg-sd%.2f corner%2.0f%%)  %s" % (
        name, s['L']['maxrun'], s['L']['seg_sd'], s['L']['corners']*100,
        s['R']['maxrun'], s['R']['seg_sd'], s['R']['corners']*100,
        "CRISP" if ok else "blobby"))
    return ok
