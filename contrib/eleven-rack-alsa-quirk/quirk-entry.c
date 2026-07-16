/*
 * The quirks-table.h portion of the Eleven Rack driver (0dba:b011).
 *
 * This is ONE of five changes in eleven-rack-uac2-quirk.patch -- the full,
 * working driver also patches card.c (ctrl_intf selection), format.c (fixed
 * rate-list fallback for the missing SAM_FREQ RANGE), pcm.c (non-fatal PITCH
 * enable), and quirks.c (QUIRK_FLAG_IGNORE_CTL_ERROR for the device). See
 * README.adoc. This entry alone is not sufficient.
 *
 * It forces snd-usb-audio to parse the vendor-class (0xFF) audio interfaces as
 * the standard UAC2 they actually are: mixer on interface 1, USB-MIDI on 2,
 * streaming (playback EP 0x03 / capture EP 0x83) on 3/4. Implicit feedback,
 * terminals, and the clock are then set up from the descriptors by the normal
 * parser (given the card.c ctrl_intf fix).
 */

{
	USB_DEVICE(0x0dba, 0xb011),
	QUIRK_DRIVER_INFO {
		.vendor_name = "Digidesign",
		.product_name = "Eleven Rack",
		QUIRK_DATA_COMPOSITE {
			{ QUIRK_DATA_IGNORE(0) },          /* DFU */
			{ QUIRK_DATA_STANDARD_MIXER(1) },  /* AudioControl */
			{ QUIRK_DATA_STANDARD_MIDI(2) },   /* MIDIStreaming */
			{ QUIRK_DATA_STANDARD_AUDIO(3) },  /* playback, 6ch */
			{ QUIRK_DATA_STANDARD_AUDIO(4) },  /* capture, 8ch */
			QUIRK_COMPOSITE_END
		}
	}
},
