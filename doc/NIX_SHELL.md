# Nix Shell - Reproducible Development Environments

Nix Shell provides reproducible development environments by declaratively managing dependencies through the Nix package manager.

It allows you to specify exact versions of tools and libraries in a `shell.nix` file, ensuring consistent environments across different machines and team members.

Assume Nix is installed on the host system; do not try to automatically install Nix on the host system.

## Installing shell.nix

Create a `shell.nix` file in your project root to define your development environment:

```nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
 buildInputs = with pkgs; [
 just
 ];

 shellHook = ''
 export NODE_ENV=development
 export PATH="$PWD/node_modules/.bin:$PATH"
 '';
}
```

This example contains the `just` package which lets one define and run justfiles.

You can also set environment variables in `shell.nix`.

## Basic Usage

```shell
# Run a command within the nix shell environment
nix-shell --run "just --help"

# Run multiple commands
nix-shell --run "just build && just test"
```

## Adding Packages from nixpkgs

Add tools, language toolchains, and other dependencies by including them in the `buildInputs` list.

You can arrange them in neat categories:

```nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
 buildInputs = with pkgs; [
 # Build tools
 just
 gnumake
 
 # Language runtimes
 nodejs_20
 python3
 rustc
 cargo
 
 # Development tools
 git
 jq
 curl
 
 # Testing tools
 pytest
 
 # Documentation
 pandoc
 ];
}
```

## Claude Integration

When using Claude Code, always use `nix-shell --run` to execute tools instead of calling them directly:

```shell
# Instead of: just build
nix-shell --run "just build"

# Instead of: npm test
nix-shell --run "npm test"
```

This ensures all commands run within the proper development environment with all dependencies available.