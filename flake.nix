{
  description = "MyCitadel Wallet app for Linux, Windows & MacOS desktop made with GTK+ ";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, naersk, nixpkgs, flake-utils, flake-compat, fenix, crane }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
      };
      fenix-pkgs = fenix.packages.${system};
      fenix-channel = (fenix-pkgs.stable);

      craneLib = (crane.mkLib pkgs).overrideScope' (final: prev: {
        cargo = fenix-channel.cargo;
        rustc = fenix-channel.rustc;
      });

      commonArgs = {
        src = ./.;
        buildInputs = with pkgs; [
          python311
          pango
          atk
          gtk3
          wrapGAppsHook
        ];
        nativeBuildInputs = [
          pkgs.pkgconfig
          fenix-channel.rustc
          pkgs.lld
        ];
      };

      cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
        pname = "mycitadel-deps";
      });

      mycitadel = craneLib.buildPackage (commonArgs // {
        pname = "mycitadel";
      });

    in {
      defaultPackage = mycitadel;

      devShell = pkgs.mkShell {
        buildInputs = cargoArtifacts.buildInputs;
        nativeBuildInputs = cargoArtifacts.nativeBuildInputs ++ [
          fenix-pkgs.rust-analyzer
          fenix-channel.rustfmt
          fenix-channel.rustc
          fenix-channel.cargo
        ];
        RUST_SRC_PATH = "${fenix-channel.rust-src}/lib/rustlib/src/rust/library";
      };
  });
}
