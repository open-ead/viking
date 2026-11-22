"""
CLI entry points code generator.

If CI=1, run in check mode.

Steps to add a new CLI command:
1. Add the new command to COMMANDS below. The order in the list will be how
   the commands show up in help text.
2. Run `task install` to generate the Rust files at `src/viking_core/src/cmd_<name>`.
3. Edit the source of truth of the description of a command, which is the documentation
   comment on the `Args` struct (src/viking_core/src/cmd_<name>/args.rs). The first
   paragraph is the short description (when listing the commands, or calling `viking <command> -h`).
   The full text is the long description when calling `viking <command> --help`.
4. Run `task install` again to update the description everywhere.

Then, run `task run -- --help`, and your new command should be listed.
Now you can implement the command in src/viking_core/src/cmd_<name>/driver.rs
"""

COMMANDS = [
    "check",
    "decompme",
    "listsym",
    "progress",

    # ^ Add new commands here
]

ARGS_TEMPLATE = """use cu::pre::*;

use crate::config::Config;

/// {{PLACEHOLDER}}
#[derive(Debug, clap::Parser)]
#[clap(bin_name = "{{COMMAND}}")]
pub struct Args {
    /// Specify the version to use
    #[clap(long)]
    pub version: Option<String>,
}

/// Parse the args. If None is returned, the help or error message
/// has already been printed
pub fn parse_args(config: &Config, cli_args: &[String]) -> Option<Args> {
    // need to put a arg[0] as the placeholder for the binary
    let mut args: Args = cu::cli::try_parse(
        std::iter::once("{{COMMAND}}").chain(cli_args.iter().map(|x| x.as_str())),
        true,
    )?;

    if args.version.is_none() {
        args.version = config.repo.default_version.clone();
    }

    Some(args)
}
"""

RUN_TEMPLATE = """use cu::pre::*;

use crate::config::Config;

use crate::cmd_{{COMMAND}}::Args;

/// Entry point for the {{COMMAND}} command
pub fn run(config: &Config, args: Args) -> cu::Result<()> {
    // implement the command here
    todo!()
}
"""

MOD_TEMPLATE = """mod args;
pub use args::*;
mod driver;
pub use driver::run;
"""

import argparse
import os
import sys
from pathlib import Path

# pyright: reportAny=false

ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT / "src" / "viking_core" / "Cargo.toml"
SRC_DIR = ROOT / "src" / "viking_core" / "src"
LAYERFILE_TOML = ROOT / "src" / "viking_core" / "Layerfile.toml"
LIB_RS = ROOT / "src" / "viking_core" / "src" / "lib.rs"
CLI_RS = ROOT / "src" / "viking_core" / "src" / "cli.rs"
PYAPI_RS = ROOT / "src" / "viking_core" / "src" / "pyapi.rs"
MAIN_RS = ROOT / "src" / "viking_core" / "main.rs"
CLI_PY = ROOT / "src" / "viking" / "cli.py"

def main() -> int:
    args = parse_args()
    if args.list:
        for command in COMMANDS:
            print(command)
        return 0
    if args.describe is not None:
        print(get_description(args.describe))
        return 0

    return generate(args.check)

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="CLI entry points code generator. With no flags, generate the code."
    )
    mode = parser.add_mutually_exclusive_group()
    _ = mode.add_argument(
        "--list",
        action="store_true",
        help="print the list of commands, one per line",
    )
    _ = mode.add_argument(
        "--describe",
        metavar="COMMAND",
        choices=COMMANDS,
        help="print the description of a command",
    )
    _ = mode.add_argument(
        "--check",
        action="store_true",
        help="check if generated code is up-to-date (forced on if CI is set)",
    )
    args = parser.parse_args()
    # in CI, the generate mode becomes check mode
    if os.environ.get("CI") and not args.list and args.describe is None:
        args.check = True
    return args

