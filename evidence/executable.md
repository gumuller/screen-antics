# SCRANTIC.EXE static investigation

The supplied `original/SCRANTIC.EXE` and `SCRANTIC.SCR` are byte-identical,
295,952-byte Microsoft Windows NE executables. The NE header is at file offset
`0x250`. The executable has thirteen code segments and one 18,368-byte data
segment; resource alignment is 512 bytes. This investigation used a small
Python NE table parser, `strings -a`, and GNU `objdump -D -b binary -m i8086`
on individual code segments. No claims here require running the Windows code.

## Observed entry points and timing

The nonresident export-name table supplies these exact locations:

| Export | NE segment:offset |
| --- | --- |
| ScreenSaverProc | 2:0x0fcd |
| ScreenSaverConfigureDialog | 2:0x034f |
| PasswordDialog | 5:0x026e |
| ChangePasswordDialog | 2:0x0a2d |
| AdsControlDialog | 5:0x082a |

At segment 2 offset `0x1029`, the WM_CREATE branch calls initialization, then
pushes HWND, timer ID 1, interval `0x32` (50), and a null callback to the far
call at `0x103a`. The corresponding NE relocation resolves to USER ordinal 10,
the Win16 SetTimer API. Thus the requested Windows timer cadence is 50 ms,
or 20 ticks per second. This is a requested cadence, not a measured frame rate.
Message `0x113` (WM_TIMER) dispatches to `2:0x125a`; it calls a far engine
routine and an optional function pointer stored at DS:0x2d90.

## Observed resources and configuration

The data segment begins at file offset `0x17e00`. Relevant literal strings:

- `JOHNCAST.PAL` at DS:0x024a / file 0x1804a.
- `INTRO.SCR` at DS:0x0257 / file 0x18057.
- `OCEAN00.SCR` at DS:0x028c / file 0x1808c; also OCEAN01.SCR,
  OCEAN02.SCR, NIGHT.SCR, HOLIDAY.BMP, BACKGRND.BMP, MRAFT.BMP.
- ACTIVITY.ADS at DS:0x1681 / file 0x19481; also BUILDING.ADS, FISHING.ADS,
  JOHNNY.ADS, MARY.ADS, MISCGAG.ADS, STAND.ADS, SUZY.ADS, VISITOR.ADS,
  WALKSTUF.ADS. These names are evidence of modules, not proof of their opcode
  semantics.
- `WAVESFX%d` at DS:0x1d58 / file 0x19b58.
- `JOHNWALK.BMP`, `WILLY.FON`, `Willy Beamish Dialog`, `Screen Antics I`,
  `Dynamix, JTP - Jeff Tunnell Productions`, `New Scene`, `Start of day`.
- Recognized format strings BMP:INF:, BMP:BIN:, BMP:VGA:, BMP:AMG:,
  BMP:MTX:, SCR:DIM:, SCR:BIN:, SCR:VGA:, SCR:AMG:, PAL:VGA:,
  ADS:RES:, ADS:SCR:.

The supplied INI has section ScreenSaver.ScreenAntics, SourceDir=C:\\,
Background=Clouds=Waves=Sounds=Introduction=1, blank Password, NumDays=1,
CurrentYear=2021, CurrentMonth=5, CurrentDay=10, StartTime=900. The executable
contains all these configuration names. `%2d:%02d %cm` is also present; a clock
display is a reasonable inference, not a runtime observation.

## Original audio, recovered exactly

NE resource type `0xf005` contains 23 complete RIFF WAVE files with resource
IDs 1 through 23. All are PCM format 1, mono, 11,025 samples/second,
8 bits/sample, block alignment 1. Their combined sample data is 167,534 bytes.
The resource byte lengths include NE alignment padding; the RIFF and data
chunk sizes identify the actual audio bytes.

The RT_NAMETABLE resource at file `0x1ca00` maps numeric IDs to names in
lexicographic order. Consequently resource ID is not the WAVESFX suffix:

| Numeric resource ID | WAVESFX suffix |
| --- | --- |
| 1..9 | 1, 10, 12, 14, 15, 16, 17, 18, 19 |
| 10..16 | 2, 20, 21, 22, 23, 24, 25 |
| 17..23 | 3, 4, 5, 6, 7, 8, 9 |

No acoustic labels or ocean/gag assignments were inferred from the names.
`src/audio.rs` reads these NE resources from the supplied EXE at runtime,
validates their PCM format, and returns only original sample data. Its
`index_for_sfx` helper implements the verified name mapping. A standalone
Rust smoke check decoded all 23 waves, checked the total sample byte count,
and confirmed clean errors for three truncated/invalid inputs. It does not
use a Windows emulator or external audio conversion tool.

## Remaining limits

This pass does not establish complete ADS opcode behavior, original actor
placement, randomized gag scheduling, holiday triggers, or full day/night
timing. These require script/resource decoding or original runtime evidence.
The supplied 2021 file timestamps do not establish the original release date.
