Aria Language (v1 Draft)

0 · Purpose

Aria is a declarative, statically‑typed agent‑programming language built for the Truffle firmware stack.  It is the canonical way to describe tools, agents, and orchestration graphs that the cloud firmware can compile into containerised gRPC services.  The arc compiler turns .aria sources into .symphony bundles ready for hot‑loading by the orchestrator.

⸻

1 · Design Pillars

Pillar  Implication
Deterministic agent graphs  Every public entity is a total, typed JSON function.
Rust‑inspired algebraic data types  Sum / product types, pattern matching, zero‑cost abstractions.
Zig‑style readability Minimal punctuation, block‑based syntax.
Pythonic ergonomics get/set APIs, significant indentation optional but recommended.
Nix‑like declarativity  Configuration is data; side‑effects are explicit via effect rows.


⸻

2 · Hello World

package "hello‑world@0.1.0"

use std.net.http
use std.mem as mem   # KV helpers

# A pure tool
tool Echo(input { message: Str }) -> { echoed: Str }:
    return { echoed: input.message }

# An agent that calls Echo inside a container
agent Greeter(name: Str) -> { greeting: Str }:
    let future = spawn Echo({ message: "Hello, \(name)!" })
    let out = await future
    return { greeting: out.echoed }

# Firmware entry‑point
pipeline Main():
    mem.set key: "visits", inc: 1
    let res = Greeter("world")
    log.info res.greeting


⸻

3 · Syntax Reference (abridged)

3.1 Blocks & Statements
  • Braces optional when indentation is present.
  • Semicolons are not required; newlines terminate statements.

let x = 10            # immutable
let mut y = 20         # mutable

3.2 Entities

Keyword Purpose
tool  Pure, side‑effect‑free function.
agent May spawn, await, read/write memory.
team  Declarative fan‑out/fan‑in orchestration.
pipeline  Exposed gRPC entry‑point.
state Long‑lived strongly‑typed KV.
secret  Write‑once encrypted value.

3.3 Type Syntax

enum Role { Admin | User | Guest }
struct User { id: UUID, role: Role, name: Str }

3.4 Effects

Effects are declared with a postfix row:

tool PureAdder(a: Int, b: Int) -> Int | {}
agent NetworkPing(host: Str) -> Bool | { net, io }

An empty row {} means deterministic.

⸻

4 · Memory & Cache API

Aria code never instantiates storage; firmware mounts it.

mem.get key: "profile:<id>"
mem.set key: "chat‑history", value: msg, ttl: 2h      # short‑term

state visits: UInt64 default 0                        # long‑term

Granular retention:
  • ttl in seconds / human string (2h, 7d).
  • tier: scratch | short | long (compile‑time constant).
  • weight: 0‑1 → how much an LLM should consider this memory.

⸻

5 · Database Helpers

db.get table: "users", id: userId

db.set table: "orders", row: {
    id: newUUID(), amount: 42.0,
    created_at: now()
}

Underlying adapter is chosen by firmware; Aria just emits intents.

⸻

6 · Borrow & Effect Checker (“lite”)
  • Linear resources (state, secret) produce an affine token.
  • Compiler walks the AST; verifies no duplicate mutable borrows.
  • For JSON I/O the checker ensures every output key is populated exactly once.

⸻

7 · Standard Library Snapshot
  • std.json – schema derive, encode/decode.
  • std.mem  – get, set, del, ttl, weight.
  • std.db   – relational helpers with prepared statements.
  • std.net  – HTTP(S) client with auto‑retry & backoff.
  • std.llm  – call LLM with structured prompt & tool hints.
  • std.time, std.crypto, std.log.

⸻

8 · Toolchain (arc CLI)

arc new {name}         # scaffold
arc check                  # type+effect check only
arc build                  # emit .symphony
arc upload        # upload bundle

arc check --strict enables JSON key coverage & borrow lint.

⸻

9 · Edge‑Cases & Open Questions
  • Inline retention weights – accept expressions or literals only?
  • Dynamic spawn – allow computed target names, or insist on statically known identifiers?
  • Mutable arrays in JSON output – require mut annotation to reduce accidental sharing.
  • Foreign Functions – early plan: allow WASM plug‑ins declared via extern.
  • Error handling – surface as Result<T, E> style union or Pythonic exceptions?  (Vote needed.)

⸻

10 · Roadmap to v0.1
  1.  Grammar freeze → PEG spec.
  2.  arc parser & typer (rowan + custom infer).
  3.  TS backend & local dev loop.
  4.  Container backend integrating with containerismd.
  5.  Firmware ingestion path + retention tier plumbing.

Feedback needed: syntax bikeshedding, retention weights semantics, FFI priority.