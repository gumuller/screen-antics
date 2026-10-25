# Original Windows buffer evidence

Target: `original/SCRANTIC.EXE`, SHA-256
`f0552c281a2a9b367ad5f41402a907a3c99e73ad0a221ba14d063e31cff594ac`.

Read-only static investigation, 2026-10-24. REA's native opening route rejected
the Windows NE executable; no active native session or original runtime trace
was available. Evidence below comes from GNU `objdump -m i8086` on extracted NE
segments and resolving far-call fixups in the original executable's relocation
tables. Addresses are hexadecimal `segment:offset`, not linear file offsets.
Segment 11 begins at file `0x14400`, segment 12 at `0x15200`; automatic data
segment 14 begins at `0x17e00`.

## Get/put instructions are no-ops in this build

**Observed:** exact opcode `4214` maps through the 47-word opcode table at
`12:12bd` and corresponding handler table at `12:131b` to `12:0860`.
That handler tests `DS:46d8`; both branches jump directly to `12:1056`
(`12:0867` and `12:086a`). It performs no capture.

**Observed:** the main decoder routes A-family instructions through `12:0144`
(call at `12:0679`). Entry 15 (zero based) of its 20-word opcode table at
`12:03bc` is exact opcode `A601`; the corresponding handler table at `12:03e4`
selects `12:02d8`. That handler tests `DS:46d8`; both branches jump directly to
`12:03b1` (`12:02df` and `12:02e2`). It performs no restore.

**Inference:** treating these two instructions as capture/restore introduces
behavior absent from this shipped build. However, removing these operations
alone did not remove the stationary ship in the indexed regression. The
separate A054/A064 capture pair below explains the missing cleanup operation.

## Buffer numbers and stores

**Observed:** mapper `12:14e0–150f` returns:

| Script number | Native handle | Identified use |
| --- | --- | --- |
| 0 | `DS:1eb4` | Display |
| 1, or another default value | `DS:1eb2` | Composition |
| 2 | `DS:1eb0` | Stored backing |

Exact `B606` handler `12:0eb4` maps argument 4 (local `BP-0x0c`) into
source `DS:1eb6` at `12:0ec7`, then argument 5 (`BP-0x0e`) into destination
`DS:1eb8` at `12:0ed3`. Far call `12:0ee2` relocates to `8:06ce`, an opaque
rectangle blitter. Its `8:079e–07c6` path supplies source and destination handles
to the underlying raster operation with `SRCCOPY` (`0x00cc0020`). After the
copy, `12:0eea–0ef3` sets source to display and destination to composition.

Exact `4204` store-area handler `12:0e55` calls `11:0b74` at `12:0e75`.
The callee temporarily sets source=composition (`DS:1eb2`) and
destination=stored (`DS:1eb0`) at `11:0b94–0b9d`, calls `8:06ce` at
`11:0bac`, then restores the former source/destination handles.

## SCR loading initializes stored backing with the picture

**Observed:** the interpreter defaults source and destination to composition
at `12:048e–0494`. Exact `F01F` handler `12:06c5` calls `6:1813` at
`12:06d2`; that wrapper calls the SCR loader `7:16d8` at `6:1820`.
After the load, `12:06d8` jumps to `12:0e4a`, whose far call targets
`11:0b31`.

`11:0b31` marks `DS:4640=1`, then calls `11:0b74` at `11:0b54` with the
full-screen rectangle. Consequently the loaded composition is copied into
stored backing. Call `11:0b68` targets `13:06ac`, retiring rectangle records
for stored→composition. Stored backing is therefore initialized with the SCR
picture; it is not a freshly zero-filled transparent overlay in this build.

At the frame-end marker, `12:1212` checks `DS:4640`. Presentation at
`12:1255 → 13:007e` normally uses composition as source and display as
destination; `DS:287a` selects stored backing as the alternative source.

## Dirty restoration, with ordering limitation

