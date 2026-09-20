//! The contract held to itself: encodings pinned, tables closed, NROM's
//! two output pins doing what the schematic says. The cross-repo half of
//! the N0 gate (ntsc-crt and 2c02 compiling against these types with
//! their goldens unchanged) lives in those repos' own suites.

use nes_bus::cart::{A12Watcher, CartEdge, Cartridge, Cnrom, Gxrom, Mirroring, Mmc1, Mmc1Mirroring, Mmc2, Mmc3, Nrom, Uxrom, CART_PINS, A12_FILTER_DOTS};
use nes_bus::pins::{CpuPins, PpuPins, CPU_PINS, PPU_PINS};
use nes_bus::{DotFrame, FrameParity, ACTIVE_DOTS, ACTIVE_ROWS, DOTS_PER_LINE, LINES};

#[test]
fn dot_frame_roundtrips_and_the_entry_encoding_is_pinned() {
    let mut f = DotFrame::filled(FrameParity::Even, 0x0f, 0);
    assert_eq!(f.colour.len(), DOTS_PER_LINE * LINES);
    f.set(3, 7, 0x2a, 0b101);
    assert_eq!(f.at(3, 7), (0x2a, 0b101));
    let entries = f.active_entries();
    assert_eq!(entries.len(), ACTIVE_DOTS * ACTIVE_ROWS);
    // Row 3, dot 7 is active column 6 (active starts at dot 1). The
    // blargg encoding is emphasis << 6 | colour, pinned by value here so
    // a re-encoding cannot pass as a refactor.
    assert_eq!(entries[3 * ACTIVE_DOTS + 6], (0b101 << 6) | 0x2a);
    assert_eq!(entries[0], 0x0f);
}

#[test]
fn the_pin_tables_are_closed_and_positionally_complete() {
    for (table, n) in [(&PPU_PINS[..], 40u8), (&CPU_PINS[..], 40), (&CART_PINS[..], 72)] {
        assert_eq!(table.len(), n as usize);
        let mut seen = vec![false; n as usize + 1];
        for &(pos, name) in table {
            assert!(pos >= 1 && pos <= n, "position {pos} out of range");
            assert!(!seen[pos as usize], "position {pos} listed twice");
            seen[pos as usize] = true;
            assert!(!name.is_empty());
        }
    }
    // The signals the console's glue is built on, present by name.
    let has = |t: &[(u8, &str)], s: &str| t.iter().any(|&(_, n)| n == s);
    for s in ["/ROMSEL", "M2", "CIRAM A10", "CIRAM /CE", "PPU /A13", "SYSTEM CLK"] {
        assert!(has(&CART_PINS, s), "cart edge missing {s}");
    }
    for s in ["ALE", "/RD", "/WR", "VOUT", "/INT"] {
        assert!(has(&PPU_PINS, s), "PPU missing {s}");
    }
    for s in ["AD1", "AD2", "M2", "OUT0", "/OE1", "/OE2", "TST"] {
        assert!(has(&CPU_PINS, s), "CPU missing {s}");
    }
    // A15 must NOT reach the cartridge edge: /ROMSEL replaces it.
    assert!(!has(&CART_PINS, "CPU A15"));
}

#[test]
fn nrom_mirrors_16k_and_refuses_wrong_sizes() {
    let mut prg = vec![0u8; 0x4000];
    prg[0x0123] = 0xab;
    let mut c = Nrom::new(prg, vec![0u8; 0x2000], Mirroring::Vertical).unwrap();
    assert_eq!(c.cpu_read(0x8123), Some(0xab));
    assert_eq!(c.cpu_read(0xc123), Some(0xab), "16 KiB PRG mirrors");
    assert_eq!(c.cpu_read(0x4123), None, "below $8000 is not the cartridge's");

    let mut prg32 = vec![0u8; 0x8000];
    prg32[0x4123] = 0xcd;
    let mut c32 = Nrom::new(prg32, vec![0u8; 0x2000], Mirroring::Vertical).unwrap();
    assert_eq!(c32.cpu_read(0xc123), Some(0xcd));
    assert_ne!(c32.cpu_read(0x8123), Some(0xcd), "32 KiB PRG does not mirror");

    assert!(Nrom::new(vec![0; 0x2000], vec![0; 0x2000], Mirroring::Vertical).is_err());
    assert!(Nrom::new(vec![0; 0x4000], vec![0; 0x1000], Mirroring::Vertical).is_err());
}

