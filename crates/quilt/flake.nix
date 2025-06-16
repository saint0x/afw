{
  description = "Quilt - Lightweight container runtime";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay }: 
      let
    system = "x86_64-linux";
    
    # Standard packages
        pkgs = import nixpkgs {
          inherit system;
      overlays = [ rust-overlay.overlays.default ];
        };

    # Rust toolchain for development
    rustVersion = pkgs.rust-bin.stable."1.75.0".default.override {
      extensions = ["rust-src" "rustfmt" "clippy"];
        };

    # Standard Rust package builder
    buildRustPackage = args: pkgs.rustPlatform.buildRustPackage (args // {
      cargoLock = {
        lockFile = ./Cargo.lock;
      };
      
      nativeBuildInputs = with pkgs; [
        protobuf
        pkg-config
          ];
          
      buildInputs = with pkgs; [
        openssl.out
        zlib.out
          ];

      # Environment variables for dependencies
      PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.zlib.dev}/lib/pkgconfig";
      OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
      OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        });

    # Production-ready container rootfs for agents
    containerRootfs = pkgs.runCommand "aria-runtime-rootfs" {} ''
      # Create standard directories
      mkdir -p $out/{dev,proc,sys,tmp,var,root,workspace}
      mkdir -p $out/{bin,etc,lib,usr/bin}
      
      # Create essential system files
      cat > $out/etc/passwd << 'EOF'
root:x:0:0:root:/root:/bin/bash
EOF
      cat > $out/etc/group << 'EOF'
root:x:0:
EOF
      cat > $out/etc/hosts << 'EOF'
127.0.0.1 localhost
EOF
      echo "aria-container" > $out/etc/hostname
      
      # Set up environment
      cat > $out/etc/profile << 'EOF'
export PATH="/bin:/usr/bin:/usr/local/bin"
export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
export SHELL="/bin/bash"
export HOME="/root"
EOF
      
      # Create symlinks for essential binaries in /bin
      ln -sf ${pkgs.bash}/bin/bash $out/bin/bash
      ln -sf ${pkgs.bash}/bin/sh $out/bin/sh
      ln -sf ${pkgs.coreutils}/bin/* $out/bin/ 2>/dev/null || true
      ln -sf ${pkgs.bun}/bin/bun $out/bin/bun
      ln -sf ${pkgs.nodejs_20}/bin/node $out/bin/node
      ln -sf ${pkgs.curl}/bin/curl $out/bin/curl
      ln -sf ${pkgs.gnutar}/bin/tar $out/bin/tar
      ln -sf ${pkgs.gzip}/bin/gzip $out/bin/gzip
      ln -sf ${pkgs.busybox}/bin/busybox $out/bin/busybox
      
      # Copy SSL certificates
      mkdir -p $out/etc/ssl/certs
      cp -r ${pkgs.cacert}/etc/ssl/certs/* $out/etc/ssl/certs/
    '';

  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        rustVersion
        protobuf
        pkg-config
        openssl
        zlib
        stdenv.cc
          ];

          shellHook = ''
        echo "Quilt development environment"
        echo "Use 'cargo build' to build standard binaries"
        echo "Use 'nix build .#container-tarball' to build production container image"
        
        export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.zlib.dev}/lib/pkgconfig"
        export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
        export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
          '';
        };

    packages.${system} = {
      quiltd = buildRustPackage {
            pname = "quiltd";
            version = "0.1.0";
            src = ./.;
        cargoBuildFlags = "--bin quiltd";
          };

      quilt-cli = buildRustPackage {
            pname = "quilt-cli";
            version = "0.1.0";
              src = ./.;
        cargoBuildFlags = "--bin cli";
          };

      # Production container rootfs
      container-rootfs = containerRootfs;

      # Export as tarball for quilt runtime
      container-tarball = pkgs.runCommand "aria-runtime.tar.gz" {} ''
        cd ${containerRootfs}
        ${pkgs.gnutar}/bin/tar -czf $out .
      '';

          default = self.packages.${system}.quiltd;
        };
  };
}