# Open questions raised by the per-card audit

Things found during `/check-card-procedure` that are real but that I did not fix
on the spot, with the reasoning. Each names the cards it touches so it can be
revisited when the audit reaches them.

## A werewolf's upkeep trigger re-checks the condition of the wrong face

**Cards**: every Werewolf DFC — Mayor of Avabruck, Gatstaf Shepherd, Reckless
Waif, Villagers of Estwald, Grizzled Outcasts, Tormented Pariah, Hanweir
Watchkeep, Ulvenwald Mystics, Village Ironsmith, Daybreak Ranger, Kruin Outlaw,
Instigator Gang — all through `helpers::werewolf_should_transform`.

**Rule**: CR 603.4. An intervening-if condition is checked when the ability
triggers *and* again when it resolves — but it is the condition of the ability
that triggered, which belongs to one face.

**What the code does**: `werewolf_should_transform` branches on the object's
*current* `is_transformed` to decide which condition to test:

```rust
if state.get_object(object_id).is_some_and(|o| o.is_transformed) {
    state.num_spells_cast_last_turn.values().any(|&count| count >= 2)
} else {
    state.num_spells_cast_last_turn.values().sum::<u32>() == 0
}
```

At trigger time that is right, because the face that triggered is the current
one. At resolution it is right only if the permanent has not transformed in the
meantime.

**How it is reachable**: Moonmist ({1}{G} instant, "Transform all Humans") cast
in response to a front-face Werewolf's upkeep trigger. The trigger is the front
face's — "if no spells were cast last turn, transform this creature" — and that
condition is about *last* turn, so casting Moonmist does not falsify it. By the
rules the ability resolves and transforms the permanent, flipping the
now-Howlpack Alpha back to Mayor. The code instead re-reads the current face,
tests the back face's condition ("a player cast two or more spells last turn"),
finds it false, and does nothing.

**Why it is not fixed here**: the fix is a mechanism, not a card change — a
trigger has to carry the face it fired from. `TriggerSource` snapshots id,
card_id, controller, description and targets, but not `is_transformed`, and the
resolution hooks (`on_upkeep(&self, state, self_id, targets, registry)`) have no
handle on the trigger at all. Adding the snapshot means threading it through
every `emit` site and giving the hooks a way to read it.

That is worth doing once, not twelve times, and it wants deciding after the
other Werewolves have been read — several of them may need the same snapshot for
their own reasons, which would settle the shape. Revisit at Ulvenwald Mystics
(the next Werewolf on the list) at the latest.