#[test]
fn nrom_drives_ciram_the_way_the_solder_options_do() {
    let v = Nrom::new(vec![0; 0x4000], vec![0; 0x2000], Mirroring::Vertical).unwrap();
    let h = Nrom::new(vec![0; 0x4000], vec![0; 0x2000], Mirroring::Horizontal).unwrap();
    // Vertical: CIRAM A10 = PPU A10, so $2000 and $2800 share it low/low
    // while $2400 raises it; horizontal: = PPU A11, so $2400 stays low
    // and $2800 raises it.
    assert!(!v.ciram(0x2000).0);
    assert!(v.ciram(0x2400).0);
    assert!(!v.ciram(0x2800).0);
    assert!(!h.ciram(0x2400).0);
    assert!(h.ciram(0x2800).0);
    // /CE follows PPU /A13: enabled only in $2000..$3FFF.
    assert!(!v.ciram(0x2000).1, "/CE asserted for nametables");
    assert!(v.ciram(0x1000).1, "/CE deasserted for pattern tables");
}

#[test]
fn chr_reads_are_the_cartridges_and_writes_are_not_nroms() {
    let mut chr = vec![0u8; 0x2000];
    chr[0x1abc] = 0x5a;
    let mut c = Nrom::new(vec![0; 0x4000], chr, Mirroring::Vertical).unwrap();
    assert_eq!(c.chr_read(0x1abc), Some(0x5a));
    assert_eq!(c.chr_read(0x2abc), None, "nametable space is CIRAM's, not CHR's");
    c.chr_write(0x1abc, 0x00);
    assert_eq!(c.chr_read(0x1abc), Some(0x5a), "CHR is ROM on NROM");
}

#[test]
fn the_rd_mutation_flips_exactly_one_pin() {
    let p = PpuPins {
        clk: false,
        cpu_rw: true,
        cpu_d: 0x00,
        cpu_a: 0,
        cs_n: true,
        ext: 0,
        int_n: true,
        rst_n: true,
        vout: None,
        ale: false,
        ad: 0x55,
        a_hi: 0x2a,
        rd_n: false,
        wr_n: true,
    };
    let m = p.mutated_rd_for_proof();
    assert_eq!(m.rd_n, !p.rd_n);
    assert_eq!(PpuPins { rd_n: p.rd_n, ..m }, p, "nothing else moved");
}

#[test]
fn the_frames_construct_with_every_field_named() {
    // Exhaustive construction is the drift alarm: a pin added to a frame
    // fails to compile here until this test (and so the release note)
    // names it.
    let _ = CpuPins {
        clk: false,
        ad1: None,
        ad2: None,
        rst_n: true,
        a: 0,
        d: 0,
        tst: false,
        m2: false,
        irq_n: true,
        nmi_n: true,
        rw: true,
        oe2_n: true,
        oe1_n: true,
        out: 0,
    };
    let _ = CartEdge {
        cpu_a: 0,
        romsel_n: true,
        m2: false,
        cpu_rw: true,
        cpu_d: 0,
        irq_n: true,
        ppu_a: 0,
        ppu_a13_n: true,
        ppu_d: 0,
        ppu_rd_n: true,
        ppu_wr_n: true,
        exp: 0,
        system_clk: false,
        cic_to_pak: false,
        cic_to_mb: false,
        cic_clk: false,
        cic_rst: false,
        ciram_a10: false,
        ciram_ce_n: true,
    };
}

/// Mapper 66: 32 KiB PRG banks and 8 KiB CHR banks from one register,
/// the write ANDed with the ROM byte it lands on, and the sizes refused
/// by name. Every branch here fails without the code it tests: the
/// bank switch, the conflict, and the size check each have a mutation
/// (drop the AND, ignore bits 4 and 5, accept any size) that this catches.
#[test]
fn gxrom_switches_both_banks_from_one_register_through_a_bus_conflict() {
    // PRG: bank b holds b everywhere except one byte of 0xFF at $8000's
    // slot, so a write of 0x3F there is not masked by the conflict.
    let mut prg = vec![0u8; 0x10000];
    for b in 0..2usize {
        for i in 0..0x8000usize {
            prg[b * 0x8000 + i] = b as u8;
        }
        prg[b * 0x8000] = 0xFF;
    }
    let mut chr = vec![0u8; 0x4000];
    for b in 0..2usize {
        for i in 0..0x2000usize {
            chr[b * 0x2000 + i] = 0x10 + b as u8;
        }
    }
    let mut c = Gxrom::new(prg, chr, Mirroring::Vertical).unwrap();
    assert_eq!(c.cpu_read(0x8001), Some(0), "power-on: PRG bank 0");
    assert_eq!(c.chr_read(0x0010), Some(0x10), "power-on: CHR bank 0");
    c.cpu_write(0x8000, 0x11); // PRG bank 1, CHR bank 1; ROM byte there is 0xFF, no masking
    assert_eq!(c.bank(), 0x11);
    assert_eq!(c.cpu_read(0x8001), Some(1), "PRG bank 1 after the write");
    assert_eq!(c.chr_read(0x0010), Some(0x11), "CHR bank 1 after the write");
    // The conflict: writing 0x00 at an address whose ROM byte is 1 gives
    // 0 (bank 0); writing 0x11 there gives 0x11 & 0x01 = 0x01.
    c.cpu_write(0x9000, 0x11);
    assert_eq!(c.bank(), 0x01, "the register sees the write ANDed with the ROM byte");
    assert_eq!(c.cpu_read(0x8001), Some(0));
    assert_eq!(c.chr_read(0x0010), Some(0x11));
    // Below $8000 the register is not reached.
    c.cpu_write(0x6000, 0x11);
    assert_eq!(c.bank(), 0x01);
    // Sizes refused by name.
    assert!(Gxrom::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical).is_err());
    assert!(Gxrom::new(vec![0; 0x10000], vec![0; 0x1000], Mirroring::Vertical).is_err());
    // Mirroring pins as NROM's.
    let v = Gxrom::new(vec![0; 0x10000], vec![0; 0x2000], Mirroring::Vertical).unwrap();
    assert_eq!(v.ciram(0x2400), (true, false));
    assert_eq!(v.ciram(0x2800), (false, false));
}

