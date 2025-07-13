{
    description = "Development environment for Archypix App Back";

    inputs = {
        nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, flake-utils }:
        flake-utils.lib.eachDefaultSystem (system:
            let pkgs = import nixpkgs {
                inherit system;
                config = {
                    allowUnfree = true;
                };
            };
            in with pkgs; rec {
                devShell = pkgs.mkShellNoCC {
                    name = "archypix-app-back-nix-shell";
                    nativeBuildInputs = with pkgs; [
                       cargo
                       rustc
                       clang
                       pkg-config
                       lolcat
                       cowsay
                    ];
                    buildInputs = with pkgs; [
                       cacert
                       llvmPackages.libclang
                       glib
                       gexiv2
                       libpq
                       imagemagick
                    ];
                    shellHook = "echo 'Welcome to the Archypix App Back Nix shell!' | cowsay | lolcat";
                };
                packages.default = rustPlatform.buildRustPackage rec {
                    pname = "archypix-app-back";
                    version = "0.1.0";

                    src =  ./.;

                    nativeBuildInputs = with pkgs; [
                        cargo
                        rustc
                        clang
                        pkg-config
                        lolcat
                        cowsay
                        llvmPackages.libclang
                    ];
                    buildInputs = with pkgs; [
                        cacert
                        glib
                        gexiv2
                        libpq
                        imagemagick
                    ];

                    cargoLock = {
                        lockFile = ./Cargo.lock; # To determine the output hash
                    };

                    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
                    # Required env var for the `build.rs` of magick_rust to use tho correct ImageMagick CLANG flags.
                    # This variable is used here: https://github.com/nlfiedler/magick-rust/blob/dfd8df0dd102348c23b33bfc946a9d70b5db25bf/build.rs#L127C9-L127C28
                    # Not setting this variable will throw the error: "you should set MAGICKCORE_HDRI_ENABLE"
                    BINDGEN_EXTRA_CLANG_ARGS = "-DMAGICKCORE_HDRI_ENABLE=1 -DMAGICKCORE_QUANTUM_DEPTH=16 -DMAGICKCORE_CHANNEL_MASK_DEPTH=32 -I${pkgs.imagemagick.dev}/include/ImageMagick-7";

                    meta = {
                        homepage = "";
                        description = "";
                        license = lib.licenses.elastic20;
                        allowUnfree = true;
                    };
                };
           }
        );
}
