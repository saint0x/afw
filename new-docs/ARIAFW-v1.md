Aria Firmware (Rust) — Authoritative Design

Scope: Defines the Rust runtime that lives on our DigitalOcean droplet today and cross‑compiles to edge hardware tomorrow.  It consumes .aria bundles produced by Arc (Aria Compiler), whose job is to parse decorator-based TypeScript into manifests and executable bundles that the agentic runtime understands.

⸻

1 ▪ Big Picture

Layer	Role	Notes
User Source	• TypeScript with @aria decorators → declare Tools/Agents/Teams/Pipelines	Decorator-based syntax for agentic logic definition.
Arc Compiler	TypeScript decorators → manifest.json + source → .aria bundle	Lightweight parser; extracts configurations and packages source.
Aria Runtime (this doc)	Loads .aria bundles, orchestrates execution in Quilt containers	Written in Rust; manages tool registry & resource coordination.

Goal: Users can build entire programs by composing agentic primitives with shareable .aria bundles—no complex compilation needed.

⸻

2 ▪ Key Responsibilities of the Runtime
	1.	Ingest signed .aria bundles.
	2.	Populate tool/agent registries from bundles.
	3.	Delegate resource‑token scheduling to Quilt.
	4.	Execute bundles in isolated containers (Bun runtime).
	5.	Expose gRPC/WS APIs + Telemetry.

⸻

3 ▪ Components (Rust Crates)

Crate	Purpose
aria_runtime	Core agentic execution engine with planning/reflection
orc	Orchestrator (DAG planning, retries, RL)
token_api	Thin wrapper over Quilt sync‑engine (spawn, wait, events)
state_store	Custom KV storage (in-memory + planned disk persistence)
vec_cache	Lightweight vector database for embeddings
pkg_store	.aria bundle loading, CAS + Minisign verify
hostfx	host_llm / host_net / host_io / host_gpu interfaces
telemetry	Custom metrics and observability
crypto	Cryptographic utilities (wrapping safe libs)


⸻

4 ▪ Runtime Lifecycle

graph TD
    BOOT[Boot: systemd] --> CONFIG[Load aria.toml]
    CONFIG --> PKG_STORE[Start pkg_store]
    PKG_STORE --> LISTEN(Listen gRPC 7600 / WS 7601)
    LISTEN -->|Upload .aria| VERIFY[Verify & Install]
    VERIFY --> REGISTRY[Update Tool/Agent Registry]
    REGISTRY --> ORC[Plan DAG]
    ORC --> TOKEN[token_api.spawn(tokens)]
    TOKEN --> QuiltSync[Quilt Sync‑Engine]
    QuiltSync --> QMGR[Quilt ContainerMgr]
    QMGR --> GUEST[Bun Container Execution]
    GUEST -->|ICC delta| STATE[state_store]
    STATE --> ORC
    GUEST -->|telemetry| METRICS[telemetry]
    QuiltSync -->|deadlock evt| ORC


⸻

5 ▪ Resource Tokens

Arc compiler extracts tokens from decorator configs → Runtime passes them to Quilt.
	•	Example: kv:orders.pending:rw, vector:embeddings:user:append, llm:openai:gpt4.
	•	Quilt sync‑engine decides queueing; runtime listens to events.

⸻

6 ▪ IPC: Unix‑Socket Fabric

Path	Purpose
/run/aria/containers/<CID>/icc.sock	STDIO, logs, KV deltas
/run/quilt/api.sock	token_api control channel
/run/aria/hostfx/<fx>.sock	local host‑function shims
Fast, secure, auto‑cleaning—no TCP overhead on‑box.	


⸻

7 ▪ Security
	•	Minisign signatures on .aria bundles.
	•	Token‑derived seccomp profiles.
	•	Cgroup v2 & userns via Quilt.
	•	Optional HTTP egress proxy for audit.

⸻

8 ▪ Observability
	•	Custom metrics: cpu/ram, token backlog, registry stats.
	•	Structured logging per execution context.
	•	Real-time dashboard via WebSocket.

⸻

9 ▪ DigitalOcean Deployment
	1.	Resize droplet → 4 vCPU / 8 GB + 100 GB volume.
	2.	Install Rust toolchain and Bun runtime.
	3.	Deploy aria-fw binary + Enable aria-fw.service via systemd.
	4.	Open ports 7600 (gRPC), 7601 (WebSocket), 9090 (metrics).
	5.	TLS terminator (Caddy/Traefik) in front.

⸻

10 ▪ Roadmap (12 Weeks)

Wk	Deliverable
1‑2	pkg_store + .aria bundle verification
3‑4	token_api integration with Quilt
5‑6	aria_runtime with tool/agent registries
7‑8	gRPC/WS API + custom telemetry
9‑10	Arc compiler for TypeScript decorators
11‑12	Load testing & production deployment


⸻

TL;DR

The Rust runtime loads .aria bundles (manifest + TypeScript source) and either executes them in isolated Bun containers or extracts their tools/agents into global registries for reuse. Quilt handles resource isolation; aria_runtime orchestrates strategy. Users write decorator-based TypeScript, compile with `arc build`, and deploy bundles that become living, composable agentic capabilities.