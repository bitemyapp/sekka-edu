{
  description = "Sekka Edu: educational software for Sekka, on Aizome's kit";

  # Aizome's kit and Sukumo come from Sekka (desktop/aizome), at a revision
  # named here, with Sekka's Nixpkgs. Move it forward to build against a
  # newer kit (DEVELOPERS.md).
  inputs.sekka.url = "github:bitemyapp/sekka/cd82f3c1ab8c06dd1fdc70ec6f49c37ba11c089c";
  inputs.nixpkgs.follows = "sekka/nixpkgs";

  outputs =
    {
      self,
      nixpkgs,
      sekka,
    }:
    let
      systems = [ "x86_64-linux" ];
      each = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      # The kit's crates and Sukumo's, with the workspace manifests they
      # inherit from, as Sekka gives them to Uta (its nix/uta.nix).
      aizomeFor =
        pkgs:
        pkgs.runCommand "aizome-kit-source" { } ''
          a=${sekka}/desktop/aizome
          mkdir -p $out/crates $out/sukumo
          cp $a/Cargo.toml $out/
          for c in aizome-kit aizome-theme aizome-config aizome-shell-proto; do
            cp -r $a/crates/$c $out/crates/
          done
          cp $a/sukumo/Cargo.toml $out/sukumo/
          cp -r $a/sukumo/crates $out/sukumo/
        '';
      # The type Aizome's session draws with.
      fontsFor =
        pkgs:
        pkgs.callPackage "${sekka}/desktop/aizome/nix/aizome-fonts.nix" {
          archivo = pkgs.callPackage "${sekka}/desktop/aizome/nix/fonts.nix" { };
        };
    in
    {
      packages = each (pkgs: rec {
        sekka-edu = pkgs.callPackage ./nix/package.nix {
          src = pkgs.lib.cleanSource ./.;
          aizome = aizomeFor pkgs;
          fonts = fontsFor pkgs;
          inherit kokoro;
        };
        default = sekka-edu;
        # The voice Sekka Edu's apps speak with.
        kokoro = pkgs.callPackage ./nix/kokoro.nix { };
      });

      apps = each (
        pkgs:
        let
          p = self.packages.${pkgs.stdenv.hostPlatform.system}.sekka-edu;
        in
        {
          default = self.apps.${pkgs.stdenv.hostPlatform.system}.anzan;
          anzan = {
            type = "app";
            program = "${p}/bin/sekka-anzan";
          };
          kids = {
            type = "app";
            program = "${p}/bin/sekka-kids";
          };
        }
      );

      checks = each (pkgs: {
        inherit (self.packages.${pkgs.stdenv.hostPlatform.system}) sekka-edu;
      });

      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        {
          options.programs.sekka-edu.enable = lib.mkEnableOption "Sekka Edu: Anzan and Sekka Kids";
          config = lib.mkIf config.programs.sekka-edu.enable {
            environment.systemPackages = [ self.packages.${pkgs.stdenv.hostPlatform.system}.sekka-edu ];
          };
        };
    };
}
