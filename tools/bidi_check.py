"""Fails when a file tracked by git holds a bidirectional control character
(U+202A-U+202E, U+2066-U+2069): they can make code read differently from
what the compiler sees ("Trojan Source"). Also fails on any other control
character but tab and line ends (a stray carriage return or backspace in a
string literal changes what the program does without showing in a diff).
Run by CI and tools/check.bat.
Usage: python tools/bidi_check.py [file ...]  (all tracked files if none)."""

import subprocess
import sys

BIDI = {chr(c) for c in list(range(0x202A, 0x202F)) + list(range(0x2066, 0x206A))}
CONTROL = {chr(c) for c in range(0x20) if c not in (0x09, 0x0A)} | {chr(0x7F)}


def tracked() -> list[str]:
    out = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout
    return [p.decode("utf-8") for p in out.split(b"\0") if p]


def problem(line: str) -> str | None:
    """What is wrong with `line` (read with its line end), if anything."""
    if BIDI.intersection(line):
        return "bidirectional control character"
    body = line[:-2] if line.endswith("\r\n") else line.rstrip("\n")
    if CONTROL.intersection(body):
        return "control character"
    return None


def main(paths: list[str]) -> int:
    found = []
    for path in paths or tracked():
        try:
            with open(path, encoding="utf-8", newline="") as f:
                for number, line in enumerate(f, 1):
                    if (what := problem(line)) is not None:
                        found.append(f"{what}: {path}:{number}")
        except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
            continue  # binary, or not a file: nothing to read as text
    for place in found:
        print(place)
    print(f"{len(found)} line(s) with hidden control characters")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
