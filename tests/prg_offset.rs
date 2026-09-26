//! `prg_offset` is the board's own read, told as a place: for every board,
//! after every write in a long run of bank switches, the PRG byte at the
//! offset it names is the byte `cpu_read` answers, at every address of
//! the window. The PRG is pseudo-random, so a wrong bank or a wrong
//! offset inside one disagrees somewhere in 32 KiB; and each banked board
//! must be seen at more than one bank, or the run proved nothing.

use nes_bus::cart::{Cartridge, Cnrom, Gxrom, Mirroring, Mmc1, Mmc2, Mmc3, Nrom, Uxrom};
use std::collections::HashSet;

fn noise(n: usize, seed: u32) -> Vec<u8> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

/// Holds the board to its reads across `writes`, and returns how many
/// places $8000 was seen at.
fn holds(c: &mut dyn Cartridge, prg: &[u8], writes: &[(u16, u8)]) -> usize {
    let mut at_8000 = HashSet::new();
    let mut check = |c: &mut dyn Cartridge| {
        for a in 0x8000u32..=0xffff {
            let a = a as u16;
            let off = c.prg_offset(a).unwrap_or_else(|| panic!("no offset for ${a:04x}"));
            assert_eq!(Some(prg[off]), c.cpu_read(a), "${a:04x} named offset {off:#x}");
        }
        for a in [0x0000u16, 0x2002, 0x4016, 0x6000, 0x7fff] {
            assert_eq!(c.prg_offset(a), None, "${a:04x} is not the ROM");
        }
        at_8000.insert(c.prg_offset(0x8000).unwrap());
    };
    check(c);
    for &(a, v) in writes {
        c.cpu_write(a, v);
        check(c);
    }
    at_8000.len()
}

fn switching(seed: u32, n: usize) -> Vec<(u16, u8)> {
    let r = noise(3 * n, seed);
    (0..n).map(|i| (0x8000 | u16::from_le_bytes([r[3 * i], r[3 * i + 1]]), r[3 * i + 2])).collect()
}

#[test]
fn every_board_names_the_place_its_own_read_comes_from() {
    let prg = |n| noise(n, 0x9e37_79b9);
    let chr = vec![0u8; 0x8000];

    let p = prg(0x4000);
    assert_eq!(holds(&mut Nrom::new(p.clone(), chr[..0x2000].to_vec(), Mirroring::Vertical).unwrap(), &p, &[]), 1);
    let p = prg(0x8000);
    assert_eq!(holds(&mut Cnrom::new(p.clone(), chr.clone(), Mirroring::Vertical).unwrap(), &p, &switching(1, 16)), 1);

    let p = prg(0x20000);
    assert!(holds(&mut Gxrom::new(p.clone(), chr.clone(), Mirroring::Vertical).unwrap(), &p, &switching(2, 24)) > 1);
    let p = prg(0x20000);
    assert!(holds(&mut Uxrom::new(p.clone(), Vec::new(), Mirroring::Vertical).unwrap(), &p, &switching(3, 24)) > 1);
    let p = prg(0x20000);
    assert!(holds(&mut Mmc2::new(p.clone(), chr.clone(), Mirroring::Vertical).unwrap(), &p, &switching(4, 24)) > 1);
    // MMC1 takes a register a bit at a time, MMC3 a select then a value:
    // runs of random writes reach both (MMC1 wants five in a row with bit 7
    // clear, so its run is long).
    let p = prg(0x40000);
    assert!(holds(&mut Mmc1::new(p.clone(), chr.clone(), Mirroring::Vertical).unwrap(), &p, &switching(5, 2000)) > 1);
    let p = prg(0x40000);
    assert!(holds(&mut Mmc3::new(p.clone(), chr.clone(), Mirroring::Vertical).unwrap(), &p, &switching(6, 60)) > 1);
}
