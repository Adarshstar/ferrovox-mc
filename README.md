# FerroVox

**FerroVox** is a from-scratch, high-performance Minecraft server implementation written in
[Rust](https://www.rust-lang.org/), targeting **both** Minecraft: Java Edition and
Minecraft: Bedrock Edition natively — no proxy translation layer, no JVM.

> ⚠️ **Status: pre-alpha / Phase 0.** Networking handshakes work (Java status ping,
> Bedrock RakNet unconnected ping). Nothing resembling a playable server exists yet.
> See [ROADMAP.md](ROADMAP.md) for what's done and what's next.

## Why

Existing options force a tradeoff:

- **Vanilla / Paper / Spigot** — full parity, JVM overhead, closed-source core (vanilla) or
  patched-on performance (Paper).
- **PocketMine-MP** — Bedrock-only, PHP.
- **Geyser + Floodgate** — lets Bedrock players join a Java server via protocol translation,
  but it's a bridge, not a native Bedrock server.
- **Pumpkin, Feather, Valence** — excellent modern Rust Java-Edition servers, but none target
  Bedrock natively today.

FerroVox aims to natively speak **both** wire protocols from one Rust codebase, built around
an async, multi-core-first runtime, with a goal of full vanilla gameplay parity over time.

## Target versions

| Edition | Version  | Protocol |
|---------|----------|----------|
| Java    | 26.3     | 777      |
| Bedrock | 26.52.3  | 2193     |

Version pins live in one place: [`crates/ferrovox-core/src/lib.rs`](crates/ferrovox-core/src/lib.rs#L10)
(`ferrovox_core::versions`). Bumping a target version means updating that module plus whatever
packet changes the new protocol revision introduces — see
[ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the protocol crates are structured.

## Design goals

- **Native dual-protocol** — one world/entity/game-logic core, two protocol front-ends
  (`ferrovox-protocol-java`, `ferrovox-protocol-bedrock`), no runtime translation shim.
- **Multi-core by default** — built on Tokio's multi-threaded runtime; worker thread count
  auto-detects all available CPU cores (`num_cpus`) unless pinned in config.
- **Modern, safe systems code** — no `unsafe` outside narrowly justified, reviewed blocks;
  leverage Rust's ownership model instead of locks wherever the architecture allows.
- **From-scratch protocol/NBT/world code** — minimal external dependencies for wire formats,
  so behavior and performance are fully understood and controllable.
- **Vanilla-accurate over time** — gameplay systems (combat, redstone, world gen, mobs,
  villagers, enchanting, etc.) are built to match vanilla behavior, not just "close enough."

## Project layout

```
crates/
  ferrovox-core/              shared types, config, multi-core runtime setup
  ferrovox-net/                low-level codecs (VarInt, framing helpers)
  ferrovox-nbt/                from-scratch NBT reader/writer
  ferrovox-protocol-java/      Java Edition wire protocol
  ferrovox-protocol-bedrock/   Bedrock Edition / RakNet wire protocol
  ferrovox-server/             the `ferrovox` binary: wires protocols + core together
docs/
  ARCHITECTURE.md              crate boundaries and how data flows
ROADMAP.md                     phased build plan + live status
```

## Building

Requires a recent stable Rust toolchain (pinned in [`rust-toolchain.toml`](rust-toolchain.toml)).

```bash
cargo build --workspace --release
cargo test --workspace
./target/release/ferrovox
```

On first run, `ferrovox` writes a default `ferrovox.toml` next to the binary (ports, MOTD,
max players, performance tuning) and starts both the Java (TCP `25565`) and Bedrock
(UDP `19132`) listeners.

## Contributing

See [ROADMAP.md](ROADMAP.md) for current phase and open work. Issues and PRs welcome —
this is early and the architecture is still settling, so check in on scope before large
patches.

## License

MIT — see [LICENSE](LICENSE).
