# nes-bus

The NES console's inter-chip contracts, in one dependency-free crate.
Companion to [tinymachines/6502](https://github.com/tinymachines/6502),
[tinymachines/2c02](https://github.com/tinymachines/2c02) and
[tinymachines/ntsc-crt](https://github.com/tinymachines/ntsc-crt); the
plan it implements is the console sketch's milestone N0, and the rule it
enforces is the sketch's own: **a chip crate never knows what is on the
other side of its pins.** The console is the only thing that plugs
anything into anything, and everything two crates must agree on to talk
lives here, where it cannot drift between repositories.

The precedent is `v6502-pins` in the 6502 repository: one contract, no
dependencies, no die data, changed by nobody without a note.

## What is in it

| Module | Contents | Provenance |
|---|---|---|
| crate root | `FrameParity`, the NES frame constants, `DotFrame` | Moved from ntsc-crt (`ntsc-grid` / `ntsc-source-nes` re-export them); measured there by the grid suite and filled by the 2c02 repo's P1 golden |
| `pins` | `PpuPins`, `CpuPins`, the two 40-pin DIP tables | Authored from the nesdev wiki pinout pages, fetched 2026-09-02 |
| `cart` | `CartEdge`, the 72-pin table, the `Cartridge` trait, `Nrom`, `Mmc1` (mapper 1, the serial port), `Uxrom` (2), `Cnrom` (3), `Mmc3` (4, with the scanline counter on PPU A12), `Mmc2` (9, whose CHR bank the PPU's own fetches choose), `Gxrom` (66, the bench's cartridge) | Authored from the nesdev wiki cartridge connector, MMC1, MMC2, MMC3, UxROM, CNROM and pinout pages, and the NES-001 schematic |
| `audio` | `AudioSamples`, the AD1/AD2 stream with its rate as an exact ratio | Authored; first consumer is the 2A03 repo's first-sound milestone |

Authored means: the table is the claim until a gate in a chip repo holds
it to a netlist pad or a scope capture, and the milestone report that
does so says which pins it covered.

## Commands

```bash
cargo test          # the contract held to itself: encodings pinned,
                    # tables closed, NROM's CIRAM pins per the solder
                    # options, MMC3's two banking modes and its CHR
                    # inversion, and its counter clocked once a line
                    # with the sprite window's gaps filtered out
                    # (`without_the_filter_for_proof` is the board that
                    # must count nine times instead, so the claim can
                    # fail); UxROM's fixed high half and CNROM's
                    # two-bit latch, both through their bus conflict;
                    # MMC1's three PRG modes, its two CHR modes, its
                    # four mirroring modes at the pin, its reset write,
                    # and its serial port taking two writes on
                    # consecutive CPU cycles as one
                    # (`without_the_pair_rule_for_proof` shifts twice,
                    # so that claim can fail too); and MMC2's two
                    # latches, flipped by the PPU's fetches after the
                    # byte is read and not before, with the asymmetry
                    # between them stated as a difference ($0FD9 is not
                    # a trigger, $1FE9 is); and the A12 filter's own
                    # boundary, the two dots either side of it, which
                    # is one clock a frame with the background at $1000
cargo clippy --all-targets
```

A cartridge that counts lines needs two things from a console that NROM
never did, and both are trait methods with defaults so a board without
them is unchanged: `ppu_bus` is every PPU access with the console's dot,
because what MMC3 counts is the LINE A12 and not the data, and `irq` is
pin 15 as the cartridge drives it. `owns_chr_ram` is the third: a board
that banks its own CHR RAM says so, and the console keeps no second
copy. MMC2 needs nothing new: its latches ride on `chr_read`, which is
every pattern fetch and already carries the address the part watches.
The fourth is `cpu_write_at`, the same write with the console's
dot: MMC1's serial port ignores the second of two writes on consecutive
CPU cycles, which is what an RMW instruction on the window is, and the
dot is the only clock the edge carries.

The other half of the N0 gate lives in the dependent repositories:
ntsc-crt and 2c02 compile against these types with their existing
goldens unchanged, and under `MUTATE=1` the 2c02 harness reads the CHR
bus through `PpuPins::mutated_rd_for_proof`, which must send its P1
golden red.

## Licensing

MIT. This crate embeds no die data; the chips it describes carry their
own obligations in their own repositories.