/// MMC3's two banking modes and the CHR inversion, from the register
/// writes a game makes: each bank filled with its own index, so the byte
/// read back names the bank that answered.
#[test]
fn mmc3_banks_prg_and_chr_the_way_the_two_modes_say() {
    let mut prg = vec![0u8; 8 * 0x2000];
    for (i, b) in prg.chunks_mut(0x2000).enumerate() {
        b.fill(i as u8);
    }
    let mut chr = vec![0u8; 16 * 0x400];
    for (i, b) in chr.chunks_mut(0x400).enumerate() {
        b.fill(i as u8);
    }
    let mut c = Mmc3::new(prg, chr, Mirroring::Vertical).unwrap();
    // R6 = 3, R7 = 5, and the four CHR registers.
    for (r, v) in [(6u8, 3u8), (7, 5), (0, 8), (1, 10), (2, 1), (3, 2), (4, 4), (5, 6)] {
        c.cpu_write(0x8000, r);
        c.cpu_write(0x8001, v);
    }
    // Mode 0: R6 at $8000, R7 at $A000, the second last at $C000, the
    // last at $E000.
    assert_eq!(c.cpu_read(0x8000), Some(3));
    assert_eq!(c.cpu_read(0xa000), Some(5));
    assert_eq!(c.cpu_read(0xc000), Some(6));
    assert_eq!(c.cpu_read(0xe000), Some(7));
    // Mode 1 swaps the first and third windows and nothing else.
    c.cpu_write(0x8000, 0x40 | 6);
    assert_eq!(c.cpu_read(0x8000), Some(6));
    assert_eq!(c.cpu_read(0xa000), Some(5));
    assert_eq!(c.cpu_read(0xc000), Some(3));
    assert_eq!(c.cpu_read(0xe000), Some(7));
    // CHR, inversion off: R0 and R1 are the 2 KiB windows low, and they
    // ignore their own low bit, so R1 = 10 answers 10 then 11.
    c.cpu_write(0x8000, 0);
    assert_eq!(c.chr_read(0x0000), Some(8));
    assert_eq!(c.chr_read(0x0400), Some(9));
    assert_eq!(c.chr_read(0x0800), Some(10));
    assert_eq!(c.chr_read(0x0c00), Some(11));
    assert_eq!(c.chr_read(0x1000), Some(1));
    assert_eq!(c.chr_read(0x1c00), Some(6));
    // Inversion on: the same registers, the halves the other way up.
    c.cpu_write(0x8000, 0x80);
    assert_eq!(c.chr_read(0x0000), Some(1));
    assert_eq!(c.chr_read(0x0c00), Some(6));
    assert_eq!(c.chr_read(0x1000), Some(8));
    assert_eq!(c.chr_read(0x1400), Some(9));
    assert_eq!(c.chr_read(0x1800), Some(10));
    assert_eq!(c.chr_read(0x1c00), Some(11));
    assert!(Mmc3::new(vec![0; 0x4000], vec![0; 0x2000], Mirroring::Vertical).is_err());
    assert!(Mmc3::new(vec![0; 0x8000], vec![0; 0x300], Mirroring::Vertical).is_err());
}

