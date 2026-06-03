"""Lean shared library discovery for importing the native extension."""

from __future__ import annotations

import ctypes
import os
from pathlib import Path
import platform
import shutil
import subprocess
from typing import Iterable


_PRELOADED: list[ctypes.CDLL] = []
_PRELOADED_RUNTIME: Path | None = None


def preload_lean_runtime(extension_dir: Path | None = None) -> Path | None:
    """Find and preload Lean's external shared runtime before importing _ai."""
    global _PRELOADED_RUNTIME
    if _PRELOADED_RUNTIME is not None:
        return _PRELOADED_RUNTIME

    # Kept for compatibility with older callers. Runtime libraries are loaded
    # from their external Lean installation path instead of linked into the
    # Python extension directory.
    _ = extension_dir

    for lib_dir in iter_lean_library_dirs():
        lib_paths = list(_runtime_library_paths(lib_dir))
        if not lib_paths:
            continue
        try:
            handles = _load_runtime_libraries(lib_paths)
        except OSError:
            continue
        if handles:
            _PRELOADED.extend(handles)
            _PRELOADED_RUNTIME = _primary_runtime_path(lib_paths)
            return _PRELOADED_RUNTIME

    return None


def iter_lean_shared_libraries() -> Iterable[Path]:
    seen: set[Path] = set()
    for lib_dir in iter_lean_library_dirs():
        for lib_path in _runtime_library_paths(lib_dir):
            if lib_path.name != _shared_library_name():
                continue
            if lib_path.exists():
                resolved = lib_path.resolve()
                if resolved not in seen:
                    seen.add(resolved)
                    yield resolved


def iter_lean_library_dirs() -> Iterable[Path]:
    seen: set[Path] = set()

    for path in _env_paths("LEAN_LIB_DIR"):
        yield from _once(seen, path)

    lean_home = os.environ.get("LEAN_HOME")
    if lean_home:
        yield from _once(seen, Path(lean_home) / "lib" / "lean")

    yield from _lean_prefix_command_dirs(seen)
    yield from _lake_env_dirs(seen)
    yield from _elan_toolchain_dirs(seen)


def _load_runtime_libraries(lib_paths: list[Path]) -> list[ctypes.CDLL]:
    handles: list[ctypes.CDLL] = []
    pending = list(reversed(lib_paths))
    last_error: OSError | None = None

    while pending:
        next_pending: list[Path] = []
        made_progress = False

        for lib_path in pending:
            try:
                handles.append(_load_shared_library(lib_path))
                made_progress = True
            except OSError as error:
                last_error = error
                next_pending.append(lib_path)

        if not made_progress:
            raise last_error or OSError("could not load Lean runtime")
        pending = next_pending

    return handles


def _load_shared_library(lib_path: Path) -> ctypes.CDLL:
    if platform.system() == "Windows":
        os.add_dll_directory(str(lib_path.parent))
        return ctypes.CDLL(str(lib_path))

    mode = getattr(ctypes, "RTLD_GLOBAL", 0)
    return ctypes.CDLL(str(lib_path), mode=mode)


def _shared_library_name() -> str:
    system = platform.system()
    if system == "Darwin":
        return "libleanshared.dylib"
    if system == "Windows":
        return "libleanshared.dll"
    return "libleanshared.so"


def _runtime_library_names() -> tuple[str, ...]:
    system = platform.system()
    if system == "Darwin":
        return (
            "libInit_shared.dylib",
            "libleanshared_2.dylib",
            "libleanshared_1.dylib",
            "libleanshared.dylib",
        )
    if system == "Windows":
        return (
            "Init_shared.dll",
            "libInit_shared.dll",
            "libleanshared_2.dll",
            "libleanshared_1.dll",
            "libleanshared.dll",
        )
    return (
        "libInit_shared.so",
        "libleanshared_2.so",
        "libleanshared_1.so",
        "libleanshared.so",
    )


def _runtime_library_paths(lib_dir: Path) -> Iterable[Path]:
    for name in _runtime_library_names():
        path = lib_dir / name
        if path.exists():
            yield path.resolve()


def _primary_runtime_path(lib_paths: list[Path]) -> Path:
    shared_name = _shared_library_name()
    for path in lib_paths:
        if path.name == shared_name:
            return path
    return lib_paths[-1]


def _env_paths(name: str) -> Iterable[Path]:
    value = os.environ.get(name)
    if not value:
        return
    for entry in value.split(os.pathsep):
        if entry:
            yield Path(entry)


def _lean_prefix_command_dirs(seen: set[Path]) -> Iterable[Path]:
    candidates = [
        os.environ.get("LEAN"),
        shutil.which("lean"),
        str(Path.home() / ".elan" / "bin" / _exe("lean")),
    ]
    for command in candidates:
        if not command:
            continue
        prefix = _run_stdout([command, "--print-prefix"])
        if prefix:
            yield from _once(seen, Path(prefix) / "lib" / "lean")


def _lake_env_dirs(seen: set[Path]) -> Iterable[Path]:
    lake = shutil.which("lake")
    if not lake:
        return
    lean_home = _run_stdout([lake, "env", "printenv", "LEAN_HOME"])
    if lean_home:
        yield from _once(seen, Path(lean_home) / "lib" / "lean")


def _elan_toolchain_dirs(seen: set[Path]) -> Iterable[Path]:
    toolchains = Path.home() / ".elan" / "toolchains"
    if not toolchains.is_dir():
        return

    entries = sorted(
        (entry for entry in toolchains.iterdir() if entry.is_dir()),
        key=lambda entry: entry.stat().st_mtime,
        reverse=True,
    )
    for entry in entries:
        yield from _once(seen, entry / "lib" / "lean")


def _run_stdout(command: list[str]) -> str | None:
    try:
        result = subprocess.run(
            command,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            timeout=2,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None

    if result.returncode != 0:
        return None
    output = result.stdout.strip()
    return output or None


def _once(seen: set[Path], path: Path) -> Iterable[Path]:
    try:
        resolved = path.expanduser().resolve()
    except OSError:
        return
    if resolved in seen:
        return
    seen.add(resolved)
    yield resolved


def _exe(name: str) -> str:
    if platform.system() == "Windows":
        return f"{name}.exe"
    return name
