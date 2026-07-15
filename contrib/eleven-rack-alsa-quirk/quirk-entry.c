/*
 * snd-usb-audio quirk entry for the Avid/Digidesign Eleven Rack (0dba:b011).
 *
 * This is NOT a standalone file -- it is the entry to add to the kernel's
 * `sound/usb/quirks-table.h`, inside the `usb_audio_ids[]` array, before the
 * terminating `{ }`. See README.adoc for how to insert, build and test it.
 *
 * Why this works (see docs/eleven-rack-audio-driver-scope.adoc "KEY FINDING"):
 * the Eleven's interfaces 1/3/4 are bit-for-bit USB Audio Class 2.0 -- standard
 * AC/AS subclasses, UAC2 protocol (0x20), standard clock/terminal descriptors,
 * standard control requests. The ONLY non-standard byte is bInterfaceClass=0xFF
 * (vendor) instead of 0x01 (audio), which is what stops snd-usb-audio binding.
 * This quirk forces the driver to claim those interfaces and parse them as
 * standard UAC2 -- no format reverse-engineering needed.
 *
 * Interface disposition (from the 419-byte config descriptor):
 *   0  DFU (class 0xFE)                -> IGNORE, leave to userspace dfu-util
 *   1  AudioControl  (vendor/0x01/0x20)-> STANDARD_MIXER: terminals, clocks, mixer
 *   2  MIDIStreaming (audio/0x03)      -> STANDARD_MIDI  (this is today's hw:2,0)
 *   3  AudioStreaming (vendor/0x02/0x20)-> STANDARD_AUDIO: playback, EP 0x03, 6ch S32_LE
 *   4  AudioStreaming (vendor/0x02/0x20)-> STANDARD_AUDIO: capture,  EP 0x83, 8ch S32_LE
 *
 * Format is 4-byte subslot (S32_LE); the converters are 24-bit (Avid spec), so
 * 24 significant bits ride in 32-bit slots. Rate is clock-programmable (UAC2
 * clock 0x81): 44.1 / 48 / 88.2 / 96 kHz, all within the 416 B packet. The
 * driver sets it via SAM_FREQ the normal way -- but note Avid's driver never
 * issues GET RANGE, so if the firmware does not answer it snd-usb-audio may see
 * only one rate (a fixed-rate quirk fixes that). Playback OUT (0x03) is async
 * and slaved to capture IN (0x83) via implicit feedback -- handled natively.
 */

/* --- Preferred form: modern QUIRK_DATA_* macros (kernels ~6.1+). --- */
{
	USB_DEVICE(0x0dba, 0xb011),
	QUIRK_DRIVER_INFO {
		.vendor_name = "Digidesign",
		.product_name = "Eleven Rack",
		QUIRK_DATA_COMPOSITE {
			{ QUIRK_DATA_IGNORE(0) },
			{ QUIRK_DATA_STANDARD_MIXER(1) },
			{ QUIRK_DATA_STANDARD_MIDI(2) },
			{ QUIRK_DATA_STANDARD_AUDIO(3) },
			{ QUIRK_DATA_STANDARD_AUDIO(4) },
			QUIRK_COMPOSITE_END
		}
	}
},

#if 0
/* --- Fallback form: explicit driver_info, compiles on any kernel that has
 *     the composite quirk (i.e. essentially all of them). Use this instead of
 *     the macro form above if your tree predates the QUIRK_DATA_* macros. --- */
{
	USB_DEVICE(0x0dba, 0xb011),
	.driver_info = (unsigned long) &(const struct snd_usb_audio_quirk) {
		.vendor_name = "Digidesign",
		.product_name = "Eleven Rack",
		.ifnum = QUIRK_ANY_INTERFACE,
		.type = QUIRK_COMPOSITE,
		.data = &(const struct snd_usb_audio_quirk[]) {
			{ .ifnum = 0, .type = QUIRK_IGNORE_INTERFACE },
			{ .ifnum = 1, .type = QUIRK_AUDIO_STANDARD_MIXER },
			{ .ifnum = 2, .type = QUIRK_MIDI_STANDARD_INTERFACE },
			{ .ifnum = 3, .type = QUIRK_AUDIO_STANDARD_INTERFACE },
			{ .ifnum = 4, .type = QUIRK_AUDIO_STANDARD_INTERFACE },
			{ .ifnum = -1 }
		}
	}
},
#endif
