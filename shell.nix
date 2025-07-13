let
    nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/archive/10e687235226880ed5e9f33f1ffa71fe60f2638a.tar.gz"; # nixos-25.05
    pkgs = import nixpkgs {
        config = {};
        overlays = [];
    };
in
    pkgs.mkShellNoCC {
        name = "archypix-app-back-nix-shell";

        nativeBuildInputs = with pkgs; [ # = packages
            cargo
            rustc
            rustup
            clang
            pkg-config

            lolcat
            cowsay
        ];
        buildInputs = with pkgs; [
            cacert
            glib
            gexiv2
            libpq
            imagemagick
        ];

        GREETING = "Welcome to the Archypix App Back Nix shell!";

        shellHook = ''
            echo $GREETING | cowsay | lolcat
          '';
    }