def generate(check: bool) -> int:
    s = generate_cmd_dirs(check)
    if s: return s
    s = generate_cargo_toml(check)
    if s: return s
    s = generate_layerfile_toml(check)
    if s: return s
    s = generate_lib_rs(check)
    if s: return s
    s = generate_cli_rs(check)
    if s: return s
    s = generate_pyapi_rs(check)
    if s: return s
    s = generate_main_rs(check)
    if s: return s
    s = generate_cli_py(check)
    if s: return s
    return 0

def generate_cmd_dirs(check: bool) -> int:
    """
    Generate the files in src/viking_core/src/cmd_<name> if they are missing.
    Existing files are never overwritten or deleted
    """
    templates = {
        "args.rs": ARGS_TEMPLATE,
        "driver.rs": RUN_TEMPLATE,
        "mod.rs": MOD_TEMPLATE,
    }
    missing = False
    for command in COMMANDS:
        cmd_dir = SRC_DIR / f"cmd_{command}"
        for file_name, template in templates.items():
            path = cmd_dir / file_name
            if path.exists():
                continue
            if check:
                print(f"error[gen-cli]: missing src/viking_core/src/cmd_{command}/{file_name}")
                missing = True
                continue
            content = (
                template
                .replace("{{PLACEHOLDER}}", get_placeholder_description(command))
                .replace("{{COMMAND}}", command)
            )
            cmd_dir.mkdir(parents=True, exist_ok=True)
            _ = path.write_text(content)
            print(f"generated src/viking_core/src/cmd_{command}/{file_name}")

    if missing:
        return bail_regen("command files are missing")
    return 0

def generate_cargo_toml(check: bool) -> int:
    original_lines = [x.rstrip() for x in CARGO_TOML.read_text().splitlines()]
    cmd_all_line = [x for x in original_lines if x.strip().startswith("cmd-all")]
    if len(cmd_all_line) == 0:
        return bail("cannot find the cmd-all feature")
    if len(cmd_all_line) != 1:
        return bail("there are more than one line starting with cmd-all!")
    cmd_all_line=cmd_all_line[0]
    if not cmd_all_line.endswith(']'):
        return bail("cmd-all line not formatted: the array must be one line")
    cmd_all_start = original_lines.index(cmd_all_line)
    new_cmd_all_line = f"cmd-all = [ {", ".join(f"\"cmd-{x}\"" for x in COMMANDS)} ]";

    # LLM Generated

    # the cmd-<name> sections end at the first non-cmd feature (or the next table header)
    cmd_section_end = None
    for i in range(cmd_all_start + 1, len(original_lines)):
        line = original_lines[i].strip()
        if line.startswith("["):
            cmd_section_end = i
            break
        if line.endswith("= [") and not line.startswith("cmd-"):
            cmd_section_end = i
            break
    if cmd_section_end is None:
        return bail("cannot find the end of the cmd-<name> features")

    # cmd name -> lines
    sections: dict[str, list[str]] = {}
    i = cmd_all_start + 1
    while i < cmd_section_end:
        line = original_lines[i].strip()
        if not line:
            i += 1
            continue
        if not line.startswith("cmd-") or "=" not in line:
            return bail(f"unexpected line between cmd-<name> features: {original_lines[i]}")
        name = line.split("=", 1)[0].strip().removeprefix("cmd-")
        if name in sections:
            return bail(f"duplicated feature: cmd-{name}")
        start = i
        # find the closing ']' (same line for single-line arrays)
        while not original_lines[i].strip().endswith("]"):
            i += 1
            if i >= cmd_section_end:
                return bail(f"cannot find the end of the cmd-{name} feature")
        sections[name] = original_lines[start : i + 1]
        i += 1

    for name in sections:
        if name not in COMMANDS:
            print(f"warning[gen-cli]: removing feature cmd-{name}, which is not in COMMANDS")

    # construct new lines: original[0:cmd_all_start] + new_cmd_all_line + sections in command order
    new_lines = original_lines[:cmd_all_start]
    new_lines.append(new_cmd_all_line)
    new_lines.append("")
    for command in COMMANDS:
        # insert new section if not found
        new_lines.extend(sections.get(command, [f"cmd-{command} = [", "    \"cu/cli\",", "]"]))
    new_lines.append("")
    new_lines.extend(original_lines[cmd_section_end:])

    if check:
        if original_lines != new_lines:
            return bail_regen("Cargo.toml needs to be regenerated")
        return 0

    write_if_changed(CARGO_TOML, "\n".join(new_lines) + "\n")
    return 0


