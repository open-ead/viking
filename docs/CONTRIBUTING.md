# Contributing Guidelines

## Setup

Clone the repo and install these tools:

- [uv](https://docs.astral.sh/uv/getting-started/installation/)
- [Rust](https://rustup.rs)
- [Task](https://taskfile.dev), the task runner

## Development workflow

Always use the `task` commands instead of running `uv` or `cargo` directly.
They use the right flags and skip unnecessary rebuilds. See `Taskfile.yml`
for how each command works.

Run `task --list` to list all available commands.

Common workflows:
- `task install`: Sync dependencies and regenerate the Python bindings from Rust.
  Rerun it whenever the repo is updated or the Python bindings need to be regenerated.
- `task run -- [ARGS]...`: Run the viking CLI. The Rust code is rebuilt automatically.
  You may need to set the `VIKING_CONFIG` environment variable to point to the
  project you are testing with (e.g. `VIKING_CONFIG=path/to/botw/tools/config.toml task run -- check`).
- `task check`: Check for formatting and linting issues.
- `task fix`: Fix formatting issues.

Before opening or updating a PR, run `task fix` and then `task check` to catch
trivial issues, so the review goes more smoothly.

## Architecture

Refer to the [Architecture](./ARCHITECTURE.md) page for the architectural design of this repo.

## Rust feature flags

The default feature set only enables the minimal functionality. For local development,
you can comment out `default = []` in `Cargo.toml` and set `default = ...`
to whatever feature set you are working on, to get rust-analyzer pick up those features.
Remember to change them back when you commit.


## Checking core modules dependency

The Rust crate produces an interal dependency graph amongst its modules. For example,
`config` handles loading the repo's config, and `fl` loads the file list based on the
path specified in the config. Therefore `fl` depends on `config`.
To keep this dependency relationship clean and maintainable in the future,
`config` should not depend on `fl`.

This one-way dependency rule is enforced for every top-level module defined in `lib.rs`.
The current dependency graph is defined in the [`Layerfile`](../src/viking_core/Layerfile.toml).

To check the dependency, install `layered-crate`:

```shell
# install from source
cargo install layered-crate
# install from pre-built binaries on GitHub using cargo-binstall
cargo binstall layered-crate
```

Then run `task check-layers` (`task check` will also include this check once `layered-crate` is installed).


## Generating CLI entry points

Because the CLI entry points need to be hooked up in many places (Rust, Python bindings, Python, Cargo features, etc...),
the `gen-cli.py` script takes care of all of it. There are only 2 places that need to be edited:

- The list of commands in `gen-cli.py`.
- The documentation of the commands in the respective `Args` struct.

Everything else is generated from these 2 places.

To add a new command:
1. Refer to [`gen-cli.py`](../scripts/gen-cli.py) to add the new command and generate a stub.
2. Add the arguments to the command to the generated `Args` struct in `args.rs`, which uses [`clap_derive`](https://docs.rs/clap/latest/clap/_derive/index.html)
   to declare the parser directly in the struct.
3. Implement the `run` function in `driver.rs`, which takes the repo `Config` and parsed `Args` and return a `Result`

Everything else that need to be hooked up will be handled by the generator script.

