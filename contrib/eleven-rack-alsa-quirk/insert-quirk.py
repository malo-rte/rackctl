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
ENTRY = """\t/*
\t * Avid/Digidesign Eleven Rack: UAC2 in all but bInterfaceClass (0xFF).
\t * Force snd-usb-audio to claim + parse the vendor-class audio interfaces
\t * as standard UAC2. See contrib/eleven-rack-alsa-quirk/ in the rackctl repo.
\t */
{
\tUSB_DEVICE(0x0dba, 0xb011),
\tQUIRK_DRIVER_INFO {
\t\t.vendor_name = "Digidesign",
\t\t.product_name = "Eleven Rack",
\t\tQUIRK_DATA_COMPOSITE {
\t\t\t{ QUIRK_DATA_IGNORE(0) },
\t\t\t{ QUIRK_DATA_STANDARD_MIXER(1) },
\t\t\t{ QUIRK_DATA_STANDARD_MIDI(2) },
\t\t\t{ QUIRK_DATA_STANDARD_AUDIO(3) },
\t\t\t{ QUIRK_DATA_STANDARD_AUDIO(4) },
\t\t\tQUIRK_COMPOSITE_END
\t\t}
\t}
},
"""


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
