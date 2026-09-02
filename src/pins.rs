//! The two chips' package pins: an instantaneous frame per chip, and the
//! DIP position tables the frames are documented against.
//!
//! AUTHORED from the nesdev wiki's pinout pages (`CPU_pinout`,
//! `PPU_pinout`, fetched 2026-09-02). A pin here becomes measured when a
//! chip repo's gate holds it to a netlist pad or a scope capture; until
//! then the table is the claim and this comment is its label.
//!
//! Convention: active-low pins carry an `_n` suffix and hold the LEVEL of
//! the pin (`true` = electrically high = deasserted). No field stores an
//! assertion; every field stores a voltage rounded to a bool, so a
//! polarity mistake is visible at the contract rather than absorbed by
//! it. Analog pins (`vout`, `ad1`, `ad2`) are `f32` in the same units the
//! producing repo's transcription tables use.

/// One half-step at the 2C02's package pins.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PpuPins {
    /// CLK, pin 18: the 21.477272 MHz master, one toggle per half-step.
    pub clk: bool,
    /// R/W, pin 1 (CPU side): high = the CPU reads.
    pub cpu_rw: bool,
    /// CPU D0..D7, pins 2..9.
    pub cpu_d: u8,
    /// CPU A0..A2 (the register select), pins 12, 11, 10.
    pub cpu_a: u8,
    /// /CS, pin 13.
    pub cs_n: bool,
    /// EXT0..EXT3, pins 14..17.
    pub ext: u8,
    /// /INT, pin 19: the NMI line to the CPU.
    pub int_n: bool,
    /// /RST, pin 22.
    pub rst_n: bool,
    /// VOUT, pin 21: the composite level, in the transcribed level
    /// table's units.
    pub vout: f32,
    /// ALE, pin 39.
    pub ale: bool,
    /// AD0..AD7, pins 38 down to 31: the multiplexed low address/data.
    pub ad: u8,
    /// A8..A13, pins 30 down to 25.
    pub a_hi: u8,
    /// /RD, pin 24.
    pub rd_n: bool,
    /// /WR, pin 23.
    pub wr_n: bool,
}

impl PpuPins {
    /// Test-only: the same frame with /RD's polarity flipped, so MUTATE=1
    /// runs in dependent repos can prove their goldens go red when the
    /// contract lies about one pin. Using this anywhere but a mutation
    /// proof is a bug by name.
    #[doc(hidden)]
    pub fn mutated_rd_for_proof(mut self) -> PpuPins {
        self.rd_n = !self.rd_n;
        self
    }
}

/// One half-cycle at the 2A03's package pins.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CpuPins {
    /// CLK, pin 29: the 21.477272 MHz master in.
    pub clk: bool,
    /// AD1 and AD2, pins 1 and 2: the audio driver levels, in the units
    /// the mixer table uses.
    pub ad1: f32,
    pub ad2: f32,
    /// /RST, pin 3.
    pub rst_n: bool,
    /// A0..A15, pins 4..19.
    pub a: u16,
    /// D0..D7, pins 28 down to 21.
    pub d: u8,
    /// TST, pin 30 (grounded in the console).
    pub tst: bool,
    /// M2, pin 31.
    pub m2: bool,
    /// /IRQ, pin 32.
    pub irq_n: bool,
    /// /NMI, pin 33.
    pub nmi_n: bool,
    /// R/W, pin 34: high = read.
    pub rw: bool,
    /// /OE2 and /OE1, pins 35 and 36: the controller port enables.
    pub oe2_n: bool,
    pub oe1_n: bool,
    /// OUT0..OUT2, pins 39, 38, 37: OUT0 is the controller strobe.
    pub out: u8,
}

/// The 2C02's 40 DIP positions, pin number to name.
pub const PPU_PINS: [(u8, &str); 40] = [
    (1, "R/W"),
    (2, "D0"),
    (3, "D1"),
    (4, "D2"),
    (5, "D3"),
    (6, "D4"),
    (7, "D5"),
    (8, "D6"),
    (9, "D7"),
    (10, "A2"),
    (11, "A1"),
    (12, "A0"),
    (13, "/CS"),
    (14, "EXT0"),
    (15, "EXT1"),
    (16, "EXT2"),
    (17, "EXT3"),
    (18, "CLK"),
    (19, "/INT"),
    (20, "GND"),
    (21, "VOUT"),
    (22, "/RST"),
    (23, "/WR"),
    (24, "/RD"),
    (25, "A13"),
    (26, "A12"),
    (27, "A11"),
    (28, "A10"),
    (29, "A9"),
    (30, "A8"),
    (31, "AD7"),
    (32, "AD6"),
    (33, "AD5"),
    (34, "AD4"),
    (35, "AD3"),
    (36, "AD2"),
    (37, "AD1"),
    (38, "AD0"),
    (39, "ALE"),
    (40, "VCC"),
];

/// The 2A03's 40 DIP positions, pin number to name.
pub const CPU_PINS: [(u8, &str); 40] = [
    (1, "AD1"),
    (2, "AD2"),
    (3, "/RST"),
    (4, "A0"),
    (5, "A1"),
    (6, "A2"),
    (7, "A3"),
    (8, "A4"),
    (9, "A5"),
    (10, "A6"),
    (11, "A7"),
    (12, "A8"),
    (13, "A9"),
    (14, "A10"),
    (15, "A11"),
    (16, "A12"),
    (17, "A13"),
    (18, "A14"),
    (19, "A15"),
    (20, "GND"),
    (21, "D7"),
    (22, "D6"),
    (23, "D5"),
    (24, "D4"),
    (25, "D3"),
    (26, "D2"),
    (27, "D1"),
    (28, "D0"),
    (29, "CLK"),
    (30, "TST"),
    (31, "M2"),
    (32, "/IRQ"),
    (33, "/NMI"),
    (34, "R/W"),
    (35, "/OE2"),
    (36, "/OE1"),
    (37, "OUT2"),
    (38, "OUT1"),
    (39, "OUT0"),
    (40, "VCC"),
];
