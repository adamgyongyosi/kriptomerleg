{
  description = "Kriptomérleg – magyar kriptó-SZJA kalkulátor (Tauri)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (pkgs: rec {
        kriptomerleg = pkgs.callPackage ./nix/package.nix { };
        default = kriptomerleg;
      });

      apps = forAll (pkgs: {
        default = {
          type = "app";
          program = nixpkgs.lib.getExe self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          packages = [
            pkgs.cargo-tauri
            pkgs.rust-analyzer
          ];
          # `npm run tauri dev` nem a wrapGAppsHook3-mal becsomagolt binárist futtatja,
          # ezért a GTK fájlválasztó sémáit (org.gtk.Settings.FileChooser) itt kell elérhetővé tenni.
          shellHook = ''
            export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
          '';
        };
      });

      overlays.default = final: _prev: {
        kriptomerleg = final.callPackage ./nix/package.nix { };
      };
    };
}