def generate_layerfile_toml(check: bool) -> int:
    # LLM Generated
    original_lines = [x.rstrip() for x in LAYERFILE_TOML.read_text().splitlines()]

    def is_header(line: str) -> bool:
        return line.strip().startswith("[")

    def next_header(start: int) -> int:
        for i in range(start, len(original_lines)):
            if is_header(original_lines[i]):
                return i
        return len(original_lines)

    def strip_trailing_blank(lines: list[str]) -> list[str]:
        while lines and not lines[-1].strip():
            lines = lines[:-1]
        return lines

    cli_headers = [i for i, x in enumerate(original_lines) if x.strip() == "[layer.cli]"]
    if len(cli_headers) == 0:
        return bail("cannot find [layer.cli] in Layerfile.toml")
    if len(cli_headers) != 1:
        return bail("there are more than one [layer.cli] in Layerfile.toml")
    cli_start = cli_headers[0]
    cli_end = next_header(cli_start + 1)
    cli_lines = strip_trailing_blank(original_lines[cli_start:cli_end])

    # find the depends-on array of the cli layer (may span multiple lines)
    deps_start = None
    for i, line in enumerate(cli_lines):
        if line.strip().startswith("depends-on"):
            deps_start = i
            break
    if deps_start is None:
        return bail("cannot find depends-on in [layer.cli]")
    deps_end = deps_start
    while not cli_lines[deps_end].strip().endswith("]"):
        deps_end += 1
        if deps_end >= len(cli_lines):
            return bail("cannot find the end of depends-on in [layer.cli]")
    deps_text = " ".join(cli_lines[deps_start : deps_end + 1])
    deps_inner = deps_text[deps_text.index("[") + 1 : deps_text.rindex("]")]
    deps = [x.strip().strip("\"'") for x in deps_inner.split(",")]
    deps = [x for x in deps if x]

    # cmd_ layers first in command order, then the rest untouched
    new_deps = [f"cmd_{x}" for x in COMMANDS] + [x for x in deps if not x.startswith("cmd_")]
    new_deps_lines = ["depends-on = ["]
    new_deps_lines.extend(f"    \"{x}\"," for x in new_deps)
    new_deps_lines[-1] = new_deps_lines[-1].removesuffix(",")
    new_deps_lines.append("]")
    new_cli_lines = cli_lines[:deps_start] + new_deps_lines + cli_lines[deps_end + 1 :]

    # the [layer.cmd_<name>] sections follow the cli layer, until a non-cmd header
    sections: dict[str, list[str]] = {}
    i = cli_end
    while i < len(original_lines):
        header = original_lines[i].strip()
        if not (header.startswith("[layer.cmd_") and header.endswith("]")):
            break
        name = header.removeprefix("[layer.cmd_").removesuffix("]")
        if name in sections:
            return bail(f"duplicated layer: cmd_{name}")
        end = next_header(i + 1)
        sections[name] = strip_trailing_blank(original_lines[i:end])
        i = end
    cmd_section_end = i

    for line in original_lines[cmd_section_end:]:
        if line.strip().startswith("[layer.cmd_"):
            return bail(f"{line.strip()} must be placed right after [layer.cli]")
    for name in sections:
        if name not in COMMANDS:
            print(f"warning[gen-cli]: removing layer cmd_{name}, which is not in COMMANDS")

    # construct new lines: original[0:cli_start] + cli layer + cmd_ layers in command order + rest
    new_lines = original_lines[:cli_start]
    new_lines.extend(new_cli_lines)
    new_lines.append("")
    for command in COMMANDS:
        # insert new section if not found
        new_lines.extend(sections.get(command, [f"[layer.cmd_{command}]", 'depends-on = ["config"]']))
        new_lines.append("")
    rest = original_lines[cmd_section_end:]
    if rest:
        new_lines.extend(rest)
    else:
        new_lines = strip_trailing_blank(new_lines)

    if check:
        if original_lines != new_lines:
            return bail_regen("Layerfile.toml needs to be regenerated")
        return 0

    write_if_changed(LAYERFILE_TOML, "\n".join(new_lines) + "\n")
    return 0

    
    

