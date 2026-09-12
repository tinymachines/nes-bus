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
| `cart` | `CartEdge`, the 72-pin table, the `Cartridge` trait, `Nrom`, `Gxrom` (mapper 66, for the bench's cartridge) | Authored from the nesdev wiki cartridge connector page and the NES-001 schematic |
| `audio` | `AudioSamples`, the AD1/AD2 stream with its rate as an exact ratio | Authored; first consumer is the 2A03 repo's first-sound milestone |

Authored means: the table is the claim until a gate in a chip repo holds
it to a netlist pad or a scope capture, and the milestone report that
does so says which pins it covered.

## Commands

```bash
cargo test          # the contract held to itself: encodings pinned,
                    # tables closed, NROM's CIRAM pins per the solder
                    # options
cargo clippy --all-targets
```

The other half of the N0 gate lives in the dependent repositories:
ntsc-crt and 2c02 compile against these types with their existing
goldens unchanged, and under `MUTATE=1` the 2c02 harness reads the CHR
bus through `PpuPins::mutated_rd_for_proof`, which must send its P1
golden red.

## Licensing

MIT. This crate embeds no die data; the chips it describes carry their
own obligations in their own repositories.
