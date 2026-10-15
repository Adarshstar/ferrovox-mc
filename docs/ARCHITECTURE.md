# Architecture

## Overview

FerroVox is a Cargo workspace split into small crates with a strict dependency direction:
protocol crates depend on `core`/`net`/`nbt`, never the other way around. This keeps the
game-logic core protocol-agnostic so Java and Bedrock front-ends can both drive it.

```
                 ┌──────────────────────┐
                 │   ferrovox-server    │   binary: config, runtime, listener wiring
                 └──────────┬───────────┘
                 ┌──────────┴───────────┐
     ┌───────────▼──────────┐  ┌────────▼─────────────────┐
     │ ferrovox-protocol-java│  │ ferrovox-protocol-bedrock │
     │ (TCP, VarInt framing) │  │ (UDP, RakNet framing)     │
     └───────────┬──────────┘  └────────┬─────────────────┘
                 │                       │
         ┌───────▼───────┐      ┌────────▼───────┐
         │ ferrovox-net   │      │  (raknet mod)   │
         │ (varint codec) │      └─────────────────┘
         └───────┬───────┘
                 │
         ┌───────▼───────┐       ┌────────────────┐
         │ ferrovox-nbt   │       │ ferrovox-core   │
         │ (NBT format)   │       │ config, runtime,│
         └───────────────┘       │ version pins     │
                                  └────────────────┘
```

As world/entity systems land (Phase 1+), they'll live in a new `ferrovox-world` /
`ferrovox-entity` crate that both protocol crates depend on, keeping gameplay logic
single-sourced instead of duplicated per edition.

## Crates

### `ferrovox-core`

Shared, protocol-agnostic foundations:

- `config` — `ServerConfig` loaded from `ferrovox.toml` (ports, MOTD, performance tuning).
  Missing fields fall back to sane defaults; the file is created on first run if absent.
- `runtime` — builds the Tokio multi-threaded runtime. Worker thread count defaults to
  `num_cpus::get()` (every detected logical core) unless pinned via
  `performance.worker_threads` in config — this is the "native multi-core" behavior.
- `versions` — single source of truth for the Java/Bedrock edition + protocol version
  constants the rest of the codebase targets. Bump these, not scattered literals.

### `ferrovox-net`

Byte-level primitives independent of any one edition's packet semantics:

- VarInt encode/decode (sync, for in-memory buffers, and async, for reading length-prefixed
  packets directly off a `TcpStream`).

### `ferrovox-nbt`

From-scratch big-endian NBT reader/writer (no external NBT crate). Used by the Java
protocol today for chat/world data, and will back world (region/anvil) file I/O in Phase 1.

### `ferrovox-protocol-java`

Java Edition wire protocol:

- `state` — the handshake/status/login/configuration/play state machine.
- `packet` — length-prefixed packet framing + `PacketReader`/`PacketWriter` helpers for
  protocol primitives (VarInt, String, etc.).
- `status` — server list ping JSON response builder.
- `connection` — the per-connection task: reads packets, dispatches by state.

### `ferrovox-protocol-bedrock`

Bedrock Edition / RakNet wire protocol:

- `raknet` — offline message handling (Unconnected Ping/Pong today; the reliable/ordered
  datagram layer and Bedrock login sequence land in Phase 1).

### `ferrovox-server`

The actual `ferrovox` binary. Loads config, builds the runtime, and spawns one task per
listener (Java TCP accept loop, Bedrock UDP recv loop), each handing connections off to the
appropriate protocol crate.

## Concurrency model

- One multi-threaded Tokio runtime for the whole process, sized to all detected CPU cores.
- Each accepted Java connection runs as its own `tokio::spawn`'d task — no thread-per-connection,
  so the server scales to many concurrent players without OS thread overhead.
- The Bedrock UDP socket is handled by a single receive loop today (Phase 0: stateless
  ping/pong only); Phase 1 introduces per-session state once the RakNet reliability layer and
  login sequence are implemented.
- Shared server state (config today; world/entity state from Phase 1 on) is wrapped in `Arc`
  and handed to tasks by clone, favoring message-passing and fine-grained locking over a
  single global lock as gameplay systems are added.

## Modernity / hardware notes

- LTO (`fat`) + single codegen unit + `panic = "abort"` in release profile for maximum
  optimization at the cost of longer release builds — appropriate for a server binary that's
  built once and run many times.
- No external NBT/VarInt crate dependency: full control over encoding behavior and easy to
  adapt as protocol versions shift.
- Runtime worker count auto-scales to available hardware rather than hardcoding a thread
  count, so the same binary uses a 2-core box and a 64-core box efficiently.