def generate_lib_rs(check: bool) -> int:
    original_lines = [x.rstrip() for x in LIB_RS.read_text().splitlines()]

    # each command is a 3-line block: doc comment, cfg attribute, mod declaration
    sections: dict[str, list[str]] = {}
    block_starts: list[int] = []
    for i, line in enumerate(original_lines):
        stripped = line.strip()
        if not stripped.startswith("mod cmd_"):
            continue
        name = stripped.removeprefix("mod cmd_").removesuffix(";")
        if name in sections:
            return bail(f"duplicated module: cmd_{name}")
        start = i - 2
        if start < 0 or not original_lines[start].strip().startswith("///"):
            return bail(f"mod cmd_{name} in lib.rs must have a doc comment 2 lines above")
        if not original_lines[start + 1].strip().startswith("#[cfg("):
            return bail(f"mod cmd_{name} in lib.rs must have a #[cfg] attribute 1 line above")
        sections[name] = original_lines[start : i + 1]
        block_starts.append(start)
    if not block_starts:
        return bail("cannot find any mod cmd_* in lib.rs")

    # the blocks must be together, only separated by blank lines
    cmd_section_start = block_starts[0]
    cmd_section_end = block_starts[-1] + 3
    in_block = { j for s in block_starts for j in range(s, s + 3) }
    for j in range(cmd_section_start, cmd_section_end):
        if j not in in_block and original_lines[j].strip():
            return bail(f"unexpected line between mod cmd_* in lib.rs: {original_lines[j]}")

    for name in sections:
        if name not in COMMANDS:
            print(f"warning[gen-cli]: removing mod cmd_{name}, which is not in COMMANDS")

    # construct new lines: original[0:start] + blocks in command order + rest
    new_lines = original_lines[:cmd_section_start]
    for i, command in enumerate(COMMANDS):
        if i > 0:
            new_lines.append("")
        # insert new block if not found
        new_lines.extend(sections.get(command, [
            f"/// The `{command}` command",
            f"#[cfg(feature = \"cmd-{command}\")]",
            f"mod cmd_{command};",
        ]))
    new_lines.extend(original_lines[cmd_section_end:])

    if check:
        if original_lines != new_lines:
            return bail_regen("lib.rs needs to be regenerated")
        return 0

    write_if_changed(LIB_RS, "\n".join(new_lines) + "\n")
    return 0

def generate_cli_rs(check: bool) -> int:
    lines = ["// This file is automatically re-generated by gen-cli.py", ""]
    lines.extend(f"viking_proc_macro::make_cli!({x});" for x in COMMANDS)
    new_text = "\n".join(lines) + "\n"

    if check:
        if not CLI_RS.exists() or CLI_RS.read_text() != new_text:
            return bail_regen("cli.rs needs to be regenerated")
        return 0

    write_if_changed(CLI_RS, new_text)
    return 0

