
{
  inputs =  {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:

    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };        py = pkgs.python311Packages;
        python = py.python;

        dev_run = pkgs.writeScriptBin "run" ''
          python -m flask --app ./main.py --debug run
        '';

        rust = pkgs.rust-bin.nightly.latest.default;
        openssl=pkgs.openssl;

      in
        {
          devShell = pkgs.mkShell rec {
            name = "flake pyrust";
            venvDir = ".venv";
            DATABASE_URL = "sqlite://sqlite.db";
            root = ./.;

            buildInputs = [
              dev_run
              pkgs.vscode-langservers-extracted
              python
              py.venvShellHook
              py.black
              py.flask
              py.email-validator
              rust
              pkgs.bacon
              pkgs.rust-analyzer
              pkgs.sqlitebrowser
              pkgs.git-graph

            ];
            PY = py.python;

            postVenvCreation = ''
              unset SOURCE_DATE_EPOCH
              # pip install -r ${root}/deps/requirements.txt
              # pip install -r ${root}/deps/test_requirements.txt
            '';

            postShellHook = ''
              # allow pip to install wheels
              unset SOURCE_DATE_EPOCH
              export IPYTHONDIR=$PWD/.ipy/           
              export OPENSSL_DIR="${openssl.dev}"
              export OPENSSL_LIB_DIR="${openssl.out}/lib"
            '';
          };
        }
    );
}
