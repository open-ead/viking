"""Git repo utilities"""

from pathlib import Path
from typing import Literal, overload

from ._core import _core_git
from .core import core_state


@overload
def read_file_at_revision(path: str | Path, rev: str, raw: Literal[True]) -> bytes: ...
@overload
def read_file_at_revision(
    path: str | Path, rev: str, raw: Literal[False] = False
) -> str: ...


def read_file_at_revision(path: str | Path, rev: str, raw=False) -> str | bytes:
    """Read a file from a Git revision.

    The path must be within the repo and tracked by Git. If `raw=True`, then raw bytes
    are returned; otherwise the file is decoded as UTF-8
    """
    if raw:
        return _core_git.read_file_at_revision(core_state, str(path), rev)
    return _core_git.read_utf8_file_at_revision(core_state, str(path), rev)
