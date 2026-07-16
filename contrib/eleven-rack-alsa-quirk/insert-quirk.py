#!/usr/bin/env python3
"""Insert the Eleven Rack quirk entry into a kernel `sound/usb/quirks-table.h`.

`quirks-table.h` is an include-fragment: a bare list of `{ ... },` entries that
lands inside the `usb_audio_ids[]` array in `sound/usb/card.c`. We add our entry
just before the macro-teardown block at the very end of the entry list, anchored
on `#undef USB_DEVICE_VENDOR_SPEC` (present on all macro-era kernels, ~6.2+). If
that marker is absent we fall back to inserting after the last top-level entry
(a `},` at column 0).

Exit codes: 0 inserted (or already present), 2 usage/IO error, 3 unexpected file
shape (no insertion point found -- refuse rather than corrupt the table).
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

MARKER = "0x0dba, 0xb011"


def _audioformat(ifno: int, ep: int, channels: int, role: str) -> str:
    """One fixed-endpoint audioformat streaming entry (tab-indented, 3 levels)."""
    return (
        f"\t\t\t{{\n"
        f"\t\t\t\t/* {role}: EP {ep:#04x}, {channels}ch, 24-bit in S32_LE slots */\n"
        f"\t\t\t\tQUIRK_DATA_AUDIOFORMAT({ifno}) {{\n"
        f"\t\t\t\t\t.formats = SNDRV_PCM_FMTBIT_S32_LE,\n"
        f"\t\t\t\t\t.channels = {channels},\n"
        f"\t\t\t\t\t.fmt_bits = 24,\n"
        f"\t\t\t\t\t.iface = {ifno},\n"
        f"\t\t\t\t\t.altsetting = 1,\n"
        f"\t\t\t\t\t.altset_idx = 1,\n"
        f"\t\t\t\t\t.endpoint = {ep:#04x},\n"
        f"\t\t\t\t\t.ep_attr = USB_ENDPOINT_XFER_ISOC |\n"
        f"\t\t\t\t\t\t   USB_ENDPOINT_SYNC_ASYNC,\n"
        f"\t\t\t\t\t.rates = SNDRV_PCM_RATE_44100 |\n"
        f"\t\t\t\t\t\t SNDRV_PCM_RATE_48000 |\n"
        f"\t\t\t\t\t\t SNDRV_PCM_RATE_88200 |\n"
        f"\t\t\t\t\t\t SNDRV_PCM_RATE_96000,\n"
        f"\t\t\t\t\t.rate_min = 44100,\n"
        f"\t\t\t\t\t.rate_max = 96000,\n"
        f"\t\t\t\t\t.nr_rates = 4,\n"
        f"\t\t\t\t\t.rate_table = (unsigned int[]) {{\n"
        f"\t\t\t\t\t\t44100, 48000, 88200, 96000\n"
        f"\t\t\t\t\t}},\n"
        f"\t\t\t\t\t.clock = 0x81,\n"
        f"\t\t\t\t}},\n"
        f"\t\t\t}},\n"
    )


ENTRY = (
    "\t/*\n"
    "\t * Avid/Digidesign Eleven Rack: the audio function is standard UAC2 but\n"
    "\t * marked vendor-class (0xFF). The DFU interface (0) enumerates first and\n"
    "\t * becomes chip->ctrl_intf, so terminal-link + clock resolution for the\n"
    "\t * streaming interfaces (which follows ctrl_intf) misfires -- interfaces\n"
    "\t * 3/4 report 'bogus bTerminalLink' and no PCM is built. Describe the two\n"
    "\t * streams with fixed audioformats so the PCMs are created directly,\n"
    "\t * bypassing terminal parsing. Mixer + MIDI parse on their own interfaces.\n"
    "\t * See contrib/eleven-rack-alsa-quirk/ in the rackctl repo.\n"
    "\t */\n"
    "{\n"
    "\tUSB_DEVICE(0x0dba, 0xb011),\n"
    "\tQUIRK_DRIVER_INFO {\n"
    '\t\t.vendor_name = "Digidesign",\n'
    '\t\t.product_name = "Eleven Rack",\n'
    "\t\tQUIRK_DATA_COMPOSITE {\n"
    "\t\t\t{ QUIRK_DATA_IGNORE(0) },\n"
    "\t\t\t{ QUIRK_DATA_STANDARD_MIXER(1) },\n"
    "\t\t\t{ QUIRK_DATA_STANDARD_MIDI(2) },\n"
    + _audioformat(3, 0x03, 6, "playback: host -> unit")
    + _audioformat(4, 0x83, 8, "capture: unit -> host, implicit fb")
    + "\t\t\tQUIRK_COMPOSITE_END\n"
    "\t\t}\n"
    "\t}\n"
    "},\n"
)


def parse_args(argv: list[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("table", type=Path, help="path to sound/usb/quirks-table.h")
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="print where the entry would go; do not write the file",
    )
    return p.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    path: Path = args.table
    try:
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    except OSError as exc:
        print(f"error: cannot read {path}: {exc}", file=sys.stderr)
        return 2

    if any(MARKER in line for line in lines):
        print(f"already present ({MARKER}); nothing to do")
        return 0

    # Preferred anchor: the macro-teardown at the end of the entry list. Insert
    # right before it so the entry lands last, regardless of the last entry's form
    # (a literal `},` or a macro like QUIRK_RME_DIGIFACE(...)).
    insert_at: int | None = None
    for i, line in enumerate(lines):
        if line.startswith("#undef USB_DEVICE_VENDOR_SPEC"):
            insert_at = i
            break
    # Fallback: after the last top-level entry terminator (a `},` at column 0).
    if insert_at is None:
        for i, line in enumerate(lines):
            if line.rstrip("\n") == "},":
                insert_at = i + 1
    if insert_at is None:
        print(
            "error: found neither '#undef USB_DEVICE_VENDOR_SPEC' nor a top-level "
            "'},' entry -- is this really a quirks-table.h fragment?",
            file=sys.stderr,
        )
        return 3

    if args.dry_run:
        ctx = "".join(lines[max(0, insert_at - 2) : insert_at])
        print(f"would insert after line {insert_at}:\n{ctx}--- entry above goes here ---")
        return 0

    lines[insert_at:insert_at] = [ENTRY]
    try:
        path.write_text("".join(lines), encoding="utf-8")
    except OSError as exc:
        print(f"error: cannot write {path}: {exc}", file=sys.stderr)
        return 2
    print(f"inserted Eleven Rack quirk into {path} after line {insert_at}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