/// The counter counts filtered rises of PPU A12, and the filter is what
/// makes it one a line: the four nametable fetches inside a line's
/// sprite window put A12 down for two dots at a time and must not count,
/// while the tile fetches of the next line do.
#[test]
fn mmc3_counts_one_line_and_its_filter_ignores_the_sprite_windows_gaps() {
    let mut c = Mmc3::new(vec![0u8; 0x8000], vec![0u8; 0x2000], Mirroring::Vertical).unwrap();
    c.cpu_write(0xc000, 2); // latch
    c.cpu_write(0xc001, 0); // reload at the next rise
    c.cpu_write(0xe001, 0); // enable
    let mut dot = 0u64;
    // A line as the PPU drives it with the background at $0000 and the
    // sprites at $1000: 256 dots of tile fetches, then eight sprite
    // slots of two nametable fetches and two pattern fetches, then the
    // next line's tiles.
    let line = |c: &mut Mmc3, dot: &mut u64| {
        for _ in 0..256 {
            c.ppu_bus(0x2000, *dot);
            *dot += 1;
        }
        for _ in 0..8 {
            c.ppu_bus(0x2000, *dot);
            *dot += 2;
            c.ppu_bus(0x2000, *dot);
            *dot += 2;
            c.ppu_bus(0x1ff0, *dot);
            *dot += 2;
            c.ppu_bus(0x1ff8, *dot);
            *dot += 2;
        }
        for _ in 0..21 {
            c.ppu_bus(0x2000, *dot);
            *dot += 1;
        }
    };
    line(&mut c, &mut dot);
    assert_eq!(c.clocks().0, 1, "one line, one clock: the gaps inside the sprite window are filtered");
    assert_eq!(c.irq_state().0, 2, "the first clock reloads from the latch");
    line(&mut c, &mut dot);
    assert_eq!(c.irq_state().0, 1);
    assert!(!c.irq(), "not at one");
    line(&mut c, &mut dot);
    assert_eq!(c.irq_state().0, 0);
    assert!(c.irq(), "the board pulls /IRQ low when the counter reaches zero");
    c.cpu_write(0xe000, 0);
    assert!(!c.irq(), "a write to $E000 acknowledges it");
    // The filter at its boundary. A12 is low here, so each round of
    // this raises it (which is the line's own clock), drops it, and
    // raises it again after a measured gap: only the second rise is
    // the one under test.
    let short = {
        c.ppu_bus(0x1000, dot);
        let before = c.clocks().0;
        dot += 2;
        c.ppu_bus(0x0000, dot);
        dot += A12_FILTER_DOTS - 1;
        c.ppu_bus(0x1000, dot);
        c.clocks().0 - before
    };
    assert_eq!(short, 0, "a rise {} dots after the fall is inside the filter", A12_FILTER_DOTS - 1);
    let long = {
        let before = c.clocks().0;
        dot += 2;
        c.ppu_bus(0x0000, dot);
        dot += A12_FILTER_DOTS;
        c.ppu_bus(0x1000, dot);
        c.clocks().0 - before
    };
    assert_eq!(long, 1, "and {A12_FILTER_DOTS} dots after it is outside");
}

/// The mirroring register drives the pin the cartridge owns, and the
/// naming is the trap: the register's 0 is called horizontal in the
/// wiki's arrangement wording and drives CIRAM A10 from PPU A10, which
/// this crate calls Vertical. The pin is what is asserted here.
#[test]
fn mmc3_mirroring_follows_the_pin_and_not_the_adjective() {
    let mut c = Mmc3::new(vec![0u8; 0x8000], vec![0u8; 0x2000], Mirroring::Horizontal).unwrap();
    c.cpu_write(0xa000, 0);
    assert!(c.ciram(0x2400).0, "A10 set: CIRAM A10 follows PPU A10");
    assert!(!c.ciram(0x2800).0);
    c.cpu_write(0xa000, 1);
    assert!(!c.ciram(0x2400).0, "A11 now: CIRAM A10 follows PPU A11");
    assert!(c.ciram(0x2800).0);
}

/// The proof that the filter test can fail: the same line through a
/// board with the filter taken out counts every gap in the sprite
/// window, nine times over.
#[test]
fn mmc3_without_its_filter_counts_the_sprite_windows_gaps_too() {
    let mut c = Mmc3::new(vec![0u8; 0x8000], vec![0u8; 0x2000], Mirroring::Vertical).unwrap().without_the_filter_for_proof();
    let mut dot = 0u64;
    for _ in 0..256 {
        c.ppu_bus(0x2000, dot);
        dot += 1;
    }
    for _ in 0..8 {
        for a in [0x2000u16, 0x2000, 0x1ff0, 0x1ff8] {
            c.ppu_bus(a, dot);
            dot += 2;
        }
    }
    assert_eq!(c.clocks().0, 8, "one rise a slot with no filter, where the part counts once a line");
}

