"""Fails when a file tracked by git holds a bidirectional control character
(U+202A-U+202E, U+2066-U+2069): they can make code read differently from
what the compiler sees ("Trojan Source"). Run by CI and tools/check.bat.
Usage: python tools/bidi_check.py [file ...]  (all tracked files if none)."""

import subprocess
import sys

BIDI = {chr(c) for c in list(range(0x202A, 0x202F)) + list(range(0x2066, 0x206A))}


def tracked() -> list[str]:
    out = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout
    return [p.decode("utf-8") for p in out.split(b"\0") if p]


def main(paths: list[str]) -> int:
    found = []
    for path in paths or tracked():
        try:
            with open(path, encoding="utf-8") as f:
                for number, line in enumerate(f, 1):
                    if BIDI.intersection(line):
                        found.append(f"{path}:{number}")
        except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
            continue  # binary, or not a file: nothing to read as text
    for place in found:
        print(f"bidirectional control character: {place}")
    print(f"{len(found)} line(s) with bidirectional control characters")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
