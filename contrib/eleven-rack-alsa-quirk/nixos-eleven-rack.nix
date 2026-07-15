# NixOS module: build the kernel with the Eleven Rack UAC2 audio quirk.
#
# Import it from your configuration.nix / flake NixOS module:
#
#     imports = [ ./contrib/eleven-rack-alsa-quirk/nixos-eleven-rack.nix ];
#
# This pins the kernel to 6.18 (the base our patch targets) and applies the
# quirk via boot.kernelPatches. It forces a full kernel rebuild the first time.
#
# After a `nixos-rebuild switch` and reboot, the Eleven Rack (0dba:b011) shows
# up as a normal ALSA card: 8-channel capture / 6-channel playback, 32-bit,
# 44.1 kHz (and up, per its programmable clock). Check with `aplay -l` /
# `arecord -l`. MIDI keeps working in parallel (rackctl-eleven on hw:X,0).
#
# See ./README.adoc for background and the on-hardware verification checklist,
# and ../../docs/eleven-rack-audio-driver-scope.adoc for the full USB analysis.

{ pkgs, lib, ... }:

{
  # Pin to the 6.18 series the patch is generated against. If this attribute is
  # missing on your nixpkgs, use `pkgs.linuxKernel.packages.linux_6_18` instead.
  boot.kernelPackages = pkgs.linuxPackages_6_18;

  boot.kernelPatches = [
    {
      name = "eleven-rack-uac2-quirk";
      patch = ./eleven-rack-uac2-quirk.patch;
    }
  ];

  # snd-usb-audio is already in the standard kernel; nothing else to enable.
  # (No udev rule needed -- ALSA owns the PCM device nodes once the quirk binds.)
}