/// UxROM: the low half of the window switches and the high half does
/// not, through the same bus conflict GxROM has. Each bank filled with
/// its own index, so the byte read back names the bank that answered.
#[test]
fn uxrom_switches_the_low_half_and_fixes_the_last_bank_high() {
    let mut prg = vec![0u8; 8 * 0x4000];
    for (i, b) in prg.chunks_mut(0x4000).enumerate() {
        b.fill(i as u8);
        // One byte of $FF at each bank's start, so a write there is not
        // masked by the conflict.
        b[0] = 0xff;
    }
    let mut c = Uxrom::new(prg, Vec::new(), Mirroring::Vertical).unwrap();
    assert_eq!(c.cpu_read(0x8001), Some(0), "power-on: bank 0 low");
    assert_eq!(c.cpu_read(0xc001), Some(7), "the last bank is fixed high");
    c.cpu_write(0x8000, 0x05);
    assert_eq!(c.bank(), 5);
    assert_eq!(c.cpu_read(0x8001), Some(5), "bank 5 low");
    assert_eq!(c.cpu_read(0xc001), Some(7), "and the high half has not moved");
    // The conflict: at $8001 the ROM byte is 5, so a write of 3 lands as
    // 3 & 5 = 1.
    c.cpu_write(0x8001, 0x03);
    assert_eq!(c.bank(), 0x01, "the register sees the write ANDed with the ROM byte");
    // Below $8000 the register is not reached.
    c.cpu_write(0x6000, 0x07);
    assert_eq!(c.bank(), 0x01);
    // CHR is the board's own RAM, and it says so.
    assert!(c.owns_chr_ram());
    c.chr_write(0x0123, 0x5a);
    assert_eq!(c.chr_read(0x0123), Some(0x5a));
    assert_eq!(c.chr_read(0x2000), None, "the board answers below $2000 only");
    // Refused by name: CHR ROM, and a size that is not whole banks.
    assert!(Uxrom::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical).is_err());
    assert!(Uxrom::new(vec![0; 0x5000], Vec::new(), Mirroring::Vertical).is_err());
    assert!(Uxrom::new(vec![0; 0x80000], Vec::new(), Mirroring::Vertical).is_err());
}

/// CNROM: the PRG does not move, the CHR does, and the latch is two bits
/// wide, so a write of $07 selects bank 3.
#[test]
fn cnrom_switches_chr_only_and_latches_two_bits() {
    let mut prg = vec![0u8; 0x8000];
    prg.fill(0xff);
    let mut chr = vec![0u8; 4 * 0x2000];
    for (i, b) in chr.chunks_mut(0x2000).enumerate() {
        b.fill(0x10 + i as u8);
    }
    let mut c = Cnrom::new(prg, chr, Mirroring::Horizontal).unwrap();
    assert_eq!(c.chr_read(0x0010), Some(0x10), "power-on: CHR bank 0");
    c.cpu_write(0x8000, 0x02);
    assert_eq!(c.bank(), 2);
    assert_eq!(c.chr_read(0x0010), Some(0x12));
    assert_eq!(c.cpu_read(0x8000), Some(0xff), "PRG does not move on this board");
    // Two bits: $07 is bank 3, not bank 7, and there is no bank 7 to
    // reach. The ROM is all $FF, so the conflict masks nothing.
    c.cpu_write(0xffff, 0x07);
    assert_eq!(c.bank(), 3);
    assert_eq!(c.chr_read(0x0010), Some(0x13));
    // Mirroring pins as NROM's.
    assert_eq!(c.ciram(0x2400), (false, false));
    assert_eq!(c.ciram(0x2800), (true, false));
    assert!(!c.owns_chr_ram());
    assert!(Cnrom::new(vec![0; 0x2000], vec![0; 0x2000], Mirroring::Vertical).is_err());
    assert!(Cnrom::new(vec![0; 0x8000], vec![0; 0x1000], Mirroring::Vertical).is_err());
}

/// One five-bit word through MMC1's serial port, the way a game writes
/// it: five writes carrying bit 0, lowest first, and the FIFTH write's
/// address choosing the register.
fn mmc1_write(c: &mut Mmc1, a: u16, word: u8) {
    for i in 0..5 {
        c.cpu_write(a, (word >> i) & 1);
    }
}

