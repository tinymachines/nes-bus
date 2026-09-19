//! The cartridge edge: the NES-001 72-pin connector as a typed struct,
//! the trait the console plugs a cartridge in through, and the boards:
//! NROM (mapper 0), GxROM (66) and MMC3 (4).
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

    /// The PPU's address bus at the dot the console is on, every access
    /// and not only the ones this cartridge answers: a board that counts
    /// lines watches the LINE A12, and the nametable fetches that pull
    /// it low between one line's sprites and the next line's tiles are
    /// what makes the count one a line. A board with nothing to count
    /// ignores it, which is why it has a default.
    ///
    /// The dot is the console's own PPU dot counter. It is time, not an
    /// index into anything: all that is read off it is how long A12 has
    /// been low, which is the filter the counting boards have.
    fn ppu_bus(&mut self, _ppu_a: u16, _dot: u64) {}

    /// /IRQ, pin 15, as the cartridge drives it: true while the
    /// cartridge pulls the line low. Open on a board with no interrupt
    /// of its own, which is the default.
    fn irq(&self) -> bool {
        false
    }

    /// True when the cartridge carries its own CHR RAM and banks it, so
    /// a console must not keep a second copy of it outside. NROM and
    /// GxROM are CHR ROM boards and say no; a console that wants CHR RAM
    /// with one of those keeps the 8 KiB itself, which is what the `nes`
    /// console has always done for a test cartridge.
    fn owns_chr_ram(&self) -> bool {
        false
    }
}

/// Mapper 0: 16 KiB (mirrored) or 32 KiB PRG ROM, 8 KiB CHR ROM, the
/// mirroring solder option, no PRG RAM (the Family BASIC variant is out
/// of scope). The one other board here is `Gxrom`, mapper 66, added
/// 2026-09-12 because the bench's cartridge is one.
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

/// Mapper 66, GNROM and MHROM: 64 or 128 KiB of PRG ROM in 32 KiB banks,
/// 8 to 32 KiB of CHR ROM in 8 KiB banks, one write-only register at
/// $8000-$FFFF selecting both (bits 4 and 5 the PRG bank, bits 0 and 1
/// the CHR bank), the mirroring solder option, no PRG RAM. The board has
/// no bus-conflict protection: a write is ANDed with the ROM byte at the
/// address it hits, as on the part (games write to an address whose ROM
/// byte equals the value for that reason). Super Mario Bros. + Duck Hunt
/// is this board (MHROM: 64 KiB PRG, 16 KiB CHR).
///
/// AUTHORED: the register's power-on value is not defined by the part; it
/// is 0 here (bank 0 of each), and nothing measured says otherwise yet.
pub struct Gxrom {
    prg: Vec<u8>,
    chr: Vec<u8>,
    mirroring: Mirroring,
    bank: u8,
}

impl Gxrom {
    /// `prg` must be 64 or 128 KiB; `chr` 8, 16 or 32 KiB. Refuses
    /// anything else by name rather than padding quietly.
    pub fn new(prg: Vec<u8>, chr: Vec<u8>, mirroring: Mirroring) -> Result<Gxrom, String> {
        match prg.len() {
            0x10000 | 0x20000 => {}
            n => return Err(format!("GxROM PRG must be 64 or 128 KiB, got {n} bytes")),
        }
        match chr.len() {
            0x2000 | 0x4000 | 0x8000 => {}
            n => return Err(format!("GxROM CHR must be 8, 16 or 32 KiB, got {n} bytes")),
        }
        Ok(Gxrom { prg, chr, mirroring, bank: 0 })
    }

    /// The register as last written (bits 4 and 5 PRG, bits 0 and 1 CHR).
    pub fn bank(&self) -> u8 {
        self.bank
    }

    fn prg_index(&self, a: u16) -> usize {
        let bank = ((self.bank >> 4) & 0x03) as usize;
        (bank * 0x8000 + (a as usize - 0x8000)) % self.prg.len()
    }
}

impl Cartridge for Gxrom {
    fn cpu_read(&mut self, a: u16) -> Option<u8> {
        if a >= 0x8000 {
            Some(self.prg[self.prg_index(a)])
        } else {
            None
        }
    }

    fn cpu_write(&mut self, a: u16, v: u8) {
        if a >= 0x8000 {
            // Bus conflict: the ROM drives the data bus at the same time,
            // and the register sees the AND of the two.
            let rom = self.prg[self.prg_index(a)];
            self.bank = v & rom;
        }
    }

