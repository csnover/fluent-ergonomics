let
    pkgs = import <nixpkgs-19.09> {};
    ld = import <luminescent-dreams> {};
    frameworks = pkgs.darwin.apple_sdk.frameworks;

    darwin_frameworks = if pkgs.stdenv.buildPlatform.system == "x86_64-darwin"
      then with pkgs.darwin.apple_sdk.frameworks; [
          Security
        ]
      else [];

in pkgs.mkShell {
    name = "fluent-ergonomics";

    buildInputs = [ pkgs.pkgconfig
                    pkgs.carnix
                    ld.rust_1_41
                  ] ++ darwin_frameworks;

    shellHook = ''if [ -e ~/.nixpkgs/shellhook.sh ]; then . ~/.nixpkgs/shellhook.sh; fi'';
}
