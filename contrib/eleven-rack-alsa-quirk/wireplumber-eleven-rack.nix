# NixOS module: WirePlumber tuning for the Eleven Rack (userspace companion to
# the kernel quirk in nixos-eleven-rack.nix). Import alongside it:
#
#     imports = [
#       ./contrib/eleven-rack-alsa-quirk/nixos-eleven-rack.nix       # kernel patch
#       ./contrib/eleven-rack-alsa-quirk/wireplumber-eleven-rack.nix # this
#     ];
#
# See wireplumber-eleven-rack.conf for the rationale and the channel map.
{ ... }:
{
  services.pipewire.wireplumber.extraConfig."51-eleven-rack" = {
    "monitor.alsa.rules" = [
      {
        matches = [ { "node.name" = "~alsa_output.usb-Digidesign_Eleven_Rack.*"; } ];
        actions.update-props = {
          "session.suspend-timeout-seconds" = 0;
          "audio.position" = [ "FL" "FR" "AUX2" "AUX3" "AUX4" "AUX5" ];
        };
      }
      {
        matches = [ { "node.name" = "~alsa_input.usb-Digidesign_Eleven_Rack.*"; } ];
        actions.update-props = {
          "session.suspend-timeout-seconds" = 0;
          "audio.position" = [ "AUX0" "AUX1" "AUX2" "AUX3" "AUX4" "AUX5" "AUX6" "AUX7" ];
        };
      }
    ];
  };
}
