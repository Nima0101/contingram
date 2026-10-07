"""Original held-out fixture authoring; deliberately contains no policy search."""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).parent
TRUE = {"op": "true"}


def eq(var, value):
    return {"op": "eq", "var": var, "value": value}


def all_of(*args):
    return {"op": "all", "args": list(args)}


def any_of(*args):
    return {"op": "any", "args": list(args)}


def neg(arg):
    return {"op": "not", "arg": arg}


def outcome(name, when, observation, **assignments):
    return {"id": name, "when": when,
            "set": [{"var": k, "value": v} for k, v in assignments.items()],
            "observe": observation}


def action(name, *outcomes):
    return {"id": name, "enabled": TRUE, "outcomes": list(outcomes)}


def contract(name, variables, initial, safe, goal, actions):
    return {"schema_version": 1, "name": name,
            "variables": [{"name": k, "values": v} for k, v in variables.items()],
            "initial": initial, "safe": safe, "goal": goal, "actions": actions}


fixtures = []
expectations = []


def save(model, first_winning_horizon, rationale):
    fixtures.append(model)
    expectations.append({
        "file": model["name"] + ".json",
        "first_winning_horizon": first_winning_horizon,
        "expected": [{"horizon": h,
                      "status": "policy_found" if first_winning_horizon is not None
                      and h >= first_winning_horizon else "no_policy_within_horizon"}
                     for h in [0, 1, 2, 3, 4, 8]],
        "rationale": rationale,
    })


# 1. A cached receipt belongs to an old request; refreshing may never replace it.
effect = ["absent", "done", "duplicate"]
send = action("send",
    outcome("first", eq("effect", "absent"), "ack", effect="done"),
    outcome("repeat", neg(eq("effect", "absent")), "ack", effect="duplicate"))
status = action("status",
    outcome("old-receipt", eq("receipt", "old"), "receipt-other"),
    *[outcome("current-" + value,
              all_of(eq("receipt", "current"), eq("effect", value)), value)
      for value in effect])
for sticky in [True, False]:
    refresh = action("refresh",
        outcome("fresh", TRUE, "refresh-ack", receipt="current"))
    if sticky:
        refresh["outcomes"].append(outcome("stale", TRUE, "refresh-ack"))
    model = contract("stale-receipt-" + ("sticky" if sticky else "refresh"),
        {"effect": effect, "receipt": ["old", "current"]},
        all_of(eq("receipt", "old"), any_of(eq("effect", "absent"), eq("effect", "done"))),
        neg(eq("effect", "duplicate")), eq("effect", "done"),
        [copy.deepcopy(send), copy.deepcopy(status), refresh])
    save(model, None if sticky else 3,
         "Old receipts never distinguish absent from done; an adversary may keep refresh stale forever."
         if sticky else "Refresh then status then conditional send; absent requires three calls. No shorter informative path exists.")

# 2. Cancel acknowledgement is not quiescence; status can be negative while pending.
phases = ["absent", "pending", "done", "duplicate"]
cancel = action("cancel", *[
    outcome("ack-" + p, eq("phase", p), "cancel-ack") for p in phases],
    outcome("late-commit", eq("phase", "pending"), "cancel-ack", phase="done"))
read = action("status",
    outcome("none", eq("phase", "absent"), "absent"),
    outcome("pending-hidden", eq("phase", "pending"), "absent"),
    outcome("late-hidden", eq("phase", "pending"), "absent", phase="done"),
    outcome("done", eq("phase", "done"), "committed"),
    outcome("bad", eq("phase", "duplicate"), "duplicate"))
retry = action("retry",
    outcome("first", eq("phase", "absent"), "ack", phase="done"),
    outcome("unsafe", neg(eq("phase", "absent")), "ack", phase="duplicate"))
fence = action("fence",
    outcome("absent", eq("phase", "absent"), "absent-final"),
    outcome("quiesced", eq("phase", "pending"), "absent-final", phase="absent"),
    outcome("settled", eq("phase", "pending"), "committed", phase="done"),
    outcome("done", eq("phase", "done"), "committed"),
    outcome("bad", eq("phase", "duplicate"), "duplicate"))