/// MMC1's three PRG modes and its two CHR modes, from the register
/// writes a game makes.
#[test]
fn mmc1_banks_prg_and_chr_the_way_its_modes_say() {
    let mut prg = vec![0u8; 8 * 0x4000];
    for (i, b) in prg.chunks_mut(0x4000).enumerate() {
        b.fill(i as u8);
    }
    let mut chr = vec![0u8; 8 * 0x1000];
    for (i, b) in chr.chunks_mut(0x1000).enumerate() {
        b.fill(0x10 + i as u8);
    }
    let mut c = Mmc1::new(prg, chr, Mirroring::Vertical).unwrap();
    // Power-on is the reset state: PRG mode 3, the last bank at $C000.
    assert_eq!(c.registers().0, 0x0c);
    assert_eq!(c.cpu_read(0x8000), Some(0), "mode 3: bank 0 at $8000");
    assert_eq!(c.cpu_read(0xc000), Some(7), "mode 3: the last bank at $C000");
    mmc1_write(&mut c, 0xe000, 5);
    assert_eq!(c.cpu_read(0x8000), Some(5));
    assert_eq!(c.cpu_read(0xc000), Some(7), "the fixed half does not move");
    // Mode 2: the FIRST bank fixed low, the selection high.
    mmc1_write(&mut c, 0x8000, 0x08);
    assert_eq!(c.cpu_read(0x8000), Some(0), "mode 2: bank 0 fixed at $8000");
    assert_eq!(c.cpu_read(0xc000), Some(5), "mode 2: the selection at $C000");
    // Mode 0: one 32 KiB bank, the selection's low bit ignored.
    mmc1_write(&mut c, 0x8000, 0x00);
    assert_eq!(c.cpu_read(0x8000), Some(4), "mode 0: the pair starting at 4");
    assert_eq!(c.cpu_read(0xc000), Some(5), "mode 0: and its second half");
    // CHR, 8 KiB at a time (control bit 4 clear, as it is now): chr0's
    // low bit is ignored and the halves follow each other.
    mmc1_write(&mut c, 0xa000, 3);
    assert_eq!(c.chr_read(0x0000), Some(0x12), "8 KiB mode: the pair starting at 2");
    assert_eq!(c.chr_read(0x1000), Some(0x13));
    // Two 4 KiB banks: control bit 4 set, and chr1 now reaches $1000.
    mmc1_write(&mut c, 0x8000, 0x10);
    mmc1_write(&mut c, 0xa000, 3);
    mmc1_write(&mut c, 0xc000, 6);
    assert_eq!(c.chr_read(0x0000), Some(0x13), "4 KiB mode: chr0 exactly");
    assert_eq!(c.chr_read(0x1000), Some(0x16), "4 KiB mode: chr1 at $1000");
}

/// MMC1's four mirroring modes, at the pin. Two of them are one screen,
/// which no solder option can do and which is why this board keeps its
/// own enum.
#[test]
fn mmc1_drives_ciram_four_ways_including_one_screen() {
    let mut c = Mmc1::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical).unwrap();
    for (bits, mode, lower, upper) in [
        (0u8, Mmc1Mirroring::OneScreenLower, false, false),
        (1, Mmc1Mirroring::OneScreenUpper, true, true),
        (2, Mmc1Mirroring::Vertical, false, true),
        (3, Mmc1Mirroring::Horizontal, false, false),
    ] {
        mmc1_write(&mut c, 0x8000, bits);
        assert_eq!(c.mirroring(), mode, "control bits {bits}");
        assert_eq!(c.ciram(0x2000).0, lower, "{mode:?} at $2000");
        assert_eq!(c.ciram(0x2400).0, upper, "{mode:?} at $2400");
        assert!(!c.ciram(0x2000).1, "/CE follows PPU A13, not the mode");
        assert!(c.ciram(0x1000).1);
    }
    // Horizontal is the one that reads A11, so $2800 is the other page.
    mmc1_write(&mut c, 0x8000, 3);
    assert!(c.ciram(0x2800).0);
}

/// A write with bit 7 set clears the serial port and forces PRG mode 3,
/// and leaves the rest of the control register alone. This is how every
/// SxROM game's reset code starts.
#[test]
fn mmc1_reset_write_clears_the_port_and_fixes_the_last_bank() {
    let mut prg = vec![0u8; 8 * 0x4000];
    for (i, b) in prg.chunks_mut(0x4000).enumerate() {
        b.fill(i as u8);
    }
    let mut c = Mmc1::new(prg, vec![0; 0x2000], Mirroring::Vertical).unwrap();
    // Horizontal mirroring, PRG mode 0, 4 KiB CHR: control = $13.
    mmc1_write(&mut c, 0x8000, 0x13);
    assert_eq!(c.registers().0, 0x13);
    assert_eq!(c.cpu_read(0xc000), Some(1), "mode 0 before the reset write");
    // Three bits into the port, then the reset.
    c.cpu_write(0xe000, 1);
    c.cpu_write(0xe000, 1);
    c.cpu_write(0xe000, 1);
    assert_eq!(c.shift_state(), (0b111, 3));
    c.cpu_write(0xe000, 0x80);
    assert_eq!(c.shift_state(), (0, 0), "the port is cleared");
    assert_eq!(c.registers().0, 0x1f, "PRG mode 3 is ORed in; mirroring and CHR mode stand");
    assert_eq!(c.cpu_read(0xc000), Some(7), "and the last bank is at $C000");
}

