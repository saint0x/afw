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

    # Production-ready static container for agents (minimal + bulletproof)
    containerRootfs = pkgs.runCommand "aria-runtime-rootfs" {
      buildInputs = with pkgs; [ 
        (busybox.override { enableStatic = true; })  # Static busybox build
        bun nodejs_20     # JavaScript runtime for agents
        curl              # HTTP client for web operations
        cacert            # SSL certificates
      ];
    } ''
      # Create standard directories
      mkdir -p $out/{dev,proc,sys,tmp,var,root,workspace}
      mkdir -p $out/{bin,etc,lib,lib64,usr/bin}
      
      # Essential system files
      cat > $out/etc/passwd << 'EOF'
root:x:0:0:root:/root:/bin/sh
EOF
      cat > $out/etc/group << 'EOF'
root:x:0:
EOF
      cat > $out/etc/hosts << 'EOF'
127.0.0.1 localhost
EOF
      echo "aria-container" > $out/etc/hostname
      
      # Static shell environment (no LD_LIBRARY_PATH needed)
      cat > $out/etc/profile << 'EOF'
export PATH="/bin:/usr/bin"
export SSL_CERT_FILE="/etc/ssl/certs/ca-bundle.crt"
export SHELL="/bin/sh"
export HOME="/root"
EOF
      
      # Copy static busybox and create all essential command symlinks
      cp ${pkgs.busybox.override { enableStatic = true; }}/bin/busybox $out/bin/busybox
      
      # Create symlinks for all essential commands (static, no dependencies)
      cd $out/bin
      for cmd in sh bash echo ls cat mkdir cp mv rm chmod chown grep sed awk sort uniq head tail wc find xargs sleep; do
        ln -s busybox $cmd
      done
      cd -
      
      # Copy JavaScript runtime (these handle their own dynamic linking)
      cp ${pkgs.bun}/bin/bun $out/bin/bun
      cp ${pkgs.nodejs_20}/bin/node $out/bin/node
      cp ${pkgs.curl}/bin/curl $out/bin/curl
      
      # Only copy libraries needed for bun/node/curl (not for shell)
      mkdir -p $out/lib/x86_64-linux-gnu $out/lib64
      
      # Minimal glibc for JavaScript runtime
      cp -L ${pkgs.glibc}/lib/libc.so.6 $out/lib/
      cp -L ${pkgs.glibc}/lib/libm.so.6 $out/lib/
      cp -L ${pkgs.glibc}/lib/libdl.so.2 $out/lib/
      cp -L ${pkgs.glibc}/lib/ld-linux-x86-64.so.2 $out/lib64/
      
      # SSL support for curl/node HTTPS
      cp -L ${pkgs.openssl.out}/lib/libssl.so.3 $out/lib/ || true
      cp -L ${pkgs.openssl.out}/lib/libcrypto.so.3 $out/lib/ || true
      
      # Make all binaries executable
      chmod +x $out/bin/*
      
      # Copy SSL certificates for HTTPS
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
        echo "Use 'nix build .#container-tarball' to build static container image"
        
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
        cargoBuildFlags = "--bin quilt";
          };

      # Static container rootfs
      container-rootfs = containerRootfs;

      # Export as tarball for quilt runtime
      container-tarball = pkgs.runCommand "nixos-production.tar.gz" {} ''
        ${pkgs.gnutar}/bin/tar -czf $out -C ${containerRootfs} .
      '';

          default = self.packages.${system}.quiltd;
        };
  };
}