def generate_pyapi_rs(check: bool) -> int:
    original_lines = [x.rstrip() for x in PYAPI_RS.read_text().splitlines()]
    prefix = "viking_proc_macro::make_python_cli!("

    # the make_python_cli calls must be one continuous block of lines
    indices = [i for i, x in enumerate(original_lines) if x.strip().startswith(prefix)]
    if not indices:
        return bail("cannot find any make_python_cli calls in pyapi.rs")
    block_start = indices[0]
    block_end = indices[-1] + 1
    if len(indices) != block_end - block_start:
        return bail("make_python_cli calls in pyapi.rs must be one continuous block of lines")

    new_lines = original_lines[:block_start]
    new_lines.extend(f"{prefix}{x});" for x in COMMANDS)
    new_lines.extend(original_lines[block_end:])

    # the exports inside _core: the block after the cli header until an empty line
    headers = [i for i, x in enumerate(new_lines) if x.strip() == "// ======= cli ======="]
    if len(headers) == 0:
        return bail("cannot find the '// ======= cli =======' header in pyapi.rs")
    if len(headers) != 1:
        return bail("there are more than one '// ======= cli =======' header in pyapi.rs")
    header = headers[0]
    export_start = header + 1
    export_end = export_start
    while export_end < len(new_lines) and new_lines[export_end].strip():
        export_end += 1
    if export_end >= len(new_lines):
        return bail("cannot find the empty line after the cli header in pyapi.rs")
    indent = new_lines[header][: len(new_lines[header]) - len(new_lines[header].lstrip())]
    export_lines = [f"{indent}// This block is generated by gen-cli.py"]
    for command in COMMANDS:
        export_lines.append(f"{indent}#[cfg(feature = \"cmd-{command}\")]")
        export_lines.append(f"{indent}#[pymodule_export]")
        export_lines.append(f"{indent}use _core_cmd_{command};")
    new_lines = new_lines[:export_start] + export_lines + new_lines[export_end:]

    if check:
        if original_lines != new_lines:
            return bail_regen("pyapi.rs needs to be regenerated")
        return 0

    write_if_changed(PYAPI_RS, "\n".join(new_lines) + "\n")
    return 0



def generate_main_rs(check: bool) -> int:
    original_lines = [x.rstrip() for x in MAIN_RS.read_text().splitlines()]
    marker = "// This function is generated by gen-cli.py"

    # everything from the first generated function to the end of the file is regenerated
    if marker not in original_lines:
        return bail(f"cannot find '{marker}' in main.rs")
    generated_start = original_lines.index(marker)

    def multi_call_fn(name: str, condition: str) -> list[str]:
        lines = [
            marker,
            "#[rustfmt::skip]",
            "#[inline(always)]",
            f"fn {name}(exe_name: &[u8], args: Args) -> Result<cu::Result<u32>, Args> {{",
        ]
        for command in COMMANDS:
            lines.append(f"    if {condition.format(command)} {{")
            lines.append(f"        return Ok(viking::cli::{command}_argv(args.collect()));")
            lines.append("    }")
        lines.append("    Err(args)")
        lines.append("}")
        return lines

    new_lines = original_lines[:generated_start]
    new_lines.extend(multi_call_fn("try_multi_call", 'exe_name.ends_with(b"{}")'))
    new_lines.append("")
    new_lines.extend(multi_call_fn("try_multi_call_exact", 'exe_name == b"{}"'))
    new_lines.append("")
    new_lines.extend([
        marker,
        "#[rustfmt::skip]",
        "fn print_multicall_help() {",
        '    eprintln!("NX decomp tools multi-call binary");',
        '    eprintln!("The command that runs depends on the executable name, or call a command directly: viking COMMAND [ARGS...]");',
        "    eprintln!();",
        '    eprintln!("Available commands:");',
    ])
    for command in COMMANDS:
        line = f"  {command:<12}{get_short_description(command)}"
        new_lines.append(f'    eprintln!("{rust_format_str(line)}");')
    new_lines.append("}")

    if check:
        if original_lines != new_lines:
            return bail_regen("main.rs needs to be regenerated")
        return 0

    write_if_changed(MAIN_RS, "\n".join(new_lines) + "\n")
    return 0