/// Two writes on consecutive CPU cycles are one write: the port takes
/// the first and ignores the second, which is what an RMW instruction's
/// dummy write and real write are. Without the rule an `INC $8000`
/// shifts twice and every word after it is wrong.
#[test]
fn mmc1_ignores_the_second_write_of_a_pair_on_consecutive_cycles() {
    let build = || Mmc1::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical).unwrap();
    // Three dots to a CPU cycle: an RMW's two writes are three apart.
    let rmw = |c: &mut Mmc1, dot: u64, v: u8| {
        c.cpu_write_at(0x8000, v, dot);
        c.cpu_write_at(0x8000, v, dot + nes_bus::cart::MMC1_PAIR_DOTS);
    };
    let mut c = build();
    rmw(&mut c, 100, 1);
    assert_eq!(c.shift_state(), (1, 1), "one bit in, not two");
    rmw(&mut c, 200, 1);
    assert_eq!(c.shift_state(), (0b11, 2));
    // And two writes far apart are two writes.
    c.cpu_write_at(0x8000, 1, 300);
    c.cpu_write_at(0x8000, 1, 400);
    assert_eq!(c.shift_state(), (0b1111, 4));
    // The proof that the rule is what does it.
    let mut m = build().without_the_pair_rule_for_proof();
    rmw(&mut m, 100, 1);
    assert_eq!(m.shift_state(), (0b11, 2), "without the rule the RMW shifts twice");
}

/// MMC2's PRG window moves and its three fixed banks do not.
#[test]
fn mmc2_switches_one_eighth_of_the_window_and_fixes_the_last_three() {
    let mut prg = vec![0u8; 16 * 0x2000];
    for (i, b) in prg.chunks_mut(0x2000).enumerate() {
        b.fill(i as u8);
    }
    let mut c = Mmc2::new(prg, vec![0u8; 0x2000], Mirroring::Vertical).unwrap();
    assert_eq!(c.cpu_read(0x8000), Some(0), "power-on: bank 0 in the window");
    assert_eq!(c.cpu_read(0xa000), Some(13), "the third-from-last is fixed at $A000");
    assert_eq!(c.cpu_read(0xc000), Some(14));
    assert_eq!(c.cpu_read(0xe000), Some(15), "and the last at $E000");
    c.cpu_write(0xa123, 0x05); // the PRG register, anywhere in $Axxx
    assert_eq!(c.banks().0, 5);
    assert_eq!(c.cpu_read(0x8000), Some(5), "the window moved");
    assert_eq!(c.cpu_read(0xa000), Some(13), "the fixed banks did not");
    assert_eq!(c.cpu_read(0xe000), Some(15));
    // $8000..$9FFF is the window, not a register: a write there is not
    // a bank select on this board.
    c.cpu_write(0x9000, 0x02);
    assert_eq!(c.banks().0, 5);
    assert_eq!(c.cpu_read(0x7fff), None, "and nothing below the window is the board's");
    assert!(Mmc2::new(vec![0; 0x4000], vec![0; 0x2000], Mirroring::Vertical).is_err());
    assert!(Mmc2::new(vec![0; 0x8000], Vec::new(), Mirroring::Vertical).is_err());
}

