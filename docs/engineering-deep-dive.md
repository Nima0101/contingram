# The policy and obstruction recurrence

Let `S` be finite worlds, `I` the nonempty initial set, `Safe` and `Goal`
subsets, and `T(s,a)` the set of `(successor, observation)` outcomes for action
`a` in world `s`. Empty `T(s,a)` means the action is disabled in that world.

For a nonempty belief `B`, define:

```text
post(B,a,o) = { s' | some s in B admits (s',o) in T(s,a) }

W(B,0) = (B subset Safe) and (B subset Goal)

W(B,h) = (B subset Safe) and (
    (B subset Goal)
    or exists action a:
       T(s,a) is nonempty for every s in B
       and every successor is safe
       and W(post(B,a,o), h-1) for every possible observation o
)
```

The quantifier order is the important part: choose one action before knowing
the hidden world, then cover every observation and every matching successor.
Dropping a world where that action is disabled or selecting one favorable
outcome would change the problem.

For the receipt example, the initial belief is `{absent, done}`. Sending again
could create a duplicate in `done`. Final status partitions the belief into
singleton sets: `committed` permits termination, while `absent-final` permits
one send. A stale status that emits the same labels in both worlds does not
reduce the belief, even if a human-readable label sounds reassuring.

## Evidence is an induction on horizon

A positive node is either a universally safe goal or a chosen action with one
positive child for each possible observation. A negative base node witnesses
an unsafe belief or a safe non-goal belief at horizon zero. A negative internal
node gives an obstruction for every action: disabled in some world, an unsafe
possible successor, or one losing observation child.

The checker derives beliefs from `I` and applies those rules recursively with
strictly decreasing horizon. References point backward in the flat node array;
the root is last and all nodes must be reachable. Shared nodes are checked
with their belief, remaining budget, and expected polarity, preventing a proof
for one context from being reused silently in another.

This induction explains the finite guarantee. It is not a machine-checked
theorem about Rust, the parser, or a downstream API. See the
[threat model](threat-model.md).

## UNKNOWN is a distinct result

Search reserves memo entries and charges work before taking the corresponding
steps. Running out of budget never becomes a losing memo entry. Evidence
extraction, automatic checking, and serialization also have limits. If a
complete result cannot be produced and checked, analysis has no theorem to
export and returns `unknown` with no certificate.

An unknown remote effect is different: it is represented by multiple worlds
in an otherwise valid finite model. Computational `unknown` describes the
analyzer's unfinished work.

## Model changes and hashes

The model hash includes the normalized transition relation and its semantic
version. A changed contract invalidates an old binding. To test whether a
manually specified decision table still works under a changed interface, explicitly
rebind that same table and run the checker. A successful hash update alone
establishes nothing; the checker must reconstruct all branches again.
