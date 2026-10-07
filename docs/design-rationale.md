# Design rationale

## Why an explicit finite contract?

A lost response can correspond to several different effects. Recovery depends
on which worlds remain possible and what future responses can distinguish.
JSON schemas alone cannot express those facts. Contingram asks an author to
declare them, keeps observations separate from effects, and analyzes every
admitted outcome.

Finite enumerated domains and data-only predicates make that model reviewable
and bounded. The cost is abstraction work and a small state space. Model
fidelity remains the author's responsibility; no automatic inference from
OpenAPI, prose, prompts, or traces is attempted.

## Why strong bounded completion?

The question is whether one observation-based controller safely reaches the
goal for every possible outcome within a fixed number of decisions. This
avoids assuming that a retry eventually works, a replica eventually catches
up, or the scheduler is fair. A loop that might succeed eventually is
insufficient. A negative result is scoped to its horizon.

Safety and completion are separate. Stopping safely with an unresolved effect
does not complete a task unless the author explicitly made that an acceptable
goal. History-sensitive safety must be modeled with state that remembers the
bad event, such as a duplicate-effect marker.

## Why both positive and negative evidence?

A policy must cover every possible observation after each selected action.
A negative certificate must rule out every available action. One alarming
trace cannot establish impossibility for all controllers. Portable certificates
make those obligations independently inspectable and allow manually authored
policies to use the same checker.

Search and checking use separate traversals but share the compiled finite
model. This is a useful reduction in trusted search logic, not full formal
verification or a proof that the source contract matches an API.

## Why Rust and a small dependency set?

Typed states and evidence variants make invalid combinations visible in code.
Safe Rust, explicit allocation limits, and portable CLI packaging fit hostile
JSON inputs. JSON serialization and SHA-256 use established dependencies. On Unix, `libc` supplies the `O_NONBLOCK` constant used by the safe standard-library file opener to avoid blocking on a raced FIFO; project code makes no unsafe FFI calls.
The project forbids unsafe code in its own sources. The digest binds canonical
semantics; it does not authenticate an author.

## Why no runtime executor?

Mapping an abstract action to a live API requires credentials, identity,
authorization, concurrency, and conformance assumptions. Those are separate
boundaries. `follow` returns a checked abstract decision and never performs it.
Existing runtime recovery systems may be the appropriate production tool;
Contingram helps review the interface they rely on.