/// The CHR bank is chosen by a latch the PPU's own fetches flip, and the
/// byte the triggering fetch returns comes from the bank the latch chose
/// BEFORE it. Each half of the pattern table has its own latch and its
/// own pair of registers.
#[test]
fn mmc2s_latches_are_flipped_by_the_ppus_fetches_after_the_byte_is_read() {
    let mut chr = vec![0u8; 32 * 0x1000];
    for (i, b) in chr.chunks_mut(0x1000).enumerate() {
        b.fill(0x10 + i as u8);
    }
    let mut c = Mmc2::new(vec![0u8; 0x8000], chr, Mirroring::Vertical).unwrap();
    // Four different banks, one per (half, latch).
    c.cpu_write(0xb000, 1); // $0000 while latch 0 is $FD
    c.cpu_write(0xc000, 2); // $0000 while it is $FE
    c.cpu_write(0xd000, 3); // $1000 while latch 1 is $FD
    c.cpu_write(0xe000, 4); // $1000 while it is $FE
    assert_eq!(c.banks(), (0, [1, 3], [2, 4]));
    assert_eq!(c.latches(), [0xfd, 0xfd], "power-on: both on $FD");
    assert_eq!(c.chr_read(0x0000), Some(0x11), "the low half on its $FD bank");
    assert_eq!(c.chr_read(0x1000), Some(0x13), "the high half on its own");

    // The trigger for the low half is exactly $0FD8 and $0FE8. Reading
    // $0FE8 returns the $FD bank's byte and leaves the latch on $FE.
    assert_eq!(c.chr_read(0x0fe8), Some(0x11), "the byte comes from the bank the OLD latch chose");
    assert_eq!(c.latches()[0], 0xfe, "and the latch is left on $FE");
    assert_eq!(c.chr_read(0x0000), Some(0x12), "so the next read is the $FE bank's");
    assert_eq!(c.chr_read(0x1000), Some(0x13), "the other half is untouched");
    assert_eq!(c.chr_read(0x0fd8), Some(0x12), "and back the same way");
    assert_eq!(c.latches()[0], 0xfd);
    assert_eq!(c.chr_read(0x0000), Some(0x11));

    // The high half's triggers are RUNS of eight, not single addresses.
    assert_eq!(c.chr_read(0x1fef), Some(0x13), "the last of the $FE run");
    assert_eq!(c.latches()[1], 0xfe);
    assert_eq!(c.chr_read(0x1000), Some(0x14));
    assert_eq!(c.chr_read(0x1fdb), Some(0x14), "and inside the $FD run");
    assert_eq!(c.latches()[1], 0xfd);
    assert_eq!(c.chr_read(0x1000), Some(0x13));

    // The asymmetry, stated as a difference: the address one past the
    // low half's trigger does nothing, where the same offset in the high
    // half's run does.
    c.chr_read(0x0fd9);
    assert_eq!(c.latches()[0], 0xfd, "$0FD9 is not a trigger");
    c.chr_read(0x0fe9);
    assert_eq!(c.latches()[0], 0xfd, "nor is $0FE9");
    c.chr_read(0x1fd9);
    assert_eq!(c.latches()[1], 0xfd);
    c.chr_read(0x1fe9);
    assert_eq!(c.latches()[1], 0xfe, "$1FE9 is, because the high half's trigger is a run");

    assert_eq!(c.chr_read(0x2000), None, "the board answers below $2000 only");
    assert!(!c.owns_chr_ram());
}

/// MMC2's mirroring register, at the pin.
#[test]
fn mmc2_drives_ciram_from_its_register() {
    let mut c = Mmc2::new(vec![0u8; 0x8000], vec![0u8; 0x2000], Mirroring::Horizontal).unwrap();
    c.cpu_write(0xf000, 0);
    assert_eq!(c.ciram(0x2400), (true, false), "bit 0 clear puts CIRAM A10 on PPU A10");
    assert_eq!(c.ciram(0x2800), (false, false));
    c.cpu_write(0xffff, 1);
    assert_eq!(c.ciram(0x2400), (false, false), "bit 0 set puts it on PPU A11");
    assert_eq!(c.ciram(0x2800), (true, false));
    assert!(c.ciram(0x1000).1, "/CE follows PPU A13 either way");
}

/// The filter's own boundary, stated as the pair of dots on either side
/// of it.
///
/// Nine dots of A12 low is exactly three CPU cycles, so the third
/// falling edge of M2 lands ON the rise rather than before it and the
/// part does not count it; ten does. That one dot is a whole clock a
/// frame with the background at $1000, where A12 falls after the
/// pre-render line's last pattern fetch and rises again at line 0's
/// first, nine dots later: at nine the frame came to 242 clocks on
/// alternate frames where the part makes 241, which is what blargg's
/// `2-details` and `4-scanline_timing` both caught.
#[test]
fn the_a12_filter_takes_ten_dots_of_low_and_not_nine() {
    let count_after = |low_dots: u64| {
        let mut w = A12Watcher::default();
        // High, then low, then high again after `low_dots`.
        w.saw(0x1000, 0);
        w.saw(0x0000, 1);
        w.saw(0x1000, 1 + low_dots);
        w.rises
    };
    assert_eq!(A12_FILTER_DOTS, 10, "the filter is ten dots and the two cases below are its edges");
    assert_eq!(count_after(9), 0, "nine dots is three CPU cycles exactly: the third M2 fall is not BEFORE the rise");
    assert_eq!(count_after(10), 1, "ten is, and the rise reaches the counter");
    // And far outside it, both ways.
    assert_eq!(count_after(2), 0, "two dots is a gap between sprite slots");
    assert_eq!(count_after(340), 1, "a line is a line");
}