    fn chr_read(&mut self, a: u16) -> Option<u8> {
        if a < 0x2000 {
            let bank = (self.bank & 0x03) as usize;
            Some(self.chr[(bank * 0x2000 + a as usize) % self.chr.len()])
        } else {
            None
        }
    }

    fn chr_write(&mut self, _a: u16, _v: u8) {
        // CHR is ROM on GxROM.
    }

    fn ciram(&self, ppu_a: u16) -> (bool, bool) {
        let a10 = match self.mirroring {
            Mirroring::Vertical => ppu_a & 0x0400 != 0,
            Mirroring::Horizontal => ppu_a & 0x0800 != 0,
        };
        let ce_n = ppu_a & 0x2000 == 0;
        (a10, ce_n)
    }
}

/// How long PPU A12 must have been low for its rise to reach MMC3's
/// counter, in PPU dots. The board filters the line with an RC network:
/// nesdev's MMC3 page has the counter "triggered on a rising edge after
/// the line has remained low for three falling edges of M2", and M2 is
/// the CPU's clock, three PPU dots to one. AUTHORED from that sentence
/// (fetched 2026-09-20); what it has to get right is that the four
/// garbage nametable fetches inside a line's sprite window, two dots of
/// A12 low each, do NOT clock the counter, while the tile fetches of
/// the next line, hundreds of dots, do. blargg's `3.A12_clocking` is
/// the ROM that measures the boundary.
pub const A12_FILTER_DOTS: u64 = 9;

/// PPU A12 as a counting cartridge sees it: every address the PPU puts
/// on the bus goes in with the dot it was on, and a rise is counted only
/// when the line had been low for at least `filter_dots` first.
///
/// This is the whole of what MMC3's scanline counter listens to, and it
/// is here rather than in the board so that a console's own instruments
/// and the board cannot disagree about it: it was two pieces of code
/// with two different units until 2026-09-20 (`nes-glue`'s latch test
/// counted in latch falls with a filter of three, this counts in dots
/// with a filter of nine, and they are the same physical filter).
/// `nes-glue` re-exports it.
///
/// With the filter at zero every rise counts, and a line of the PPU's
/// own schedule then reads eight instead of one: the two garbage
/// nametable fetches between one sprite slot and the next put A12 down
/// for two dots at a time. That is what the filter is for, and what
/// makes a test of it able to fail.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct A12Watcher {
    last_a12: bool,
    low_since: u64,
    pub filter_dots: u64,
    /// Counted rises since it was made.
    pub rises: u64,
}

impl Default for A12Watcher {
    fn default() -> A12Watcher {
        A12Watcher::with_filter(A12_FILTER_DOTS)
    }
}

impl A12Watcher {
    pub fn with_filter(filter_dots: u64) -> A12Watcher {
        A12Watcher { last_a12: false, low_since: 0, filter_dots, rises: 0 }
    }

    /// One PPU bus access. True when it was a rise the filter passed.
    pub fn saw(&mut self, ppu_a: u16, dot: u64) -> bool {
        let a12 = ppu_a & 0x1000 != 0;
        let counted = match (self.last_a12, a12) {
            (false, true) => {
                let passed = dot.saturating_sub(self.low_since) >= self.filter_dots;
                if passed {
                    self.rises += 1;
                }
                passed
            }
            (true, false) => {
                self.low_since = dot;
                false
            }
            _ => false,
        };
        self.last_a12 = a12;
        counted
    }
}

