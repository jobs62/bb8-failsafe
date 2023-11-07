{
  description = "thin wapper of failsafe-rs to provide circuit breaker captilites to bb8";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    crane = {
        url = "github:ipetkov/crane";
        inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        inherit (pkgs) lib;

        craneLib = crane.lib.${system};
        src = craneLib.cleanCargoSource (craneLib.path ./.);

        commonArgs = {
          inherit src;
          strictDeps = true;
        };

        craneLibLlvmTools = craneLib.overrideToolchain
          (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
          ]);

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        bb8-failsafe = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
        });
      in {
        packages = {
            default = bb8-failsafe;
        };

        apps.default = flake-utils.lib.mkLib {
            drv = bb8-failsafe;
        };
      }
    );
}
