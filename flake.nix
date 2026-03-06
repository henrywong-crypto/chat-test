{
  description = "bedrock-rs — Leptos + Axum chat app for AWS Bedrock";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Rust toolchain — nightly with wasm target
        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          targets    = [ "wasm32-unknown-unknown" ];
        };

        cargoLeptos = pkgs.cargo-leptos or (
          pkgs.rustPlatform.buildRustPackage rec {
            pname   = "cargo-leptos";
            version = "0.2.22";
            src = pkgs.fetchCrate { inherit pname version;
              hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
            };
            cargoHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
          }
        );
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain

            # cargo-leptos (from nixpkgs if available)
            cargo-leptos

            # Wasm tooling
            wasm-pack
            wasm-bindgen-cli
            binaryen           # wasm-opt

            # Node / Tailwind
            nodejs_22
            nodePackages.npm

            # AWS / infra
            awscli2

            # Dev utilities
            just               # justfile runner (alternative to make)
            watchexec
            git
            openssl
            pkg-config
          ];

          shellHook = ''
            echo "🦀 bedrock-rs dev shell"
            echo "  cargo leptos watch   — start dev server"
            echo "  npm install          — install Tailwind + PostCSS"
            echo "  npm run css:watch    — watch Tailwind (alternative to cargo-leptos style pipeline)"
            echo ""
            export RUST_LOG="info,app=debug"
            export SITE_ADDR="127.0.0.1:3000"
          '';

          # Allow dynamic linking (openssl, etc.)
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          OPENSSL_NO_VENDOR = "1";
        };
      }
    );
}
