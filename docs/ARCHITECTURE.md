# Architecture

This page documents the architectural design desicions.

## Python and Rust split

The codebase is split into the Python API (`src/viking`) and the Rust implementation
(`src/viking_core`). In most cases, the Rust side does the heavy lifting, and the Python side
exposes a nice API for downstream projects to integrate into their tooling scripts.

As a rule of thumb, when deciding whether something should be in Rust or Python:

- **Implement it in Rust and expose a Python API** when:
  - Both Rust and Python code depend on it.
  - The functionality is complex, such as parsing or validation.
  - The functionality benefits from the performance of native code.
- **Implement it in Python directly** when:
  - The functionality is simple and easier to implement in Python (such as spawning a tool).
  - The Rust side does NOT depend on it.

## Python-Rust interop

### Interop modules

There are 3 modules responsible for Python-Rust interop: `pyinterop`, `python`, and `pyapi`,
each serves a different role in the dependency graph:
- `pyinterop`: Low-level interop utils that `python` and `pyapi` depends on, does NOT depend on other modules.
- `python`: Core interop utils that can depend on other core modules such as `config`.
  Handles Rust-to-Python calls. Other CLI modules may depend on it (such as `cmd_check`)
- `pyapi`: Highest-level APIs exposed to Python for Python-to-Rust calls. No other modules
  may depend on `pyapi`.

### Calling Python from Rust

Only Python code calls Rust directly. Rust never calls into Python in the same process.
This way, Python adds no overhead to paths that don't need it, even when other
paths in the same command do (like `check`).

When Rust needs to run Python code, it does so through `PyCmd`, which serializes
the command as a JSON payload and runs the `py-json-call` command of the Python CLI
in a new process.

To add a new Python call:
- Add the command to [`PyCmd`](../src/viking_core/src/python/py_cmd.rs).
- Add a Rust function that runs the command to the
  [`python`](../src/viking_core/src/python/driver.rs) module. This is what other Rust code calls.
- Run `task install` to regenerate the Python bindings (this is only needed
  so your editor picks up the new command).
- Handle the command in [`run_json_command`](../src/viking/core.py).

### "Global" state in Rust

The Rust side should not have process-wide singletons, to support Python's upcoming
multi-interpreter feature, where a single process may host multiple Python interpreters.
This means avoiding things like `static MY_STATE: LazyLock<...> = ...` in Rust.

Unfortunately, PyO3 does not currently support per-module state. We work around this
by allocating the global state in Rust once during module initialization, then passing
(leaking) the handle to Python, which stores it as a module-scope variable. Rust functions
that depend on this global state must take it as an argument from the Python side.
See [`core.py`](../src/viking/core.py).

This global state should not be exposed in the public Python API. Instead, add a wrapper
that calls the `_core` API internally and passes in `core_state`.