/// Mapper 4, the MMC3: PRG ROM in 8 KiB banks with one of them fixed,
/// CHR ROM in 1 and 2 KiB banks either way up, mirroring under software
/// control, 8 KiB of PRG RAM, and the scanline counter on PPU A12 that
/// drives /IRQ. Super Mario Bros. 2 and 3 are this board, and so are a
/// quarter of the library.
///
/// AUTHORED from the nesdev wiki's MMC3 page (fetched 2026-09-20). Two
/// details are worth stating because both are easy to get backwards:
///
/// - **The mirroring bit's name is inverted between the two
///   conventions.** The register reads `0: horizontal (A10); 1: vertical
///   (A11)` in the wiki's arrangement wording, and the pin in brackets
///   is what decides it here: bit 0 clear drives CIRAM A10 from PPU A10,
///   which is [`Mirroring::Vertical`] in this crate's naming, the
///   nametables side by side. The pin is the fact; the adjective is a
///   convention.
/// - **The counter is clocked by a line, not by a scanline.** Nothing
///   here knows what a scanline is. It sees PPU A12 rise after a long
///   low, which happens once a line only because a game puts its
///   background tiles in one pattern table and its sprites in the other.
///   A game that uses one table for both gets no interrupts, on the part
///   and here alike.
///
/// The revision modelled is the Sharp MMC3B: the IRQ is asserted
/// whenever the counter reaches zero, so a latch of 0 interrupts every
/// line. The NEC MMC3A asserts on the 1 to 0 transition instead and goes
/// quiet with a latch of 0; blargg's `5.MMC3_rev_A` and `6.MMC3_rev_B`
/// are the two ROMs that tell them apart.
pub struct Mmc3 {
    prg: Vec<u8>,
    chr: Vec<u8>,
    /// CHR RAM boards keep their 8 KiB here and take writes.
    chr_is_ram: bool,
    prg_ram: Vec<u8>,
    prg_ram_enable: bool,
    prg_ram_deny_writes: bool,
    /// $8000 as last written: bits 0..2 the register the next $8001
    /// lands in, bit 6 the PRG mode, bit 7 the CHR inversion.
    select: u8,
    r: [u8; 8],
    mirroring: Mirroring,
    irq_latch: u8,
    irq_counter: u8,
    irq_reload: bool,
    irq_enable: bool,
    irq_pending: bool,
    /// The line the counter listens to.
    a12: A12Watcher,
    /// The dot of the last counted rise: what a probe reads to say where
    /// in the PPU's frame the board counts.
    last_clock_dot: u64,
}

impl Mmc3 {
    /// `prg` must be a whole number of 8 KiB banks, at least four, and a
    /// power of two of them; `chr` either a whole number of 1 KiB banks
    /// (ROM) or empty, which is the 8 KiB CHR RAM board. Refuses
    /// anything else by name rather than padding quietly.
    pub fn new(prg: Vec<u8>, chr: Vec<u8>, mirroring: Mirroring) -> Result<Mmc3, String> {
        if !prg.len().is_multiple_of(0x2000) || prg.len() < 0x8000 {
            return Err(format!("MMC3 PRG must be a whole number of 8 KiB banks, at least 32 KiB, got {} bytes", prg.len()));
        }
        if !chr.is_empty() && !chr.len().is_multiple_of(0x400) {
            return Err(format!("MMC3 CHR must be a whole number of 1 KiB banks, or empty for the CHR RAM board, got {} bytes", chr.len()));
        }
        let chr_is_ram = chr.is_empty();
        let chr = if chr_is_ram { vec![0u8; 0x2000] } else { chr };
        Ok(Mmc3 {
            prg,
            chr,
            chr_is_ram,
            prg_ram: vec![0u8; 0x2000],
            // The part powers up with the RAM enabled and writable; a
            // game that cares writes $A001 before it trusts it.
            prg_ram_enable: true,
            prg_ram_deny_writes: false,
            select: 0,
            r: [0; 8],
            mirroring,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enable: false,
            irq_pending: false,
            a12: A12Watcher::default(),
            last_clock_dot: 0,
        })
    }

    /// The eight bank registers as last written, for a test or a probe.
    pub fn banks(&self) -> [u8; 8] {
        self.r
    }

    /// The counter, its latch, and whether the board is asserting /IRQ.
    pub fn irq_state(&self) -> (u8, u8, bool) {
        (self.irq_counter, self.irq_latch, self.irq_pending)
    }

    /// Filtered A12 rises since power-on, and the console dot the last
    /// one fell on: where the board counts, for a probe to print.
    pub fn clocks(&self) -> (u64, u64) {
        (self.a12.rises, self.last_clock_dot)
    }

    /// Test-only: the same board with the A12 filter taken out, so a
    /// test that claims the filter is what makes the count one a line
    /// can be made to fail. Without it every gap inside a line's sprite
    /// window counts, and a line clocks nine times instead of once.
    pub fn without_the_filter_for_proof(mut self) -> Mmc3 {
        self.a12.filter_dots = 0;
        self
    }

    /// Which 8 KiB PRG bank answers at `a`, as an index into `prg`.
    fn prg_bank(&self, a: u16) -> usize {
        let banks = self.prg.len() / 0x2000;
        let last = banks - 1;
        let second_last = banks - 2;
        let fixed_low = self.select & 0x40 != 0;
        let bank = match (a >> 13) & 3 {
            // $8000..$9FFF
            0 => {
                if fixed_low {
                    second_last
                } else {
                    self.r[6] as usize
                }
            }
            // $A000..$BFFF, R7 in both modes
            1 => self.r[7] as usize,
            // $C000..$DFFF
            2 => {
                if fixed_low {
                    self.r[6] as usize
                } else {
                    second_last
                }
            }
            // $E000..$FFFF, the last bank in both modes
            _ => last,
        };
        (bank % banks) * 0x2000 + (a as usize & 0x1fff)
    }

