"""Which result rows are a seat's answer.

`replay.py` and `draft-replay.py` write a row for every call. A row with no
`action` is a call that failed; a row with `"fallback": true` carries an
action the harness chose in place of one it could not get (the pack's
first card for a pick whose answer could not be used, a game answer that
was malformed or refused). Neither is the seat's answer: counted as one, a
substitution is the model disagreeing with itself, a pick a guide left
alone, or a guide's choice sent to the judge (#1010). Every reader asks
this one question.
"""


def answered(row):
    """The seat answered this call itself."""
    return row.get("action") is not None and not row.get("fallback")
