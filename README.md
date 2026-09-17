# eoro

## Fixed Point Labs — Rust Book (Data Edition)

Data Education Data Store with Nintendo64 GamePads, Transfer L2 Market Flywheel, BookOpen.

```rust
pub struct RustBookData {
    pub organization: &'static str,
    pub project: &'static str,
    pub title: &'static str,
    pub artist: &'static str,
    pub gaming_terms: &'static [&'static str],
    pub technical_terms: &'static [&'static str],
    pub cadence: &'static str,
}

pub const BOOK_DATA: RustBookData = RustBookData {
    organization: "Fixed Point Labs",
    project: "Education Data Store",
    title: "OGA (Original Gangster Arcade)",
    artist: "Hotboii",
    gaming_terms: &[
        "Atari",
        "arcade",
        "PONG",
        "Asteroids",
        "Centipede",
        "Breakout",
        "Missile Command",
        "Tempest",
        "E.T.",
        "Atari 2600",
        "joystick",
        "CX40",
        "cartridge",
        "TIA chip",
        "pixel",
        "sprite",
        "collision detection",
        "high score",
        "8-bit",
        "retro gaming",
        "vintage gaming",
    ],
    technical_terms: &[
        "MOS 6507",
        "128 bytes RAM",
        "160×192 resolution",
        "60 Hz refresh rate",
        "pixel-perfect",
        "scanline",
        "vector graphics",
        "raster graphics",
        "ROM cartridge",
        "1Z cadence — duodecimal, 12 not 16",
    ],
    cadence: "1Z",
};
```

## Native Home Mixer

Eoro includes a small, deterministic Rust recommendation pipeline inspired by
the open-source X Home Mixer architecture. It is available through the default
`home-mixer` Cargo feature and can be disabled in one step:

```console
cargo build --no-default-features
```

Run the built-in fixture or supply a JSON fixture:

```console
cargo run -- home-mixer
cargo run -- home-mixer fixtures/home_mixer.json
```

The command writes ranked results, score explanations, pagination data,
warnings, partial failures, source contribution counters, and per-stage
latencies as JSON. Set `EORO_HOME_MIXER_ENABLED=false` to disable execution at
runtime.

### Architecture

- `contracts` owns backend-independent requests, candidates, features, scores,
  cursors, warnings, failures, and diagnostics.
- `thunder` isolates retrieval and batch feature lookup behind
  `CandidateBackend`. `ThunderAdapter` enforces readiness, result bounds,
  cancellation signaling, and timeouts. `InMemoryThunder` provides local and
  test operation.
- `home_mixer` validates input, hydrates context, retrieves candidates,
  attributes provenance, deduplicates, filters, hydrates features, scores,
  normalizes, diversifies, selects, and explains results.
- `domain` takes an immutable snapshot of Eoro simulation state and explicitly
  maps athletes, zebras, orbits, and collaboration agents to candidates.
- `config` validates limits, source policy, fallback behavior, timeout,
  freshness decay, and scorer weights.

Thunder is optional by default: an unavailable or timed-out source produces a
diagnostic and baseline results. Configure it as required to fail the request
instead. Every request is bounded by candidate and result limits; ties are
resolved by ascending content ID for reproducible output.

### Upstream compatibility

The assessed upstream snapshot is:

- repository: `https://github.com/xai-org/x-algorithm.git`
- revision: `fad2f71edc780ab14e4cfaebbb8b221385782e43`
- Thunder source: `thunder/`
- Home Mixer source: `home-mixer/`
- license: Apache-2.0

The pin is recorded in `Cargo.toml` package metadata and exported by
`thunder::{UPSTREAM_REPOSITORY, UPSTREAM_REVISION, UPSTREAM_PATH}`.

At that revision, Thunder and Home Mixer are Rust source trees but are **not
Cargo packages**: neither has a `Cargo.toml`, and they reference unpublished
`xai_*` crates and generated protocol types. Cargo therefore cannot resolve
Thunder directly as a git dependency. Eoro uses a repository-owned protocol
adapter instead of pretending the upstream source is buildable. A production
backend should implement `CandidateBackend` using the deployed Thunder gRPC
API; upstream types must not leak into Eoro contracts.

Thunder serves an in-memory per-author post index populated by Kafka, defaults
to recent-first retrieval, and relies on Kafka replay after restart. The
published O2 client is not wired into its runtime persistence path. Upstream
Home Mixer is also Rust—not Scala—and uses query hydration, parallel sources,
candidate hydration, ordered filters and scorers, selection, post-selection
processing, and asynchronous side effects.

To update the pin:

1. review the target commit's license, Thunder protocol, limits, persistence,
   and Home Mixer stage ordering;
2. update both Cargo metadata and the constants in `src/thunder.rs`;
3. update the adapter contract if the deployed protocol changed;
4. run formatting, Clippy, all-feature tests, no-default-feature tests, and the
   fixture CLI smoke test.

See `THIRD_PARTY_NOTICES.md` for attribution. The implementation provides
architectural compatibility, not byte-for-byte or model-score parity with X's
production service.

### Rollout

Use fixture mode first, then shadow traffic with response comparison, followed
by a limited canary. Monitor empty-feed, fallback, duplicate, score
distribution, source contribution, and stage latency diagnostics. Roll back by
setting `EORO_HOME_MIXER_ENABLED=false` or building without the feature.
