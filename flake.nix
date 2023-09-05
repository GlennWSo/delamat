
{
  inputs =  {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:

    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit  system;
        };
        py = pkgs.python310Packages;
        python = py.python;

        dev_run = pkgs.writeScriptBin "run" ''
          python -m flask --app ./main.py --debug run
        '';

      in
        {
          devShell = pkgs.mkShell rec {
            name = "flake pyrust";
            venvDir = ".venv";
            root = ./.;

            buildInputs = [
              dev_run
              pkgs.vscode-langservers-extracted
              python
              py.venvShellHook
              py.black
              py.flask
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
            '';
          };
        }
    );
}
