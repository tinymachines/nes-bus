//! Saved states: every board saved in the middle of a long run of CPU and
//! PPU traffic, restored through its bytes onto a fresh board with the
//! same ROM, runs on exactly as the board that never stopped: every CPU
//! read, every CHR read, the IRQ line and the nametable pins, after every
//! operation. The traffic writes the registers, the PRG RAM and the CHR
//! RAM, and walks the PPU's A12 across scanlines so the MMC3's counter
//! and IRQ move. And no state holds the ROM: the console has it already,
//! and a saved state travels without the game, as a recording does.
//!
//! MUTATE_STATE=1 loses the MMC3's scanline counter on a load and must go
//! red.

use nes_bus::cart::{CartState, Cartridge, Cnrom, Gxrom, Mirroring, Mmc1, Mmc2, Mmc3, Nrom, Uxrom};

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

#[derive(Clone, Copy, Debug)]
enum Op {
    Write(u16, u8, u64),
    Read(u16),
    ChrWrite(u16, u8),
    ChrRead(u16),
    /// A PPU fetch at a dot: the MMC3 watches A12 here.
    Ppu(u16, u64),
}

/// A run of traffic: writes spread over the registers ($8000 up), the
/// PRG RAM ($6000) and CHR; the PPU's fetches walk scanlines with A12
/// rising once per line as sprite fetches make it.
fn traffic(seed: u32, n: usize) -> Vec<Op> {
    let r = noise(6 * n, seed);
    let mut dot = 0u64;
    (0..n)
        .map(|i| {
            let b = &r[6 * i..6 * i + 6];
            let a = u16::from_le_bytes([b[1], b[2]]);
            dot += 1 + b[4] as u64;
            // Register writes are one in sixteen, so a bank or an IRQ
            // setting lives long enough to be seen; every 400 operations
            // the MMC3's IRQ is set up again (a latch of 3, reload,
            // enable), since the random writes also turn it off.
            if i % 400 == 0 {
                return [Op::Write(0xc000, 3, dot), Op::Write(0xc001, 0, dot), Op::Write(0xe001, 0, dot)][(i / 400) % 3];
            }
            match b[0] % 16 {
                0 => Op::Write(0x8000 | a, b[3], dot),
                1 | 2 => Op::Write(0x6000 | (a & 0x1fff), b[3], dot),
                3 | 4 => Op::Read(0x6000 | (a & 0x9fff)),
                5 => Op::ChrWrite(a & 0x1fff, b[3]),
                6 => Op::ChrRead(a & 0x1fff),
                // Scanlines: background fetches at $0xxx, then sprite
                // fetches at $1xxx, so A12 rises once a line.
                n => Op::Ppu(if n < 12 { a & 0x0fff } else { 0x1000 | (a & 0x0fff) }, dot),
            }
        })
        .collect()
}

/// Apply one operation and say what the board answered.
fn apply(c: &mut dyn Cartridge, op: Op) -> (Option<u8>, bool, (bool, bool), (bool, bool)) {
    let out = match op {
        Op::Write(a, v, dot) => {
            c.cpu_write_at(a, v, dot);
            None
        }
        Op::Read(a) => c.cpu_read(a),
        Op::ChrWrite(a, v) => {
            c.chr_write(a, v);
            None
        }
        Op::ChrRead(a) => c.chr_read(a),
        Op::Ppu(a, dot) => {
            c.ppu_bus(a, dot);
            c.chr_read(a)
        }
    };
    (out, c.irq(), c.ciram(0x2000), c.ciram(0x2400))
}

fn restored(fresh: &mut dyn Cartridge, saved: &[u8]) {
    let st: CartState = postcard::from_bytes(saved).expect("a state decodes");
    fresh.load_state(&st).expect("a state restores");
}

