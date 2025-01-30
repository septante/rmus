{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }:
  let
    overlays = [ (import rust-overlay) ];

    systems = [
      "x86_64-linux"
    ];

    eachSystem = f:
      nixpkgs.lib.genAttrs systems
      (system: f { pkgs = import nixpkgs { inherit system overlays; }; });
  in
  {
    devShells = eachSystem ({ pkgs }: with pkgs; {
      default = mkShell rec {
        buildInputs = [
          rust-bin.stable.latest.default

          # Needed for rodio to work with ALSA
          pkg-config
          alsa-lib
        ];
        LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
      };
    });
  };
}
