# EasyTier astra-branch build flake — reproducible musl static binaries.
#
# Source: the composed CI workspace tree (repo content @ astra + cargo-vendored
# deps), see .gitea/workflows/nix-build.yml vendor step; rev recorded in
# source-pin.json for provenance.
# Patches over upstream v2.6.4: MagicDNS UDP listener + tld_dns_zone web console.
{
  description = "EasyTier (astra) musl static build";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systems = [ "x86_64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems f;

      sourcePin = builtins.fromJSON (builtins.readFile ./source-pin.json);

      # Source = the whole repo tree (flake lives in ci/, vendor/ committed
      # at the root by the nix-build branch of CI).
      src = ../.;

      pkgsFor = system:
        import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

      mkPackages = system:
        let
          pkgs = pkgsFor system;
          inherit (pkgs) lib stdenv;

          # Matches rust-toolchain.toml (channel 1.95, profile minimal);
          # read inline to stay in pure eval.
          rustToolchain = pkgs.rust-bin.stable."1.95.0".minimal;
          muslToolchain = rustToolchain.override {
            targets = [ "x86_64-unknown-linux-musl" ];
          };

          nativeBuildInputs = with pkgs; [
            muslToolchain
            protobuf
            pkg-config
            clang
            lld
            libclang   # bindgen (kcp-sys wrapper.h) needs libclang.so
          ];

          commonEnv = {
            SOURCE_DATE_EPOCH = "1700000000";
            CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
            CARGO_NET_OFFLINE = "true";
            # store src is read-only; keep cargo's build dir + target/ in the
            # sandbox tmp (TMPDIR is the per-derivation build dir, writable).
            CARGO_TARGET_DIR = "target";
            CARGO_HOME = ".cargo-home";
            # zstd-sys: env var must be ABSENT (is_some() trigger), so the
            # vendored/bundled C sources compile statically for musl.
            # nix env can't "unset" — so don't set it at all; stdenv is clean.
            PKG_CONFIG = "/nonexistent/pkg-config";
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            # kcp-sys build.rs probes glibc headers via KCP_SYS_EXTRA_HEADER_PATH
            # (upstream flake uses the same workaround)
            KCP_SYS_EXTRA_HEADER_PATH =
              "${pkgs.libclang.lib}/lib/clang/21/include:${pkgs.glibc.dev}/include";
            # musl has no __*_chk fortify symbols; NIX_CFLAGS_COMPILE leaks
            # -DFORTIFY_SOURCE via stdenv → force it off for C builds.
            # musl has no __*_chk fortify symbols; stdenv leaks -D_FORTIFY_SOURCE via
            # NIX_CFLAGS_COMPILE → force off. cc-rs puts env CFLAGS *before* build.rs
            # .flag() calls, so the sqlite tweaks must ride this variable (which also
            # reaches every other C unit in the dependency tree). SQLITE_CUSTOM_INCLUDE
            # is sqlite3.c's sanctioned post-platform-setup hook: sys/types.h pulls in
            # the LFS macro layer that maps open64/stat64/... onto plain names; the
            # bare -D form works because INC_STRINGIFY double-expands to <sys/types.h>.
            NIX_CFLAGS_COMPILE = "-U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 -DSQLITE_DISABLE_LFS";
            # cc-rs reads env CFLAGS *first* (before build.rs .flag() calls), so we
            # own the tail: append the plain -U_FORTIFY_SOURCE pair, then a final
            # -D that wins over sqlite's own defaults. SQLITE_CUSTOM_INCLUDE is
            # sqlite3.c's sanctioned hook to force options after platform setup —
            # here it kills USE_PREAD64, whose branch references glibc-only *64
            # symbols (pread64/pwrite64/open64/mmap64/lstat64/... ) that musl does
            # not export. HAVE_PREAD/PWRITE already select the portable branch.
            # sqlite's INC_STRINGIFY double-expands: pass the include name WITHOUT
            # angle brackets so `# include <sys/types.h>` forms correctly.
            CFLAGS = "-U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 -DSQLITE_CUSTOM_INCLUDE=sys/types.h";
          };

          # Force cargo to use the in-tree vendored deps (crates.io + 7 git deps)
          cargoConfig = pkgs.writeText "cargo-vendor-config.toml"
            (builtins.readFile (src + "/vendor-config.toml"));

          mkBin = bin: stdenv.mkDerivation {
            pname = bin;
            version = sourcePin.shortRev;
            inherit src;
            # src = repo root → unpacks under a *-source dir whose name is
            # unpredictable; the crate workspace member for these binaries
            # lives in its easytier/ subdir.
            

            preBuild = ''
              cd ./easytier
            '';

            nativeBuildInputs = nativeBuildInputs;

            env = commonEnv;

            buildPhase = ''
              runHook preBuild
              mkdir -p $CARGO_HOME
              cp ${cargoConfig} $CARGO_HOME/config.toml
              # vendor-config.toml uses a relative "vendor" dir — rewrite to the
              # absolute store path of the vendored tree.
              sed -i 's|directory = "vendor"|directory = "${src}/vendor"|' \
                $CARGO_HOME/config.toml
              cargo build --release --offline \
                --target x86_64-unknown-linux-musl \
                --bin ${bin}
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              install -Dm755 \
                ./target/x86_64-unknown-linux-musl/release/${bin} \
                $out/bin/${bin}
              runHook postInstall
            '';

            strictDeps = true;
            doInstallCheck = true;
            installCheckPhase = ''
              $out/bin/${bin} --help > /dev/null
            '';

            meta = with lib; {
              description = "EasyTier ${bin} (patched build)";
              platforms = [ "x86_64-linux" ];
            };
          };
          # web-embed: the frontend dist (pnpm build output, ~2.7 MB) is
          # committed under easytier-web/frontend/dist/ so the flake src
          # carries it — rust-embed then bakes it into the binary.
          mkWeb = embed: stdenv.mkDerivation {
            pname = if embed then "easytier-web-embed" else "easytier-web";
            version = sourcePin.shortRev;
            inherit src;

            preBuild = ''
              cd ./easytier-web
            '';

            nativeBuildInputs = nativeBuildInputs;
            env = commonEnv;

            buildPhase = ''
              runHook preBuild
              mkdir -p $CARGO_HOME
              cp ${cargoConfig} $CARGO_HOME/config.toml
              sed -i 's|directory = "vendor"|directory = "${src}/vendor"|' \
                $CARGO_HOME/config.toml
              cargo build --release --offline \
                --target x86_64-unknown-linux-musl \
                --package=easytier-web${if embed then " --features embed" else ""}
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              install -Dm755 ./target/x86_64-unknown-linux-musl/release/easytier-web $out/bin/easytier-web
              runHook postInstall
            '';

            strictDeps = true;
            meta = with lib; {
              description = "EasyTier web panel (${if embed then "embedded" else "standalone"})";
              platforms = [ "x86_64-linux" ];
            };
          };
        in
        {
          easytier-core = mkBin "easytier-core";
          easytier-cli = mkBin "easytier-cli";
          easytier-web = mkWeb false;
          easytier-web-embed = mkWeb true;
        };
    in
    {
      packages = forAllSystems mkPackages;
      defaultPackage = forAllSystems (system: (mkPackages system).easytier-core);
    };
}
