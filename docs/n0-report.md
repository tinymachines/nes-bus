# N0 report: the contract exists, and two repositories hold their goldens through it

Run stamp: 2026-09-02, rustc 1.97.1. The console sketch (v0.2, in the
6502 repository until the `nes` repo exists) put N0 first: `nes-bus` as
its own repository, the frame types every chip crate speaks, and a gate
that is not a compile check but a golden check.

## What closed

- **The crate**: `FrameParity`, the NES frame constants and `DotFrame`
  (moved from ntsc-crt, which re-exports them); `PpuPins`, `CpuPins`
  and the two 40-pin DIP tables; `CartEdge` and the 72-pin table; the
  `Cartridge` trait with NROM; `AudioSamples`. No dependencies. Seven
  tests hold it to itself (encodings pinned by value, tables closed
  and positionally complete, NROM's CIRAM pins per the solder
  options, the /RD mutation flipping exactly one pin).
- **ntsc-crt adopts** (its `051ef9d`, pinned tag `v0.2.2`): the moved
  types became re-exports and nothing else changed. Its whole suite
  (58 tests) and its 54 self-counted doc claims stay green.
- **2c02 adopts** (its `58dc408`): `DotFrame` is consumed from here,
  and the harness's CHR bus service reads the chip through a `PpuPins`
  frame extracted every half-step. The P1 node golden (3,624 states,
  the whole register program and 3,000 rendering half-steps) replays
  green through the frame with `REQUIRE_GOLDEN_P1=1`, all five
  workspace tests passing, goldens byte-identical to the P1 close.
- **The mutation gate, both directions**: `MUTATE=rd` routes the CHR
  service through `PpuPins::mutated_rd_for_proof` (the contract lying
  about one pin's polarity) and the P1 replay goes red; `MUTATE=1`
  (the CHR byte off by one bit) still goes red at `_db0`, state 623,
  the same divergence the P1 report stamped, proving the refactored
  path kept the old proof alive.

## Decisions made en route

- **Analog pins are `Option<f32>`** (`vout`, `ad1`, `ad2`): a digital
  extraction cannot sample them, and filling them with a number would
  launder a fake reading into a frame. `None` means not sampled; a
  consumer that needs one must refuse `None` by name. This was changed
  before any consumer existed (v0.1.0 to v0.1.1).
- **One revision of the contract per build.** Every repository pins
  the same `nes-bus` tag, because two git sources of the same crate
  are two incompatible `DotFrame` types in one build. The bump
  sequence is: tag here, bump ntsc-crt and tag it, bump 2c02's both
  pins together.
- **Frames are instantaneous.** No half-step counter in the structs;
  trace formats and comparison drivers belong to each chip repo's own
  golden machinery, the way `v6502-pins` wraps its frames in a
  `Trace`.

## Authored, awaiting their gates

- `CpuPins` and `CartEdge` have no consumer yet; their first gates are
  N1 (the 2A03 pin lockstep) and N4/N5 (the glue and the console).
  Until then the DIP and edge tables are claims from the nesdev wiki,
  dated in the source.
- The `ad`/`a_hi` extraction in the 2c02 harness reads the internal
  address bus as the pad view, faithful to the reference's own
  handleChrBus; the full AD0..7 pad mux (address phase versus data
  phase) is the console's problem at N5, and the harness says so where
  it does it.
- The sketch wants a `check-self-counts.py` row per milestone; this
  repository does not carry the tool yet. The claims above quote test
  counts from the runs they describe, and the tool arrives when the
  repo has enough prose to drift.
