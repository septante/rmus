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
    overlays = [
      (import rust-overlay)
      (_: prev: {
        rust-toolchain = prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      })
    ];

    systems = [
      "x86_64-linux"
    ];

    eachSystem = f:
      nixpkgs.lib.genAttrs systems
      (system: f { pkgs = import nixpkgs { inherit system overlays; }; });

    version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
  in
  {
    devShells = eachSystem ({ pkgs }: with pkgs; {
      default = mkShell rec {
        buildInputs = [
          # rust-bin.stable.latest.default
          rust-toolchain

          # Needed for rodio to work with ALSA
          pkg-config
          alsa-lib
        ];
        LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
      };
    });

    packages = eachSystem ({ pkgs }: with pkgs; {
      default = rustPlatform.buildRustPackage {
        pname = "minim";
        src = ./.;
        inherit version;

        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        buildInputs = [
          alsa-lib
        ];

        nativeBuildInputs = [
          pkg-config
        ];
      };
    });
  };
}
