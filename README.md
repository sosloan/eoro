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
