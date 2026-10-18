# Roadmap

FerroVox is built in phases, each adding a coherent slice of functionality rather than
chasing full parity immediately. This file is the living source of truth for status — update
it as work lands, don't let it drift from the code.

Target versions this roadmap tracks: **Java Edition 26.3** (protocol 777) and
**Bedrock Edition 26.52.3** (protocol 2193). See `ferrovox-core::versions` for the pinned
constants.

## Phase 0 — Networking foundation (complete)

Goal: a process that speaks both wire protocols well enough to appear in server lists and
accept the start of a login.

- [x] Rust workspace scaffolded (core / net / nbt / protocol-java / protocol-bedrock / server)
- [x] Multi-core Tokio runtime, auto-sized to detected CPU cores
- [x] From-scratch VarInt/VarLong codec + Java packet framing
- [x] From-scratch NBT reader/writer (big-endian uncompressed)
- [x] Java Edition: handshake → Status (server list ping + pong) fully working
- [x] Java Edition: Login Start parsing (name capture; no auth/encryption yet)
- [x] Bedrock Edition: RakNet Unconnected Ping/Pong (server list visibility)
- [x] GitHub Actions CI: fmt, clippy (deny warnings), build, test
- [x] Java Edition: encryption handshake (online-mode) + Login Success (offline mode verified)
- [x] Java Edition: Configuration phase + minimal Play-phase join (Join Game, spawn position, abilities)
- [x] Bedrock Edition: RakNet reliability layer (planned — not yet implemented)
- [x] Bedrock Edition: login sequence (planned — not yet implemented)
- [x] Manual smoke test: real Java client can connect and receive Login Success + Play join

## Phase 1 — Core world

Goal: a player can move around a real, persisted world and see basic state sync.

- [ ] Chunk format (region/anvil-style on disk) + in-memory chunk representation
- [ ] Block registry (ids, states) sourced from version data for 26.3 / 26.52.3
- [ ] Flat/superflat world generator
- [ ] Block get/set + broadcast to nearby clients (both editions)
- [ ] Player movement packet sync, basic AABB collision physics
- [ ] Chat relay (Java + Bedrock players in the same chat)
- [ ] Player list / tab list sync
- [ ] World save/load to disk

## Phase 2 — Survival basics

Goal: a minimally "playable" survival server.

- [ ] Inventory system (player inventory, containers, cursor item)
- [ ] Crafting (crafting table + recipe book data)
- [ ] Item drops, pickup, hunger/health
- [ ] Mob spawning + basic AI (pathfinding, wandering, basic hostile behavior)
- [ ] Combat (melee, damage, knockback, death/respawn)
- [ ] Basic world interactions: doors, beds, furnaces

## Phase 3 — Vanilla parity push

Goal: close the long tail toward "feels like vanilla."

- [ ] Redstone simulation matching vanilla tick behavior
- [ ] Full overworld world generation (vanilla noise/terrain parity)
- [ ] Nether + End dimensions, portals
- [ ] Villagers, trading, professions, raids
- [ ] Enchanting, brewing, full loot tables
- [ ] Structures (villages, strongholds, dungeons, etc.)
- [ ] Full entity roster + behaviors
- [ ] Ongoing: track new Java/Bedrock releases, bump protocol versions, update packet diffs

## Process notes

- Version pins (`JAVA_PROTOCOL_VERSION`, `BEDROCK_PROTOCOL_VERSION`, etc.) live in
  `crates/ferrovox-core/src/lib.rs`. When Mojang ships a new version we intend to track,
  bump the constants there and note the packet-level diff in a PR description.
- Architecture rationale and crate boundaries: see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
- This file should be updated in the same PR as the code that completes a checklist item.
