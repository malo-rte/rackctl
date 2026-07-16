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

# NOTE: this regenerates ONLY the quirks-table.h hunk of the driver. The full,
# working fix (eleven-rack-uac2-quirk.patch) also patches card.c, format.c,
# pcm.c and quirks.c -- this entry alone will not produce a working PCM.
ENTRY = (
    "\t/*\n"
    "\t * Avid/Digidesign Eleven Rack: standard UAC2 marked vendor-class (0xFF).\n"
    "\t * Force standard parsing of the audio interfaces (needs the companion\n"
    "\t * card.c/format.c/pcm.c/quirks.c changes -- see the .patch).\n"
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
    "\t\t\t{ QUIRK_DATA_STANDARD_AUDIO(3) },\n"
    "\t\t\t{ QUIRK_DATA_STANDARD_AUDIO(4) },\n"
    "\t\t\tQUIRK_COMPOSITE_END\n"
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
