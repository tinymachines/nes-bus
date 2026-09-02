//! The cartridge edge: the NES-001 72-pin connector as a typed struct,
//! the trait the console plugs a cartridge in through, and NROM
//! (mapper 0), the first and only in-scope implementor.
//!
//! AUTHORED from the nesdev wiki's cartridge connector page (fetched
//! 2026-09-02) and the NES-001 schematic. Two facts worth stating
//! because they are easy to half-remember:
//!
//! - CPU A15 does not reach the edge. The mainboard's 74LS139 folds it
//!   into /ROMSEL (= not (A15 and M2)), which is what pin 50 carries.
//! - CIRAM A10 and /CIRAM CE are the two pins the CARTRIDGE drives on
//!   the PPU bus: nametable mirroring is a property of the cartridge,
//!   not the console. On NROM, CIRAM A10 is a solder option (PPU A10
//!   for vertical mirroring, PPU A11 for horizontal) and /CIRAM CE is
//!   wired to PPU /A13.

/// The 72-pin edge as one instantaneous frame: every signal at the
/// connector, grouped by bus rather than by pin number (the number-to-
/// name map is [`CART_PINS`]). Inputs and outputs are mixed here on
/// purpose; direction is documented per field, and the cartridge's two
/// outputs on the PPU bus are the last two fields.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CartEdge {
    /// CPU A0..A14, pins 13..2 and 39..41. A15 is folded into /ROMSEL.
    pub cpu_a: u16,
    /// /ROMSEL, pin 50: not (A15 and M2), from the mainboard's 74LS139.
    pub romsel_n: bool,
    /// M2, pin 38.
    pub m2: bool,
    /// CPU R/W, pin 14: high = read.
    pub cpu_rw: bool,
    /// CPU D0..D7, pins 49..42.
    pub cpu_d: u8,
    /// /IRQ, pin 15: driven by the cartridge, open where unused.
    pub irq_n: bool,
    /// PPU A0..A13, pins 29..23, 59..65 (see [`CART_PINS`] for the
    /// exact scatter).
    pub ppu_a: u16,
    /// PPU /A13, pin 58: the inverted A13 the 74HC04 provides.
    pub ppu_a13_n: bool,
    /// PPU D0..D7, pins 30..33 and 69..66.
    pub ppu_d: u8,
    /// PPU /RD, pin 21.
    pub ppu_rd_n: bool,
    /// PPU /WR, pin 56.
    pub ppu_wr_n: bool,
    /// EXP0..EXP9, pins 16..20 and 55..51: the expansion lines, unused
    /// by NROM.
    pub exp: u16,
    /// SYSTEM CLK, pin 37: the 21.477272 MHz master at the edge.
    pub system_clk: bool,
    /// CIC toPak / toMB / CLK / +RST, pins 34, 35, 70, 71.
    pub cic_to_pak: bool,
    pub cic_to_mb: bool,
    pub cic_clk: bool,
    pub cic_rst: bool,
    /// CIRAM A10, pin 22: DRIVEN BY THE CARTRIDGE.
    pub ciram_a10: bool,
    /// /CIRAM CE, pin 57: DRIVEN BY THE CARTRIDGE.
    pub ciram_ce_n: bool,
}

/// The 72 connector positions, pin number to name, front side 1..36 then
/// back side 37..72.
pub const CART_PINS: [(u8, &str); 72] = [
    (1, "GND"),
    (2, "CPU A11"),
    (3, "CPU A10"),
    (4, "CPU A9"),
    (5, "CPU A8"),
    (6, "CPU A7"),
    (7, "CPU A6"),
    (8, "CPU A5"),
    (9, "CPU A4"),
    (10, "CPU A3"),
    (11, "CPU A2"),
    (12, "CPU A1"),
    (13, "CPU A0"),
    (14, "CPU R/W"),
    (15, "/IRQ"),
    (16, "EXP0"),
    (17, "EXP1"),
    (18, "EXP2"),
    (19, "EXP3"),
    (20, "EXP4"),
    (21, "PPU /RD"),
    (22, "CIRAM A10"),
    (23, "PPU A6"),
    (24, "PPU A5"),
    (25, "PPU A4"),
    (26, "PPU A3"),
    (27, "PPU A2"),
    (28, "PPU A1"),
    (29, "PPU A0"),
    (30, "PPU D0"),
    (31, "PPU D1"),
    (32, "PPU D2"),
    (33, "PPU D3"),
    (34, "CIC toPak"),
    (35, "CIC toMB"),
    (36, "+5V"),
    (37, "SYSTEM CLK"),
    (38, "M2"),
    (39, "CPU A12"),
    (40, "CPU A13"),
    (41, "CPU A14"),
    (42, "CPU D7"),
    (43, "CPU D6"),
    (44, "CPU D5"),
    (45, "CPU D4"),
    (46, "CPU D3"),
    (47, "CPU D2"),
    (48, "CPU D1"),
    (49, "CPU D0"),
    (50, "/ROMSEL"),
    (51, "EXP9"),
    (52, "EXP8"),
    (53, "EXP7"),
    (54, "EXP6"),
    (55, "EXP5"),
    (56, "PPU /WR"),
    (57, "CIRAM /CE"),
    (58, "PPU /A13"),
    (59, "PPU A7"),
    (60, "PPU A8"),
    (61, "PPU A9"),
    (62, "PPU A11"),
    (63, "PPU A10"),
    (64, "PPU A12"),
    (65, "PPU A13"),
    (66, "PPU D7"),
    (67, "PPU D6"),
    (68, "PPU D5"),
    (69, "PPU D4"),
    (70, "CIC CLK"),
    (71, "CIC +RST"),
    (72, "GND"),
];