def generate_cli_py(check: bool) -> int:
    original_lines = [x.rstrip() for x in CLI_PY.read_text().splitlines()]
    anchor = "def _commands():"

    # everything from _commands() to the end of the file is regenerated
    if anchor not in original_lines:
        return bail(f"cannot find '{anchor}' in cli.py")
    generated_start = original_lines.index(anchor)
    # the generated region is excluded from ruff format
    if generated_start == 0 or original_lines[generated_start - 1] != "# fmt: off":
        return bail(f"'{anchor}' in cli.py must be right after a '# fmt: off' line")

    new_lines = original_lines[:generated_start]
    new_lines.append(anchor)
    new_lines.append(f"    return [{', '.join(COMMANDS)}, py_json_call]")
    for command in COMMANDS:
        description = get_short_description(command).replace("\\", "\\\\").replace('"', '\\"')
        new_lines.extend([
            "",
            "",
            f"def {command}(args: list[str]) -> typing.NoReturn:",
            f'    """{description}"""',
            f"    from ._core import _core_cmd_{command}",
            "    from .core import core_state",
            "",
            f"    exit_status = _core_cmd_{command}.run(core_state, args)",
            "    sys.exit(exit_status)",
        ])

    if check:
        if original_lines != new_lines:
            return bail_regen("cli.py needs to be regenerated")
        return 0

    write_if_changed(CLI_PY, "\n".join(new_lines) + "\n")
    return 0


# == helpers ==

def get_description(command: str) -> str:
    """
    Get the full description of the command, which is the doc comment on the
    `Args` struct in src/viking_core/src/cmd_<name>/args.rs. Returns the placeholder
    description if it cannot be found.
    """
    args_rs = ROOT / "src" / "viking_core" / "src" / f"cmd_{command}" / "args.rs"
    try:
        lines = args_rs.read_text().splitlines()
    except OSError:
        return get_placeholder_description(command)

    struct_line = None
    for i, line in enumerate(lines):
        if line.strip().startswith("pub struct Args"):
            struct_line = i
            break
    if struct_line is None:
        return get_placeholder_description(command)

    # walk up from the struct, skipping the attributes (which may span multiple lines),
    # then collect the doc comment. A blank line means there is no doc comment
    doc_lines: list[str] = []
    i = struct_line - 1
    while i >= 0:
        line = lines[i].strip()
        if line.startswith("///"):
            doc_lines.append(line.removeprefix("///").removeprefix(" "))
        elif doc_lines or not line:
            break
        i -= 1
    doc_lines.reverse()

    description = "\n".join(x.rstrip() for x in doc_lines).strip()
    if not description:
        return get_placeholder_description(command)
    return description


def get_short_description(command: str) -> str:
    """The first paragraph of the description, joined into one line"""
    paragraph = get_description(command).split("\n\n", 1)[0]
    return " ".join(x.strip() for x in paragraph.splitlines())


def rust_format_str(s: str) -> str:
    """Escape the string to be used as the format string literal in a Rust macro like println!"""
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("{", "{{").replace("}", "}}")


def write_if_changed(path: Path, content: str) -> None:
    """
    Write the file only if the content is different (ignoring line endings),
    to avoid updating the timestamp
    """

    def normalize(x: str) -> str:
        return x.replace("\r\n", "\n").replace("\r", "\n")

    try:
        if normalize(path.read_bytes().decode("utf-8")) == normalize(content):
            return
    except (OSError, UnicodeDecodeError):
        pass
    _ = path.write_text(content)


def bail(msg: str) -> int:
    print(f"error[gen-cli]: {msg}")
    return 1

def bail_regen(msg: str) -> int:
    print(f"error[gen-cli]: {msg}. Please run 'task cli-bindgen'")
    return 1

def get_placeholder_description(command: str):
    return f"This is a placeholder description. Please edit the documentation comment of the Args struct in src/viking_core/src/cmd_{command}/args.rs"

if __name__ == "__main__":
    sys.exit(main())