    /// Which 1 KiB CHR bank answers at `a`, as an index into `chr`.
    fn chr_bank(&self, a: u16) -> usize {
        // The inversion swaps the halves: the two 2 KiB registers sit
        // wherever A12 is what bit 7 says.
        let a = a & 0x1fff;
        let inverted = self.select & 0x80 != 0;
        let half = (a >> 12) & 1 != 0;
        let k = (a >> 10) & 7;
        let bank = if half == inverted {
            // The 2 KiB half: R0 for the first of it, R1 for the second,
            // and both ignore their low bit, which is what makes them
            // 2 KiB registers.
            let r = if k & 2 == 0 { self.r[0] } else { self.r[1] };
            (r & 0xfe) as usize + (k & 1) as usize
        } else {
            // The 1 KiB half: R2..R5 in order.
            self.r[2 + (k & 3) as usize] as usize
        };
        let banks = self.chr.len() / 0x400;
        (bank % banks) * 0x400 + (a as usize & 0x3ff)
    }

    /// One filtered rise of A12: the counter reloads if it is zero or
    /// the reload flag is set, and decrements otherwise; the board pulls
    /// /IRQ low when it is left at zero and interrupts are enabled.
    fn clock(&mut self) {
        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = self.irq_latch;
        } else {
            self.irq_counter -= 1;
        }
        self.irq_reload = false;
        if self.irq_counter == 0 && self.irq_enable {
            self.irq_pending = true;
        }
    }
}

impl Cartridge for Mmc3 {
    fn cpu_read(&mut self, a: u16) -> Option<u8> {
        if (0x6000..0x8000).contains(&a) {
            return self.prg_ram_enable.then(|| self.prg_ram[(a - 0x6000) as usize]);
        }
        if a >= 0x8000 {
            return Some(self.prg[self.prg_bank(a)]);
        }
        None
    }

    fn cpu_write(&mut self, a: u16, v: u8) {
        if (0x6000..0x8000).contains(&a) {
            if self.prg_ram_enable && !self.prg_ram_deny_writes {
                self.prg_ram[(a - 0x6000) as usize] = v;
            }
            return;
        }
        if a < 0x8000 {
            return;
        }
        // The register is chosen by the address's top two bits and its
        // lowest: four pairs, even and odd.
        match ((a >> 13) & 3, a & 1) {
            (0, 0) => self.select = v,
            (0, 1) => {
                let i = (self.select & 7) as usize;
                self.r[i] = v;
            }
            (1, 0) => {
                self.mirroring = if v & 1 == 0 { Mirroring::Vertical } else { Mirroring::Horizontal };
            }
            (1, 1) => {
                self.prg_ram_enable = v & 0x80 != 0;
                self.prg_ram_deny_writes = v & 0x40 != 0;
            }
            (2, 0) => self.irq_latch = v,
            (2, 1) => {
                // The counter is cleared now and reloaded at the next
                // filtered rise, which is what the flag is for.
                self.irq_counter = 0;
                self.irq_reload = true;
            }
            (3, 0) => {
                self.irq_enable = false;
                self.irq_pending = false;
            }
            _ => self.irq_enable = true,
        }
    }

    fn chr_read(&mut self, a: u16) -> Option<u8> {
        (a < 0x2000).then(|| self.chr[self.chr_bank(a)])
    }

    fn chr_write(&mut self, a: u16, v: u8) {
        if a < 0x2000 && self.chr_is_ram {
            let i = self.chr_bank(a);
            self.chr[i] = v;
        }
    }

    fn ciram(&self, ppu_a: u16) -> (bool, bool) {
        let a10 = match self.mirroring {
            Mirroring::Vertical => ppu_a & 0x0400 != 0,
            Mirroring::Horizontal => ppu_a & 0x0800 != 0,
        };
        let ce_n = ppu_a & 0x2000 == 0;
        (a10, ce_n)
    }

    fn ppu_bus(&mut self, ppu_a: u16, dot: u64) {
        if self.a12.saw(ppu_a, dot) {
            self.last_clock_dot = dot;
            self.clock();
        }
    }

    fn irq(&self) -> bool {
        self.irq_pending
    }

    fn owns_chr_ram(&self) -> bool {
        self.chr_is_ram
    }
}