for with_fence in [False, True]:
    model = contract("cancel-" + ("fenced" if with_fence else "ack-only"),
        {"phase": phases}, neg(eq("phase", "duplicate")),
        neg(eq("phase", "duplicate")), eq("phase", "done"),
        copy.deepcopy([cancel, read, retry] + ([fence] if with_fence else [])))
    save(model, 2 if with_fence else None,
         "Fence settles or cancels all pending work and distinguishes committed from final absence; retry only final absence."
         if with_fence else "Pending work may survive every cancel/status sequence; retry is unsafe, and the goal is not universal.")

# 3. A coarse object-exists response hides whether a notification already happened.
reservation = eq("reservation", "held")
sent = eq("notification", "sent")
reserve = action("reserve",
    outcome("first", neg(reservation), "reserved", reservation="held"),
    outcome("duplicate", reservation, "reserved", violation="yes"))
notify = action("notify",
    outcome("first", all_of(reservation, neg(sent)), "notified", notification="sent"),
    outcome("orphan", all_of(neg(reservation), neg(sent)), "notified", violation="yes"),
    outcome("duplicate", sent, "notified", violation="yes"))
for precise in [False, True]:
    inspect = action("inspect",
        outcome("empty", all_of(neg(reservation), neg(sent)), "empty"),
        outcome("partial", all_of(reservation, neg(sent)), "reserved" if precise else "record-exists"),
        outcome("complete", all_of(reservation, sent), "complete" if precise else "record-exists"),
        outcome("orphan", all_of(neg(reservation), sent), "orphan"))
    model = contract("partial-" + ("stage-status" if precise else "object-status"),
        {"reservation": ["none", "held"], "notification": ["none", "sent"],
         "violation": ["no", "yes"]},
        all_of(eq("violation", "no"), any_of(reservation, neg(sent))),
        eq("violation", "no"), all_of(reservation, sent),
        copy.deepcopy([reserve, notify, inspect]))
    save(model, 3 if precise else None,
         "Inspect; empty -> reserve then notify; reserved -> notify; complete -> stop. Empty branch requires three calls."
         if precise else "Record-exists merges reserved with complete. Notify duplicates one world; reserve duplicates both; inspect cannot refine.")

# 4. Identity retention can expire while a request is retried, without clock access.
count = ["zero", "one", "duplicate"]
for variant in ["retained", "expired-blind", "expired-ledger"]:
    resend = action("resend",
        outcome("first", eq("count", "zero"), "ack", count="one"),
        outcome("dedup", all_of(eq("count", "one"), eq("key", "retained")), "ack"),
        outcome("expired", all_of(eq("count", "one"), eq("key", "expired")), "ack", count="duplicate"),
        outcome("bad", eq("count", "duplicate"), "ack"))
    if variant != "retained":
        resend["outcomes"].append(outcome("expiry-during-retry",
            all_of(eq("count", "one"), eq("key", "retained")),
            "ack", key="expired", count="duplicate"))
    actions = [resend]
    if variant == "expired-ledger":
        actions.append(action("ledger", *[
            outcome("read-" + value, eq("count", value), value, key="expired")
            for value in count]))
    model = contract("identity-" + variant,
        {"count": count, "key": ["retained", "expired"]},
        all_of(eq("key", "retained"), neg(eq("count", "duplicate"))),
        neg(eq("count", "duplicate")), eq("count", "one"), actions)
    first = 1 if variant == "retained" else 2 if variant == "expired-ledger" else None
    save(model, first,
         {"retained": "One resend safely creates or deduplicates the operation; no sensing is necessary.",
          "expired-blind": "Retry may lose its retained identity before deduplication; one possible committed world becomes duplicate.",
          "expired-ledger": "Authoritative operation ledger survives dedup expiry. Read zero -> resend; one -> stop. Ledger never races a pending effect in this model."}[variant])

for model in fixtures:
    (ROOT / (model["name"] + ".json")).write_text(json.dumps(model, indent=2) + "\n")
(ROOT / "expectations.json").write_text(json.dumps({
    "schema_version": 1,
    "basis": "Independent manual reasoning before inspecting or running solver code.",
    "cases": expectations,
}, indent=2) + "\n")
print(f"Wrote {len(fixtures)} contracts and expected verdicts; no solver invoked.")
