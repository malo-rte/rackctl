# Tascam US-16x08 -- signal chain reference

A verified description of how audio flows through the US-16x08's built-in DSP
mixer, and how that maps onto the controls this project exposes. It exists so the
CLI's `topology` text, the control catalog's doc comments, and the future GUI all
reference one source instead of scattered assumptions.

## Sources

- **Reference Manual** D01247020B (TASCAM US-16x08), section *6 -- Using the
  Settings Panel*, the *OUTPUT SETTING page* (p.18), and *Block diagrams* /
  *Level diagrams* (p.31-32).
- **Linux kernel driver** `sound/usb/mixer_us16x08.c` (the `snd-usb-audio`
  US-16x08 quirk) -- authoritative for which controls exist and their types.
- The original `tascamgtk` app (now under `legacy/`) for the author's labels.

Each claim is tagged **[manual]**, **[driver]**, or **[inferred]**.

## Overview

The device is a 16-in / 8-out USB interface with an on-board BlackFin DSP that
runs a **monitor mixer**: the 16 inputs are processed per channel and summed into
a single **stereo master bus**. The computer's 8 USB playback streams
(`COMPUTER 1-8`) can be mixed into that bus and/or sent straight to outputs. Each
of the 8 physical line outs independently selects one source. **[manual]**

```
              +-------+   +-----------+   +------------+   +-------+   +-----+   +------------+   +----------+
 16 inputs -->| phase |-->| LCF + EQ  |-->| compressor |-->| fader |-->| pan |-->| MASTER L/R |-->|  route   |--> line out 1..8
              +-------+   +-----------+   +------------+   +-------+   +-----+   +------------+   +----------+
                          (mute is per channel)        per channel x16     summed      per output

 computer playback (COMPUTER 1..8)
     |- mixed into the MASTER bus  (Buss Out)
     '- or sent directly to a line out (route source COMPUTER n)
```

## Per-input-channel chain

In signal order: **[manual]** for the stages and their roles; **[inferred]** for
the exact EQ-vs-compressor ordering (see note).

| Stage | Control(s) in this project | Notes |
|-------|----------------------------|-------|
| Phase | `phase` | Reverse channel phase. **[manual]** |
| Low-cut filter (LCF) | *(none)* | Per-channel high-pass. Present on the device **[manual]** but **not exposed by the Linux driver** -- no ALSA control, so this project cannot set it. **[driver]** |
| EQ (4-band) | `eq-enable`, `eq-{low,midlow,midhigh,high}-volume`, `eq-{low,midlow,midhigh,high}-freq`, `eq-{midlow,midhigh}-q` | Gain +/-12 dB per band; mids have Q (0.25-16.0). Enabled by `eq-enable`. **[manual/driver]** |
| Compressor | `comp-enable`, `comp-threshold`, `comp-ratio`, `comp-attack`, `comp-release`, `comp-gain` | Threshold -32..0 dB, ratio 1.0:1..inf:1, attack 2-200 ms, release 10-1000 ms, make-up gain 0-20 dB. Meters expose INPUT/OUTPUT and **gain reduction (GR)**. **[manual]** |
| Mute | `mute` | **[manual/driver]** |
| Fader | `line-volume` | Channel level sent to the stereo bus (log dB curve). **[manual/driver]** |
| Pan | `pan` | Position into the stereo bus; center = -3 dB to both sides; hard L/R sends to one side only. **[manual]** |

**EQ vs compressor order note:** the manual presents EQ as channel module *1* and
the compressor as module *2*, and the compressor acts on the channel "input
volume" -- consistent with **phase -> LCF/EQ -> compressor -> mute -> fader -> pan**.
The manual's *Block diagrams* (p.31) depict the analog I/O path (mic/line ->
preamp -> A/D), not the internal DSP order, so the EQ-before-compressor ordering
is **strongly implied but not stated as an explicit DSP block**. **[inferred]**

## Master bus

The 16 channels are summed into a stereo master. `master-volume` sets the master
output level and `master-mute` mutes it; the master meters show the mixed signal.
**[manual/driver]** The master itself can be compressed (the compressor note
mentions a "master" stereo signal). **[manual]**

## Output routing (the 8 line outputs)

Each `LINE OUT 1-8` selects **one** source via the `route` control (per output,
indices 0-7). Options: **[manual]**

- `MASTER L` / `MASTER R` -- the **Stereo BUS**: "the signals input through each
  input jack and by USB output from the computer are mixed and output in stereo."
- `COMPUTER 1-8` (our `Output 1..8`) -- the **Computer BUS**: DAW playback streams
  sent directly to a jack.

`LINE OUT 1`/`2` are also mirrored to the PHONES jack. **[manual]** The 16 inputs
are **not** routed to outputs individually -- only the 8 outputs are routed.

## Global switches

- **`buss-out`** ("Buss Out Switch" **[driver]**; "Computer out to Stereo BUS" in
  the legacy UI) -- folds the computer/DAW playback into the stereo master bus, so
  it is monitored together with the live inputs. Confirmed by the manual's Stereo
  BUS definition (inputs + USB computer output mixed). **[manual/inferred]**
- **`dsp-bypass`** ("DSP Bypass Switch" **[driver]**) -- bypasses the channel DSP
  (EQ/compressor) for dry monitoring. The manual's Settings-Panel section
  documents per-channel `eq-enable`/`comp-enable` rather than a single global
  bypass, so the precise scope of this global switch is **not manual-verified**;
  the legacy app treated it as a global/true bypass. **[inferred]**

## Not exposed / out of scope

- **Low-cut filter (LCF)** -- on the device, but the kernel driver registers no
  control for it. **[driver]**
- **Stereo LINK** -- combining two adjacent channels into a stereo pair is a
  mixer-software grouping (it ganged the GTK widgets); the driver has no link
  control, so on the hardware each channel is independent. This is GUI state for
  the future GUI, not a hardware control. **[driver/manual]**
- **Mic preamp gain, phantom power, sample rate** -- front-panel / interface
  functions, not DSP-mixer controls; outside this project's control surface.

## Open items to confirm on hardware

- The EQ-before-compressor ordering (read the p.31/level diagrams more closely or
  verify audibly).
- The exact behaviour of `dsp-bypass` (does it bypass per channel, or the whole
  DSP path including the master?).
