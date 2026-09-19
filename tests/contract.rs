//! The contract held to itself: encodings pinned, tables closed, NROM's
//! two output pins doing what the schematic says. The cross-repo half of
//! the N0 gate (ntsc-crt and 2c02 compiling against these types with
//! their goldens unchanged) lives in those repos' own suites.

use nes_bus::cart::{CartEdge, Cartridge, Gxrom, Mirroring, Mmc3, Nrom, CART_PINS, A12_FILTER_DOTS};
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
