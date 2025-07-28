{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Build tools
    just
    
    # Language toolchain
    rustup
    
    # Issue tracking
    git-bug
    
    # Development tools
    git
  ];

  shellHook = ''
    export RUST_BACKTRACE=1
  '';
}
