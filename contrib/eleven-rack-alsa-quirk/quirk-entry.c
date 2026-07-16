/*
 * snd-usb-audio quirk entry for the Avid/Digidesign Eleven Rack (0dba:b011).
 *
 * Reference copy of the entry added to `sound/usb/quirks-table.h` by
 * eleven-rack-uac2-quirk.patch. Not a standalone file. See README.adoc.
 *
 * Background (docs/eleven-rack-audio-driver-scope.adoc "KEY FINDING"): the
 * unit's audio function is standard USB Audio Class 2.0, but its interfaces are
 * marked vendor-class (bInterfaceClass 0xFF) so snd-usb-audio does not bind them.
 *
 * v1 attempt (QUIRK_DATA_STANDARD_AUDIO on interfaces 3/4) was tested on real
 * hardware (kernel 6.18.38): the card, mixer, and MIDI came up, but BOTH
 * streaming interfaces were rejected:
 *     usb 5-1: 3:1 : bogus bTerminalLink 0
 *     usb 5-1: 4:1 : bogus bTerminalLink 1
 *     usb 5-1: 64:1: cannot get min/max values for control 7..14 (id 64)
 * so no PCM was registered.
 *
 * Root cause (confirmed against 6.18 source): the DFU interface (0) enumerates
 * first and becomes chip->ctrl_intf. The composite quirk's STANDARD_AUDIO path
 * calls snd_usb_parse_audio_interface() directly without registering a
 * per-interface control link, so snd_usb_find_ctrl_interface() falls back to
 * that DFU interface for interfaces 3/4. Terminal-link AND clock resolution then
 * search the DFU interface (no audio descriptors) and fail. The mixer's min/max
 * GETs miss for the same reason. (The card.c comment at the chip->ctrl_intf
 * assignment even says "we might need a more specific check here in the future".)
 *
 * Fix (this entry): describe the two streams with fixed audioformats
 * (QUIRK_AUDIO_FIXED_ENDPOINT). create_fixed_stream_quirk() builds the PCMs
 * directly -- no terminal parsing -- and ignores rate-init failures, so the PCMs
 * register regardless of the ctrl_intf issue; generic implicit-feedback
 * detection links the async playback EP to the capture interface.
 *
 * Interface disposition:
 *   0  DFU (0xFE)                 -> IGNORE (left to userspace dfu-util)
 *   1  AudioControl  (0xFF/01/20) -> STANDARD_MIXER (mixer + clock descriptors)
 *   2  MIDIStreaming (0x01/03)    -> STANDARD_MIDI  (hw:X,0, unchanged)
 *   3  AudioStreaming(0xFF/02/20) -> fixed audioformat: playback EP 0x03, 6ch
 *   4  AudioStreaming(0xFF/02/20) -> fixed audioformat: capture  EP 0x83, 8ch
 *
 * Format is S32_LE (4-byte slots) carrying 24-bit audio (Avid spec).
 * Rates 44.1/48/88.2/96 kHz. NOTE: rate SET_CUR also goes through ctrl_intf, so
 * until the ctrl_intf issue is fixed upstream the on-device rate may not follow
 * the host request -- set the rate on the unit's front panel to be sure (or it
 * stays at its 44.1 kHz power-on default).
 */

{
	USB_DEVICE(0x0dba, 0xb011),
	QUIRK_DRIVER_INFO {
		.vendor_name = "Digidesign",
		.product_name = "Eleven Rack",
		QUIRK_DATA_COMPOSITE {
			{ QUIRK_DATA_IGNORE(0) },
			{ QUIRK_DATA_STANDARD_MIXER(1) },
			{ QUIRK_DATA_STANDARD_MIDI(2) },
			{
				/* playback: host -> unit, EP 0x03, 6ch */
				QUIRK_DATA_AUDIOFORMAT(3) {
					.formats = SNDRV_PCM_FMTBIT_S32_LE,
					.channels = 6,
					.fmt_bits = 24,
					.iface = 3,
					.altsetting = 1,
					.altset_idx = 1,
					.endpoint = 0x03,
					.ep_attr = USB_ENDPOINT_XFER_ISOC |
						   USB_ENDPOINT_SYNC_ASYNC,
					.rates = SNDRV_PCM_RATE_44100 |
						 SNDRV_PCM_RATE_48000 |
						 SNDRV_PCM_RATE_88200 |
						 SNDRV_PCM_RATE_96000,
					.rate_min = 44100,
					.rate_max = 96000,
					.nr_rates = 4,
					.rate_table = (unsigned int[]) {
						44100, 48000, 88200, 96000
					},
					.clock = 0x81,
				},
			},
			{
				/* capture: unit -> host, EP 0x83, 8ch, implicit fb */
				QUIRK_DATA_AUDIOFORMAT(4) {
					.formats = SNDRV_PCM_FMTBIT_S32_LE,
					.channels = 8,
					.fmt_bits = 24,
					.iface = 4,
					.altsetting = 1,
					.altset_idx = 1,
					.endpoint = 0x83,
					.ep_attr = USB_ENDPOINT_XFER_ISOC |
						   USB_ENDPOINT_SYNC_ASYNC,
					.rates = SNDRV_PCM_RATE_44100 |
						 SNDRV_PCM_RATE_48000 |
						 SNDRV_PCM_RATE_88200 |
						 SNDRV_PCM_RATE_96000,
					.rate_min = 44100,
					.rate_max = 96000,
					.nr_rates = 4,
					.rate_table = (unsigned int[]) {
						44100, 48000, 88200, 96000
					},
					.clock = 0x81,
				},
			},
			QUIRK_COMPOSITE_END
		}
	}
},