/// Nametable mirroring, the NROM solder option.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mirroring {
    /// CIRAM A10 = PPU A10: nametables side by side.
    Vertical,
    /// CIRAM A10 = PPU A11: nametables stacked.
    Horizontal,
}

/// What the console plugs in. Semantic level: an address goes in, a byte
/// comes out, `None` meaning the cartridge leaves the bus floating (open
/// bus is the CONSOLE's problem, and hiding it here would launder it).
/// Timing is not modelled at this level; the chip harnesses own edges.
pub trait Cartridge {
    /// CPU bus read at `a` (the full 16-bit address; the implementor
    /// derives its own /ROMSEL view of A15).
    fn cpu_read(&mut self, a: u16) -> Option<u8>;
    /// CPU bus write at `a`.
    fn cpu_write(&mut self, a: u16, v: u8);
    /// PPU bus read at `a` (14-bit).
    fn chr_read(&mut self, a: u16) -> Option<u8>;
    /// PPU bus write at `a`.
    fn chr_write(&mut self, a: u16, v: u8);
    /// The two pins the cartridge drives from `a`: (CIRAM A10,
    /// /CIRAM CE). This is where mirroring lives.
    fn ciram(&self, ppu_a: u16) -> (bool, bool);
}

/// Mapper 0: 16 KiB (mirrored) or 32 KiB PRG ROM, 8 KiB CHR ROM, the
/// mirroring solder option, no PRG RAM (the Family BASIC variant is out
/// of scope, like every other mapper).
pub struct Nrom {
    prg: Vec<u8>,
    chr: Vec<u8>,
    mirroring: Mirroring,
}

impl Nrom {
    /// `prg` must be 16 KiB or 32 KiB; `chr` must be 8 KiB. Refuses
    /// anything else by name rather than padding quietly.
    pub fn new(prg: Vec<u8>, chr: Vec<u8>, mirroring: Mirroring) -> Result<Nrom, String> {
        match prg.len() {
            0x4000 | 0x8000 => {}
            n => return Err(format!("NROM PRG must be 16 or 32 KiB, got {n} bytes")),
        }
        if chr.len() != 0x2000 {
            return Err(format!("NROM CHR must be 8 KiB, got {} bytes", chr.len()));
        }
        Ok(Nrom { prg, chr, mirroring })
    }
}

impl Cartridge for Nrom {
    fn cpu_read(&mut self, a: u16) -> Option<u8> {
        if a >= 0x8000 {
            Some(self.prg[(a as usize - 0x8000) % self.prg.len()])
        } else {
            None
        }
    }

    fn cpu_write(&mut self, _a: u16, _v: u8) {
        // PRG is ROM; NROM has nothing writable on the CPU bus.
    }

    fn chr_read(&mut self, a: u16) -> Option<u8> {
        if a < 0x2000 {
            Some(self.chr[a as usize])
        } else {
            None
        }
    }

    fn chr_write(&mut self, _a: u16, _v: u8) {
        // CHR is ROM on NROM.
    }

    fn ciram(&self, ppu_a: u16) -> (bool, bool) {
        let a10 = match self.mirroring {
            Mirroring::Vertical => ppu_a & 0x0400 != 0,
            Mirroring::Horizontal => ppu_a & 0x0800 != 0,
        };
        // /CE is wired to PPU /A13: enabled (low) when A13 is high.
        let ce_n = ppu_a & 0x2000 == 0;
        (a10, ce_n)
    }
}
