
{
  inputs =  {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv/v0.6.3";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = { self, nixpkgs, rust-overlay, devenv, systems } @ inputs:
  let 
    forEachSystem = nixpkgs.lib.genAttrs (import systems);

  in
  {
    devShells = forEachSystem(system:
      let 
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.nightly.latest.default;
        tools = with pkgs; [
          vscode-langservers-extracted
          rust
          bacon
          rust-analyzer
          sqlitebrowser
          git-graph
          cargo-watch
        ];
      in
        { default = devenv.lib.mkShell {
          inherit inputs pkgs;
          modules = [
            {
              packages = tools;
              env  = with pkgs; {
                OPENSSL_DIR = "${openssl.dev}";
                OPENSSL_LIB_DIR = "${openssl.out}/lib";
                DATABASE_URL = "sqlite://sqlite.db";
              };
              processes.serve.exec = "cargo watch -x 'run --bin learn-htmx'";
            }
          ];
        };
      }
    );
  };
}



        
