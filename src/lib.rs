//! The NES console's inter-chip contracts, in one place.
//!
//! Five chips and pipelines are being built in sibling repositories
//! (`6502`, `2a03`, `2c02`, `ntsc-crt`, and the console `nes` that plugs
//! them together). The rule that keeps them honest: **a chip crate never
//! knows what is on the other side of its pins.** Everything two crates
//! must agree on to talk lives here, in a crate with no dependencies and
//! no die data, so the definition cannot drift between repositories and
//! none of them may change it without a note. The precedent is
//! `v6502-pins` in the 6502 repository, which does the same job for the
//! engine ladder there.
//!
//! What is here:
//!
//! - [`FrameParity`], the NES frame constants, and [`DotFrame`]: moved
//!   from ntsc-crt (`ntsc-grid` and `ntsc-source-nes` re-export them for
//!   compatibility). These are MEASURED contracts: the ntsc-crt suite
//!   holds the parity arithmetic to exact ratios, and the 2c02 repo's
//!   P1 golden fills `DotFrame` off the switch-level chip's palette bus.
//! - [`pins`]: the 2A03 and 2C02 package pin frames and their DIP
//!   position tables. AUTHORED from the nesdev wiki's pinout pages
//!   (fetched 2026-09-02); each pin becomes measured only when a gate in
//!   a chip repo holds it to a netlist pad or a scope capture, and the
//!   milestone reports say which.
//! - [`cart`]: the 72-pin cartridge edge, the [`cart::Cartridge`] trait,
//!   and NROM, its first implementor; GxROM (mapper 66) joined it
//!   2026-09-12 for the bench's own cartridge, and MMC3 (mapper 4, with
//!   the scanline counter that watches PPU A12 and drives /IRQ)
//!   2026-09-20, for the games that bank. AUTHORED from the nesdev
//!   wiki's cartridge connector page (fetched 2026-09-02), its MMC3 page
//!   (fetched 2026-09-20) and the NES-001 schematic.
//! - [`audio`]: the AD1/AD2 sample stream. AUTHORED; its first consumer
//!   is the 2A03 repo's first-sound milestone.
//!
//! Pin frames here are instantaneous: one struct is the observable level
//! of every package pin at one half-step, and nothing else. Time (trace
//! formats, half-step counters, comparison drivers) belongs to each chip
//! repo's own golden machinery, the way `v6502-pins` wraps its frames in
//! a `Trace`; this crate deliberately does not own a clock.

#![forbid(unsafe_code)]

pub mod audio;
pub mod cart;
pub mod pins;

/// Frame parity. `OddShort` is the NES odd frame with rendering enabled,
/// which drops dot 340 from the pre-render line; it does not exist on the
/// broadcast profile, and ntsc-crt's geometry refuses it there by name.
///
/// Moved here from `ntsc-grid`, which re-exports it: the PPU emits the
/// parity and the encoder consumes it, so it is a bus fact, not a grid
/// fact.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FrameParity {
    Even,
    OddFull,
    OddShort,
}

pub const DOTS_PER_LINE: usize = 341;
pub const LINES: usize = 262;
pub const SAMPLES_PER_DOT: usize = 8;
pub const ACTIVE_FIRST_DOT: usize = 1;
pub const ACTIVE_DOTS: usize = 256;
pub const ACTIVE_ROWS: usize = 240;

/// One frame of PPU output: colour index (6-bit, $00..$3F) and emphasis
/// (3-bit, PPUMASK bits 5..7) per dot, row-major, always 341 x 262 (the
/// skipped dot of an OddShort frame is simply not read).
///
/// Moved here from `ntsc-source-nes`, which re-exports it: the PPU
/// ladder produces it and the encoder consumes it, so it is the waist of
/// the whole video path and neither side may own it.
#[derive(Clone, Debug)]
pub struct DotFrame {
    pub parity: FrameParity,
    pub colour: Vec<u8>,
    pub emphasis: Vec<u8>,
}

impl DotFrame {
    pub fn filled(parity: FrameParity, colour: u8, emphasis: u8) -> DotFrame {
        DotFrame {
            parity,
            colour: vec![colour; DOTS_PER_LINE * LINES],
            emphasis: vec![emphasis; DOTS_PER_LINE * LINES],
        }
    }

    pub fn at(&self, row: usize, dot: usize) -> (u8, u8) {
        let i = row * DOTS_PER_LINE + dot;
        (self.colour[i], self.emphasis[i])
    }

    pub fn set(&mut self, row: usize, dot: usize, colour: u8, emphasis: u8) {
        let i = row * DOTS_PER_LINE + dot;
        self.colour[i] = colour;
        self.emphasis[i] = emphasis;
    }

    /// The 256 x 240 active region as blargg-style 9-bit entries
    /// (emphasis << 6 | colour), row-major: what the oracle eats.
    pub fn active_entries(&self) -> Vec<u16> {
        let mut out = Vec::with_capacity(ACTIVE_DOTS * ACTIVE_ROWS);
        for row in 0..ACTIVE_ROWS {
            for dot in ACTIVE_FIRST_DOT..ACTIVE_FIRST_DOT + ACTIVE_DOTS {
                let (c, e) = self.at(row, dot);
                out.push(((e as u16) << 6) | c as u16);
            }
        }
        out
    }
}