**Observed:** `13:03d3` saves the active source/destination and iterates buffer
pairs from the data-segment table at `DS:287e` when its argument is zero.
It calls `13:05f9` at `13:040e`. That function locates a matching rectangle
list, sets source from entry `+8` and destination from entry `+0x0a`
(`13:0629–0632`), and restores operation-1 rectangles using the opaque
blitter (`13:065e → 8:06ce`). It subsequently retires those rectangle records.
The pair table includes stored (`DS:1eb0`) → composition (`DS:1eb2`).
The far caller found for `13:03d3` is `4:088e`, passing argument zero.

**Limitation:** this establishes the dirty-rectangle restoration mechanism and
its backing-to-composition pair. The entire scheduler's ordering around each
TTM frame has not been reconstructed, so this is not proof of a full-screen
copy at every frame start. A full opaque reconstruction from stored backing in
the Rust renderer is a simplifying implementation choice, not a verbatim
recovery of the native dirty-rectangle algorithm.

## 2012 is not a buffer-clear instruction

**Observed:** exact `2012` maps to `12:0f23`. On initial execution it sets
`DS:46d6=1`; far call `12:0f36` targets `12:197c`, which contains only an
empty function's prologue and epilogue. There is no immediate buffer clearing,
copy, or stored-area reset in that handler.

Contrary to the earlier hypothesis that the flag was never read,
`12:07c5` reads `DS:46d6` into `DS:4106` before loading a BMP resource.
Call `12:07ce → 6:17fa → 7:124a` loads that resource, and
`12:07f8–07fd` clears both flags afterward. `DS:4106` gates additional resource
processing at `9:018a`. Bitmap/mask preparation is the inferred purpose;
the full processing algorithm was not recovered in this pass. Implementing
2012 as an immediate clear contradicts the observed handler.

These conclusions apply to this executable hash. They do not establish the
behavior of DOS versions or the general DGDS/ScummVM interpreter.

## A054/A064 capture and restore, rather than animated wipes

**Observed:** A-family fallback accepts A014 through A094 at `12:02f6–0304`
and calls `12:1510` at `12:0351`. That function's exact opcode table
`12:1900` and handler table `12:1910` map A054 to `12:1747` and A064 to
`12:1801`. Neither selected handler contains a wipe loop. Other entries in
the same function do contain loops, but they are different opcodes.

A054 stores its x/y/width/height in the current TTM resource context
(`DS:2638`, segment in `DS:263a`) at offsets `+0x6c`, `+0x84`, `+0x9c`,
and `+0xb4` (`12:1751–176c`). It uses the fixed capture pointer at context
`+0x3c` (`12:1771–177b`), frees an existing capture, allocates a new one,
and calls `7:01c4` at `12:17f7`. This capture routine reads source handle
`DS:1eb6` (`7:01d9`, `7:0205`), copies the rectangle using SRCCOPY
(`7:020f–0217`), and serializes the captured bitmap. The main instruction
loop establishes composition as the source at `12:048e–0494` for each
instruction, so A054 captures composition.

A064 explicitly sets destination to composition at `12:180b–180e`, obtains
the same context `+0x3c` pointer, and exits if it is absent. It uses the
**saved** x/y/width/height, not A064's supplied rectangle
(`12:1830–1843`). Call `12:1851` relocates to `7:0250`, which decodes the
saved bitmap and copies it opaquely into destination `DS:1eb8`
(`7:02a3–02be`, SRCCOPY). The enclosing function restores its original
destination handle at `12:18f1–18f4`.

The capture uses fixed slot zero per TTM resource context. It does not consult
the get/put selector set by `1121` (`12:0853`, current track member `+0x16`).
It is therefore separate from the unsupported 4214/A601 instruction pair.

**Inference tied to GJGULIVR:** the A054 at arrival captures the clean
composition before ship frame 9 is drawn and stored. The A064 at departure
restores that clean image into composition, allowing the subsequent 4204 to
replace the persistent stored ship with clean pixels. An interpreter that
treats A050/A060 solely as visual presentation events misses this state
mutation and retains ship frame 9 in backing even after correcting B606 and
disabling 4214/A601. This supersedes the initial diagnosis that those two
get/put no-ops alone would resolve the stationary ship.
