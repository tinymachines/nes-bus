//! The contract held to itself: encodings pinned, tables closed, NROM's
//! two output pins doing what the schematic says. The cross-repo half of
//! the N0 gate (ntsc-crt and 2c02 compiling against these types with
//! their goldens unchanged) lives in those repos' own suites.

use nes_bus::cart::{CartEdge, Cartridge, Mirroring, Nrom, CART_PINS};
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
        vout: 0.0,
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
        ad1: 0.0,
        ad2: 0.0,
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
