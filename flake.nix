# Nix flake for viking (nx-decomp-tools-viking)
#
# A flake is a Nix "project file" with two parts:
#   - `inputs`:  other flakes/sources this depends on (pinned in flake.lock)
#   - `outputs`: a function that takes the inputs and returns what this flake
#                provides (packages, dev shells, ...)
#
# What this flake provides (for each supported system):
#   packages.default      -> same as viking-native
#   packages.viking-native -> the native Rust multi-call binary `viking`.
#                             It calls the Python CLI below when it needs Python
#                             (e.g. asm-differ), so it is the full tool.
#   packages.viking-py    -> the Python package (maturin + Rust extension),
#                             also installs a `viking` command (Python CLI)
#   packages.asm-differ   -> the asm-differ Python package
#   packages.python-env   -> Python 3.14 with `import viking` available, for
#                             decomp projects that write Python scripts
#   devShells.default     -> environment for developing viking itself
#
# Common commands:
#   nix build                 build packages.default into ./result
#   nix build .#viking-py     build a specific package
#   nix run . -- check ...    run `viking check ...` without installing
#   nix develop               enter the dev shell
#   nix flake update          update all inputs in flake.lock
#
# NOTE: Nix only sees files tracked by git. New files (including this one)
# must be `git add`-ed before Nix can see them.
{
  description = "Tools for NX matching reimplementation projects";

  inputs = {
    # The main Nix package collection. Everything (Rust, Python, Python
    # libraries) comes from here. `nixos-unstable` is a rolling branch that
    # has recent Rust and Python 3.14.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # asm-differ source. Nix builds have no network access, so the
    # `asm-differ @ git+https://...` URL dependency in pyproject.toml
    # can't be resolved during the build. Instead, we let Nix fetch the
    # source as a flake input, and its hash is recorded in flake.lock
    # automatically (no hash to copy-paste by hand).
    #
    # `flake = false` means "this is just source code, not a flake".
    #
    # KEEP IN SYNC with the commit in pyproject.toml. When bumping:
    #   1. change the commit here
    #   2. run `nix flake update asm-differ`
    asm-differ = {
      url = "github:Pistonight/asm-differ/def8eb4b1a629c339de3eaa3c670bdd3f2a93312";
      flake = false;
    };
  };

  # `outputs` is a function. Its arguments are the inputs above, after
  # being fetched. `self` refers to this flake itself (i.e. this repo).
  outputs =
    {
      self,
      nixpkgs,
      asm-differ,
    }:
    let
      # Nix builds are per-system, so every package needs to be defined
      # once per system. These are the systems we support
      # (see docs/SETUP.md).
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      # Helper: call `f pkgs` for each system, and collect the results into
      # an attribute set like { x86_64-linux = ...; aarch64-linux = ...; }.
      # `pkgs` is the nixpkgs package set for that system.
      forAllSystems =
        f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # Read the version from Cargo.toml, so it never needs to be updated here.
      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;

      # All package definitions for one system. Defined as a function of
      # `pkgs` so it can be reused by both `packages` and `devShells`.
      mkPackages =
        pkgs:
        let
          lib = pkgs.lib;

          # Python 3.14, as required by pyproject.toml. `python.pkgs` is the
          # set of Python libraries packaged by nixpkgs for this Python version.
          python = pkgs.python314;

          # libclang is loaded at runtime (the `clang` crate uses the `runtime`
          # feature), by the `decompme` command. On NixOS there is no global
          # /usr/lib, so we point to it explicitly with LIBCLANG_PATH.
          libclangPath = "${pkgs.llvmPackages.libclang.lib}/lib";

          # Only include the files needed to build. This means changes to
          # docs, CI, scripts, etc. don't cause a rebuild.
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./pyproject.toml
              ./README.md # referenced by pyproject.toml, maturin needs it
              ./LICENSE
              ./src
            ];
          };

          # How Nix gets the Rust dependencies. Since the build can't access
          # the network, Nix pre-fetches every crate listed in Cargo.lock,
          # using the checksums already in Cargo.lock.
          #
          # Git dependencies (capstone-rs) have no checksum in Cargo.lock.
          # `allowBuiltinFetchGit = true` lets Nix fetch them by the exact git
          # commit instead. The alternative is to list a hash per git dep in
          # `outputHashes`, which you need to update by hand when it changes.
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          # --- asm-differ ---------------------------------------------------
          # A plain Python package, built from the flake input source.
          asm-differ-pkg = python.pkgs.buildPythonPackage {
            pname = "asm-differ";
            version = "0.1.0"; # from asm-differ's pyproject.toml
            src = asm-differ; # the flake input (a path to the fetched source)

            # Build with the PEP 517 backend declared in pyproject.toml
            pyproject = true;
            build-system = [ python.pkgs.poetry-core ];

            # Runtime dependencies, from nixpkgs instead of PyPI.
            dependencies = with python.pkgs; [
              colorama
              cxxfilt
              levenshtein
              watchdog
            ];

            # asm-differ pins versions like `levenshtein = "^0.27.1"`. nixpkgs
            # only has one version of each library, which may be newer.
            # Don't fail the build over version constraints.
            pythonRelaxDeps = true;

            # Check that the package can at least be imported after building.
            pythonImportsCheck = [ "diff" ];
          };

          # --- viking (Python) ----------------------------------------------
          # The maturin-built Python package with the Rust extension module.
          viking-py = python.pkgs.buildPythonPackage {
            pname = "nx-decomp-tools-viking";
            inherit version src;
            pyproject = true;

            # The pre-fetched crates (see `cargoLock` above).
            cargoDeps = pkgs.rustPlatform.importCargoLock cargoLock;

            # Build-time helpers ("hooks"):
            # - cargoSetupHook: points cargo at the pre-fetched crates
            # - maturinBuildHook: builds the wheel with maturin. It brings in
            #   maturin, cargo and rustc, and reads the [tool.maturin]
            #   section in pyproject.toml, so the `python` feature and the
            #   `viking._core` module name are picked up from there.
            nativeBuildInputs = with pkgs.rustPlatform; [
              cargoSetupHook
              maturinBuildHook
            ];

            dependencies = [ asm-differ-pkg ];

            # Python packages' commands (`viking`, from [project.scripts]) are
            # wrapped in a small shell script. This adds LIBCLANG_PATH to that
            # wrapper. `--set-default` only sets it if the user hasn't
            # already set it.
            makeWrapperArgs = [
              "--set-default"
              "LIBCLANG_PATH"
              libclangPath
            ];

            pythonImportsCheck = [
              "viking"
              "viking._core"
            ];

            meta.mainProgram = "viking";
          };

          # --- viking (native) ----------------------------------------------
          # The Rust multi-call binary, built with cargo directly.
          viking-native = pkgs.rustPlatform.buildRustPackage {
            pname = "viking";
            inherit version src cargoLock;

            # Equivalent to:
            #   cargo build --release -p viking --bin viking \
            #     --features global-allocator,cmd-all
            buildFeatures = [
              "global-allocator"
              "cmd-all"
            ];
            cargoBuildFlags = [
              "--package"
              "viking"
              "--bin"
              "viking"
            ];

            # By default Nix runs `cargo test` after building. Skip it here:
            # it would recompile everything with different settings, and
            # tests are already run in CI.
            doCheck = false;

            nativeBuildInputs = [ pkgs.makeWrapper ];

            # Runs after the binary is copied to $out/bin.
            #
            # - The crate also has a cdylib target that cargo builds along
            #   with the binary. It isn't useful here (it's not the Python
            #   module), so remove it if it got installed.
            # - Wrap the binary, so it's a small shell script that sets
            #   environment variables and then runs the real binary:
            #   - VIKING_PYTHON_COMMAND_PROGRAM: tells the Rust CLI to call
            #     the Nix-built Python CLI when it needs Python (see
            #     src/viking_core/src/python/py_cmd.rs), instead of looking
            #     for `uv` and `uv.lock`.
            #   - LIBCLANG_PATH: same as for the Python package.
            #   The wrapper keeps argv[0], so multi-call by exe name (e.g. a
            #   symlink named `check`) still works.
            postInstall = ''
              rm -rf $out/lib
              wrapProgram $out/bin/viking \
                --set-default VIKING_PYTHON_COMMAND_PROGRAM ${lib.getExe viking-py} \
                --set-default LIBCLANG_PATH ${libclangPath}
            '';

            meta.mainProgram = "viking";
          };

          # --- Python environment -------------------------------------------
          # A `python` that can `import viking`, for decomp projects' scripts.
          # This is like a virtualenv with viking installed, but built by Nix.
          # The interpreter is wrapped with LIBCLANG_PATH too, so it also works
          # when using viking as a library.
          python-env = python.buildEnv.override {
            extraLibs = [ viking-py ];
            makeWrapperArgs = [
              "--set-default"
              "LIBCLANG_PATH"
              libclangPath
            ];
          };
        in
        {
          inherit viking-native viking-py python-env;
          asm-differ = asm-differ-pkg;
          default = viking-native;
        };
    in
    {
      # `nix build`, `nix run`, `nix profile install`, and other flakes
      # referencing `viking.packages.${system}.<name>` use these.
      packages = forAllSystems mkPackages;

      # `nix develop` enters this shell. It's for working on viking itself,
      # using the same workflow as without Nix (task, uv, cargo, ...).
      devShells = forAllSystems (
        pkgs:
        let
          python = pkgs.python314;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              # Rust toolchain from nixpkgs (instead of rustup)
              cargo
              rustc
              clippy
              rustfmt
              rust-analyzer

              # Python tooling
              python
              uv
              maturin

              # Taskfile.yml runner
              go-task
            ];

            # Environment variables set inside the shell:
            # - UV_PYTHON / UV_PYTHON_DOWNLOADS: make uv use the Python from
            #   Nix, instead of downloading its own (uv-downloaded Python may
            #   not run on NixOS).
            # - LIBCLANG_PATH: for running `decompme` during development.
            # - RUST_SRC_PATH: lets rust-analyzer find the standard library
            #   source.
            UV_PYTHON = "${python}/bin/python";
            UV_PYTHON_DOWNLOADS = "never";
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          };
        }
      );
    };
}
