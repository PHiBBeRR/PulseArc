#!/usr/bin/env python3
"""
Add doc comments to Rust unit tests that are missing them.

The script scans Rust source files for test functions (#[test], #[tokio::test], etc.)
and injects a generated doc comment that summarizes the scenario plus the assertions
performed inside the test body. Existing doc comments are left untouched.
"""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Optional, Sequence, Tuple


TEST_ATTR_PATTERN = re.compile(r"#\[(?:tokio::)?test")

ACRONYM_MAP = {
    "api": "API",
    "cpu": "CPU",
    "db": "DB",
    "http": "HTTP",
    "https": "HTTPS",
    "id": "ID",
    "io": "IO",
    "jwt": "JWT",
    "sql": "SQL",
    "ssl": "SSL",
    "tls": "TLS",
    "ui": "UI",
    "url": "URL",
    "uuid": "UUID",
    "rbac": "RBAC",
    "ipc": "IPC",
    "ip": "IP",
}


def is_test_attribute(line: str) -> bool:
    """Return True when the attribute line marks a unit test."""
    stripped = line.strip()
    if not stripped.startswith("#["):
        return False
    if stripped.startswith("#[cfg"):
        return False
    return bool(TEST_ATTR_PATTERN.search(stripped))


def is_attribute(line: str) -> bool:
    """Detect any attribute (#[...]) line."""
    stripped = line.strip()
    return stripped.startswith("#[") and not stripped.startswith("///")


def is_doc_comment(line: str) -> bool:
    return line.lstrip().startswith("///")


def clean_expr(expr: str) -> str:
    """Normalize whitespace in an expression for display."""
    return " ".join(expr.replace("\n", " ").split())


def split_top_level(expr: str) -> List[str]:
    """Split an expression on commas that are not nested within delimiters."""
    parts: List[str] = []
    depth = 0
    current: List[str] = []
    for char in expr:
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        if char == "," and depth == 0:
            parts.append("".join(current).strip())
            current = []
            continue
        current.append(char)
    if current:
        parts.append("".join(current).strip())
    return parts


@dataclass
class AssertSummary:
    macro: str
    text: str


def extract_asserts(body: str) -> List[AssertSummary]:
    """Extract assertion macros from the function body."""
    summaries: List[AssertSummary] = []
    idx = 0
    while idx < len(body):
        match = re.search(r"assert[_a-zA-Z0-9]*!\s*\(", body[idx:])
        if not match:
            break
        start = idx + match.start()
        macro = match.group().split("!")[0].strip()
        paren_start = start + match.group().find("(")
        depth = 1
        pos = paren_start + 1
        while pos < len(body) and depth > 0:
            char = body[pos]
            if char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
            pos += 1
        expr = body[paren_start + 1 : pos - 1]
        summaries.append(
            AssertSummary(
                macro=macro,
                text=format_assert_text(macro, expr),
            )
        )
        semicolon = body.find(";", pos)
        idx = pos if semicolon == -1 else semicolon + 1
    return summaries


def format_assert_text(macro: str, expr: str) -> str:
    """Generate a human-friendly bullet for an assertion."""
    cleaned = clean_expr(expr)
    parts = split_top_level(cleaned)
    macro = macro.strip()
    if macro == "assert_eq" and len(parts) >= 2:
        return f"Confirms `{parts[0]}` equals `{parts[1]}`"
    if macro == "assert_ne" and len(parts) >= 2:
        return f"Confirms `{parts[0]}` differs from `{parts[1]}`"
    if macro == "assert":
        return f"Ensures `{parts[0] if parts else cleaned}` evaluates to true"
    return f"Checks `{macro}!({cleaned})`"


def find_function_body(lines: Sequence[str], fn_index: int) -> Tuple[str, int]:
    """Return the function body (between braces) and the index of the closing brace."""
    joined_from_fn = "\n".join(lines[fn_index:])
    brace_start = joined_from_fn.find("{")
    if brace_start == -1:
        return "", fn_index
    depth = 0
    close_pos = None
    for offset, char in enumerate(joined_from_fn[brace_start:], start=brace_start):
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                close_pos = offset
                break
    if close_pos is None:
        return "", len(lines)
    body = joined_from_fn[brace_start + 1 : close_pos]
    consumed_lines = joined_from_fn[: close_pos + 1].count("\n")
    end_index = fn_index + consumed_lines
    return body, end_index


