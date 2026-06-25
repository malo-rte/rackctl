{
  description = "Tascam US-16x08 DSP mixer control tools — tascamctl (CLI) and tascam-mixer (GUI)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;

        # One source of truth for the version: the workspace manifest, the same
        # field `bump-version` stamps. No need to touch this file at release time.
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;

        # Libraries eframe/winit/glow need. They are needed at build time (the
        # `-sys` crates link or probe them) and the GUI also dlopens several at
        # run time. The set mirrors the CI's apt packages:
        #   libasound2-dev   -> alsa-lib
        #   libgl1-mesa-dev  -> libGL
        #   libxkbcommon-dev -> libxkbcommon
        #   libwayland-dev   -> wayland
        #   libxcb*-dev      -> xorg.libxcb
        guiLibs = with pkgs; [
          libGL
          libxkbcommon
          wayland
          xorg.libxcb
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];

        tascam-mixer = pkgs.rustPlatform.buildRustPackage {
          pname = "tascam-mixer";
          inherit version;
          src = self;

          # Build against the committed lockfile — no network, reproducible.
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.makeWrapper
          ];
          buildInputs = [ pkgs.alsa-lib ] ++ guiLibs;

          # winit/glow load Wayland, GL and xkbcommon with dlopen at run time,
          # which does not consult the binary's RUNPATH — so put them on the GUI's
          # library path. The CLI only needs libasound, picked up via RUNPATH.
          postInstall = ''
            wrapProgram $out/bin/tascam-mixer \
              --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath guiLibs}
          '';

          meta = {
            description = "Control tools for the Tascam US-16x08 USB DSP mixer (CLI + GUI)";
            homepage = "https://github.com/malo-rte/tascam-mixer";
            license = lib.licenses.mit;
            platforms = lib.platforms.linux;
            mainProgram = "tascam-mixer";
          };
        };
      in
      {
        packages = {
          default = tascam-mixer;
          tascam-mixer = tascam-mixer;
        };

        # `nix run .#tascamctl` / `nix run .#tascam-mixer`; bare `nix run` is the GUI.
        apps = {
          default = self.apps.${system}.tascam-mixer;
          tascam-mixer = {
            type = "app";
            program = "${tascam-mixer}/bin/tascam-mixer";
          };
          tascamctl = {
            type = "app";
            program = "${tascam-mixer}/bin/tascamctl";
          };
        };

        # `nix develop` — the build inputs plus the Rust toolchain and the gate's
        # tools. LD_LIBRARY_PATH lets a plain `cargo run -p tascam-gui` find the
        # GUI's runtime libraries.
        devShells.default = pkgs.mkShell {
          inputsFrom = [ tascam-mixer ];
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
          ];
          LD_LIBRARY_PATH = lib.makeLibraryPath guiLibs;
        };
      }
    );
}
