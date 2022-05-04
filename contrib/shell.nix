with import ((import <nixpkgs> {}).fetchFromGitHub {
  owner = "NixOS";
  repo = "nixpkgs-channels";
  rev = "4762fba469e2baa82f983b262e2c06ac2fdaae67";
  sha256  = "1sidky93vc2bpnwb8avqlym1p70h2szhkfiam549377v9r5ld2r1";
}) {};
let merged-openssl = symlinkJoin { name = "merged-openssl"; paths = [ openssl.out openssl.dev ]; };
in stdenv.mkDerivation rec {
  name = "rust-env";
  env = buildEnv { name = name; paths = buildInputs; };

  buildInputs = [
    rustup
    clang
    llvm
    llvmPackages.libclang
    openssl
    cacert
    glib
    atk
    gtk3
    pango
    cairo
    harfbuzz
    gdk-pixbuf
  ];
  shellHook = let
   pkg-config-path = lib.concatMapStringsSep ":" (pkg: "${pkg.dev}/lib/pkgconfig") [
     cairo
     pango
     glib
     atk
     gtk3
     gdk-pixbuf
     harfbuzz
     librsvg
   ]; 
  in ''
  export LIBCLANG_PATH="${llvmPackages.libclang}/lib"
  export OPENSSL_DIR="${merged-openssl}"
  export PKG_CONFIG_PATH="${pkg-config-path}:$PKG_CONFIG_PATH"
  export GDK_PIXBUF_MODULE_FILE=${librsvg.out}/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache
  '';
}