def extract_function_name(line: str) -> Optional[str]:
    match = re.search(r"\bfn\s+([a-zA-Z0-9_]+)", line)
    if match:
        return match.group(1)
    return None


def humanize_name(name: str) -> str:
    tokens = [token for token in name.split("_") if token]
    if tokens and tokens[0] == "test":
        tokens = tokens[1:]
    if not tokens:
        return "test"

    words: List[str] = []
    for idx, token in enumerate(tokens):
        lower = token.lower()
        if lower in ACRONYM_MAP:
            words.append(ACRONYM_MAP[lower])
        elif idx == 0:
            words.append(token.capitalize())
        else:
            words.append(lower)
    return " ".join(words)


def find_primary_target(body: str) -> Optional[str]:
    match = re.search(r"([A-Z][A-Za-z0-9_]*::(?:<[^>]+>::)?[A-Za-z0-9_]+)", body)
    if match:
        target = match.group(1)
        target = re.sub(r"::\<[^>]+>::", "::", target)
        return target
    return None


def build_doc_comment(indent: str, name: str, body: str) -> List[str]:
    display_name = humanize_name(name)
    target = find_primary_target(body)
    preface: str
    if target:
        preface = f"Validates `{target}` behavior"
        if display_name:
            preface += f" for the {display_name.lower()} scenario"
        preface += "."
    else:
        preface = f"Validates the {display_name.lower()} scenario."
    lines = [f"{indent}/// {preface}"]

    asserts = extract_asserts(body)
    if asserts:
        lines.append(f"{indent}///")
        lines.append(f"{indent}/// Assertions:")
        for summary in asserts:
            lines.append(f"{indent}/// - {summary.text}.")
    else:
        lines.append(f"{indent}///")
        lines.append(f"{indent}/// Assertion coverage: ensures the routine completes without panicking.")
    return lines


def process_file(path: Path) -> bool:
    original = path.read_text()
    lines = original.splitlines()
    new_lines: List[str] = []
    i = 0
    changed = False

    while i < len(lines):
        line = lines[i]
        if is_test_attribute(line):
            block_start = i
            while block_start > 0 and is_attribute(lines[block_start - 1]):
                block_start -= 1

            doc_idx = block_start - 1
            while doc_idx >= 0 and lines[doc_idx].strip() == "":
                doc_idx -= 1
            doc_exists = doc_idx >= 0 and is_doc_comment(lines[doc_idx])

            if not doc_exists:
                attr_end = i
                while attr_end + 1 < len(lines) and is_attribute(lines[attr_end + 1]):
                    attr_end += 1
                fn_idx = attr_end + 1
                while fn_idx < len(lines) and "fn " not in lines[fn_idx]:
                    fn_idx += 1
                if fn_idx < len(lines):
                    body, _ = find_function_body(lines, fn_idx)
                    fn_name = extract_function_name(lines[fn_idx]) or "test"
                    indent = re.match(r"\s*", lines[block_start]).group(0)
                    comment_lines = build_doc_comment(indent, fn_name, body)
                    insert_at = len(new_lines) - (i - block_start)
                    if insert_at < 0:
                        insert_at = 0
                    new_lines[insert_at:insert_at] = comment_lines
                    changed = True
        new_lines.append(line)
        i += 1

    if changed:
        path.write_text("\n".join(new_lines) + ("\n" if original.endswith("\n") else ""))
    return changed


def iter_rust_files(paths: Sequence[Path]) -> Iterable[Path]:
    for path in paths:
        if path.is_dir():
            yield from sorted(
                p for p in path.rglob("*.rs") if "target" not in p.parts and ".cargo" not in p.parts
            )
        elif path.suffix == ".rs":
            yield path


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        default=[Path("crates")],
        help="Files or directories to scan (defaults to the ./crates workspace).",
    )
    args = parser.parse_args(argv)

    any_changed = False
    for rust_file in iter_rust_files(args.paths):
        if process_file(rust_file):
            any_changed = True
    return 0 if any_changed else 0


if __name__ == "__main__":
    raise SystemExit(main())