/// Every `every` operations, a board is saved and a fresh one restored
/// from it; each restored board then runs beside the original to the
/// end. Returns how many operations the restored boards were held over.
fn holds(make: &dyn Fn() -> Box<dyn Cartridge>, prg: &[u8], ops: &[Op], every: usize) -> (usize, usize) {
    let mut a = make();
    let mut live: Vec<(usize, Box<dyn Cartridge>)> = Vec::new();
    let (mut held, mut irqs) = (0, 0);
    for (i, &op) in ops.iter().enumerate() {
        if i % every == 0 {
            let st = a.save_state().expect("every board here saves");
            let saved = postcard::to_allocvec(&st).expect("a state encodes");
            // The ROM stays out: no 32-byte run of it is in the bytes.
            for w in prg.chunks(4096) {
                assert!(!saved.windows(32).any(|x| x == &w[..32]), "a state held the PRG ROM");
            }
            let mut b = make();
            restored(b.as_mut(), &saved);
            live.push((i, b));
        }
        let want = apply(a.as_mut(), op);
        irqs += want.1 as usize;
        for (at, b) in live.iter_mut() {
            assert_eq!(apply(b.as_mut(), op), want, "restored at op {at}, op {i} ({op:?})");
            held += 1;
        }
    }
    (held, irqs)
}

#[test]
fn every_board_restored_mid_run_answers_as_if_it_had_never_stopped() {
    let chr = noise(0x8000, 0x1234_5678);
    let cases: Vec<(&str, Vec<u8>, Box<dyn Fn(Vec<u8>) -> Box<dyn Cartridge>>)> = vec![
        ("NROM", noise(0x8000, 11), Box::new({ let chr = chr.clone(); move |p| Box::new(Nrom::new(p, chr[..0x2000].to_vec(), Mirroring::Vertical).unwrap()) })),
        ("CNROM", noise(0x8000, 12), Box::new({ let chr = chr.clone(); move |p| Box::new(Cnrom::new(p, chr.clone(), Mirroring::Horizontal).unwrap()) })),
        ("GxROM", noise(0x20000, 13), Box::new({ let chr = chr.clone(); move |p| Box::new(Gxrom::new(p, chr.clone(), Mirroring::Vertical).unwrap()) })),
        ("UxROM", noise(0x20000, 14), Box::new(|p| Box::new(Uxrom::new(p, Vec::new(), Mirroring::Vertical).unwrap()))),
        ("MMC2", noise(0x20000, 15), Box::new({ let chr = chr.clone(); move |p| Box::new(Mmc2::new(p, chr.clone(), Mirroring::Vertical).unwrap()) })),
        ("MMC1, CHR ROM", noise(0x40000, 16), Box::new({ let chr = chr.clone(); move |p| Box::new(Mmc1::new(p, chr.clone(), Mirroring::Vertical).unwrap()) })),
        ("MMC1, CHR RAM", noise(0x40000, 17), Box::new(|p| Box::new(Mmc1::new(p, Vec::new(), Mirroring::Vertical).unwrap()))),
        ("MMC3, CHR ROM", noise(0x40000, 18), Box::new({ let chr = chr.clone(); move |p| Box::new(Mmc3::new(p, chr.clone(), Mirroring::Vertical).unwrap()) })),
        ("MMC3, CHR RAM", noise(0x40000, 19), Box::new(|p| Box::new(Mmc3::new(p, Vec::new(), Mirroring::Vertical).unwrap()))),
    ];
    for (name, prg, build) in &cases {
        let make = || build(prg.clone());
        let ops = traffic(name.len() as u32 * 7919, 6000);
        let (held, irqs) = holds(&make, prg, &ops, 250);
        assert!(held > 50_000, "{name}: only {held} operations held");
        if name.starts_with("MMC3") {
            assert!(irqs > 100, "{name}: the IRQ was up after only {irqs} operations");
        }
    }
}

#[test]
fn a_state_from_another_board_or_shape_is_refused_and_changes_nothing() {
    let chr = noise(0x8000, 3);
    let mmc3 = Mmc3::new(noise(0x40000, 1), chr.clone(), Mirroring::Vertical).unwrap();
    let mut ux = Uxrom::new(noise(0x20000, 2), Vec::new(), Mirroring::Vertical).unwrap();
    ux.cpu_write(0x8000, 3);
    let before = ux.cpu_read(0x8000);
    let e = ux.load_state(&mmc3.save_state().unwrap()).unwrap_err();
    assert!(e.contains("MMC3") && e.contains("UxROM"), "{e}");
    assert_eq!(ux.cpu_read(0x8000), before, "a refused load changed the board");
    // CHR RAM saved, loaded where the cartridge has CHR ROM.
    let ram = Mmc3::new(noise(0x40000, 1), Vec::new(), Mirroring::Vertical).unwrap();
    let mut rom = mmc3;
    assert!(rom.load_state(&ram.save_state().unwrap()).is_err());
}
