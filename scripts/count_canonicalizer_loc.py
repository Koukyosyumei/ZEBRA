#!/usr/bin/env python3
"""Count VM-specific Rust LoC used for canonicalizers.

This intentionally excludes the shared generalized helpers in
`src/canonicalizer.rs`. Calls to those helpers are counted at the call site,
because they are the VM/table-specific wiring that remains after factoring out
the generic implementation.
"""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]
VMS = ("sp1", "pico", "sphinx", "ziren", "valida")


@dataclass(frozen=True)
class SpanSpec:
    vm: str
    path: str
    kind: str
    name: str
    label: str


MANUAL_SPANS: tuple[SpanSpec, ...] = (
    # SP1 hand-written canonicalizers.
    SpanSpec("sp1", "examples/sp1/examples/addsub.rs", "function", "cr_add", "add/sub"),
    SpanSpec("sp1", "examples/sp1/examples/addsub.rs", "function", "cr_sub", "add/sub"),
    SpanSpec("sp1", "examples/sp1/examples/addsub.rs", "let_closure", "final_check", "add/sub"),
    SpanSpec("sp1", "examples/sp1/examples/branch.rs", "function", "final_check", "branch"),
    SpanSpec("sp1", "examples/sp1/examples/jump.rs", "function", "final_check", "jump"),
    SpanSpec("sp1", "examples/sp1/examples/cpu.rs", "function", "get_memory", "cpu"),
    SpanSpec("sp1", "examples/sp1/examples/cpu.rs", "function", "final_check", "cpu"),
    SpanSpec(
        "sp1",
        "examples/sp1/examples/memoryreadwrite.rs",
        "call",
        "generate_memory_op_final_checker",
        "memory",
    ),
    # Pico hand-written canonicalizers.
    SpanSpec("pico", "examples/pico/examples/addsub.rs", "function", "cr_add", "add/sub"),
    SpanSpec("pico", "examples/pico/examples/addsub.rs", "function", "cr_sub", "add/sub"),
    SpanSpec("pico", "examples/pico/examples/addsub.rs", "let_closure", "final_check", "add/sub"),
    SpanSpec("pico", "examples/pico/examples/cpu.rs", "function", "get_memory", "cpu"),
    SpanSpec("pico", "examples/pico/examples/cpu.rs", "function", "final_check", "cpu"),
    SpanSpec(
        "pico",
        "examples/pico/examples/memoryreadwrite.rs",
        "call",
        "generate_memory_op_final_checker",
        "memory",
    ),
    # Sphinx hand-written canonicalizers.
    SpanSpec("sphinx", "examples/sphinx/examples/addsub.rs", "function", "cr_add", "add/sub"),
    SpanSpec("sphinx", "examples/sphinx/examples/addsub.rs", "function", "cr_sub", "add/sub"),
    SpanSpec("sphinx", "examples/sphinx/examples/addsub.rs", "let_closure", "final_check", "add/sub"),
    SpanSpec("sphinx", "examples/sphinx/examples/cpu.rs", "function", "get_memory", "cpu"),
    SpanSpec("sphinx", "examples/sphinx/examples/cpu.rs", "function", "final_check", "cpu"),
    # Ziren hand-written canonicalizers.
    SpanSpec("ziren", "examples/ziren/examples/addsub.rs", "function", "cr_add", "add/sub"),
    SpanSpec("ziren", "examples/ziren/examples/addsub.rs", "function", "cr_sub", "add/sub"),
    SpanSpec("ziren", "examples/ziren/examples/addsub.rs", "let_closure", "final_check", "add/sub"),
    SpanSpec("ziren", "examples/ziren/examples/divrem.rs", "function", "canonical_repr_div", "div/rem"),
    SpanSpec("ziren", "examples/ziren/examples/divrem.rs", "function", "canonical_repr_rem", "div/rem"),
    SpanSpec("ziren", "examples/ziren/examples/divrem.rs", "let_closure", "final_check", "div/rem"),
    SpanSpec("ziren", "examples/ziren/examples/branch.rs", "function", "final_check", "branch"),
    SpanSpec("ziren", "examples/ziren/examples/jump.rs", "function", "final_check", "jump"),
    SpanSpec("ziren", "examples/ziren/examples/cloclz.rs", "function", "final_check", "clo/clz"),
    SpanSpec("ziren", "examples/ziren/examples/movcond.rs", "function", "final_check", "movcond"),
    SpanSpec("ziren", "examples/ziren/examples/cpu.rs", "function", "get_memory", "cpu"),
    SpanSpec("ziren", "examples/ziren/examples/cpu.rs", "function", "final_check", "cpu"),
    SpanSpec(
        "ziren",
        "examples/ziren/examples/memoryreadwrite.rs",
        "call",
        "generate_memory_op_final_checker",
        "memory",
    ),
    # Valida hand-written canonicalizers.
    SpanSpec("valida", "examples/valida/examples/lt32.rs", "function", "final_check", "lt32"),
    SpanSpec("valida", "examples/valida/examples/cpu.rs", "function", "get_memory", "cpu"),
    SpanSpec("valida", "examples/valida/examples/cpu.rs", "function", "cpu_canonicalizer", "cpu"),
    SpanSpec("valida", "examples/valida/examples/memory.rs", "function", "memory_canonicalizer", "memory"),
)


def code_loc(lines: Iterable[str]) -> int:
    loc = 0
    in_block_comment = False
    for line in lines:
        s = line.strip()
        if not s:
            continue
        if in_block_comment:
            if "*/" in s:
                in_block_comment = False
            continue
        if s.startswith("/*"):
            if "*/" not in s:
                in_block_comment = True
            continue
        if s.startswith("//"):
            continue
        loc += 1
    return loc


def find_function(lines: list[str], name: str) -> tuple[int, int]:
    pattern = re.compile(rf"^\s*(?:pub\s+)?fn\s+{re.escape(name)}\b")
    for start, line in enumerate(lines):
        if pattern.search(line):
            return scan_braced_span(lines, start)
    raise ValueError(f"function `{name}` not found")


def find_let_closure(lines: list[str], name: str) -> tuple[int, int]:
    pattern = re.compile(rf"\blet\s+{re.escape(name)}\b")
    for start, line in enumerate(lines):
        if pattern.search(line):
            return scan_braced_span(lines, start)
    raise ValueError(f"let closure `{name}` not found")


def find_call_statement(lines: list[str], call_name: str) -> tuple[int, int]:
    for start, line in enumerate(lines):
        if call_name in line and not line.strip().startswith("use "):
            parens = 0
            seen_open = False
            for end in range(start, len(lines)):
                for ch in lines[end]:
                    if ch == "(":
                        parens += 1
                        seen_open = True
                    elif ch == ")":
                        parens -= 1
                if seen_open and parens <= 0 and ";" in lines[end]:
                    return start, end + 1
    raise ValueError(f"call `{call_name}` not found")


def scan_braced_span(lines: list[str], start: int) -> tuple[int, int]:
    balance = 0
    seen_open = False
    for end in range(start, len(lines)):
        for ch in lines[end]:
            if ch == "{":
                balance += 1
                seen_open = True
            elif ch == "}":
                balance -= 1
        if seen_open and balance <= 0:
            if lines[start].lstrip().startswith("let "):
                while end + 1 < len(lines) and ";" not in lines[end]:
                    end += 1
            return start, end + 1
    raise ValueError(f"unterminated braced span starting at line {start + 1}")


def span_for(spec: SpanSpec) -> tuple[int, int, int]:
    path = REPO_ROOT / spec.path
    lines = path.read_text().splitlines()
    if spec.kind == "function":
        start, end = find_function(lines, spec.name)
    elif spec.kind == "let_closure":
        start, end = find_let_closure(lines, spec.name)
    elif spec.kind == "call":
        start, end = find_call_statement(lines, spec.name)
    else:
        raise ValueError(f"unknown span kind `{spec.kind}`")
    return start + 1, end, code_loc(lines[start:end])


def discovered_helper_calls() -> list[SpanSpec]:
    specs: list[SpanSpec] = []
    helper_names = ("generate_alu_final_checker",)
    for vm in VMS:
        for path in sorted((REPO_ROOT / "examples" / vm / "examples").glob("*.rs")):
            text = path.read_text()
            for helper in helper_names:
                if helper in text:
                    specs.append(
                        SpanSpec(
                            vm,
                            str(path.relative_to(REPO_ROOT)),
                            "call",
                            helper,
                            "lookup-driven ALU",
                        )
                    )
    return specs


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--details", action="store_true", help="print every counted span")
    args = parser.parse_args()

    specs = list(MANUAL_SPANS) + discovered_helper_calls()
    rows = []
    totals = {vm: 0 for vm in VMS}
    for spec in specs:
        start, end, loc = span_for(spec)
        totals[spec.vm] += loc
        rows.append((spec.vm, spec.label, spec.path, f"{start}-{end}", loc))

    if args.details:
        print("| zkVM | Table | File | Lines | Rust LoC |")
        print("|---|---|---|---:|---:|")
        for vm, label, path, span, loc in rows:
            print(f"| {vm} | {label} | `{path}` | {span} | {loc} |")
        print()

    print("| zkVM | Total Rust LoC |")
    print("|---|---:|")
    for vm in VMS:
        print(f"| {vm} | {totals[vm]} |")
    avg = sum(totals.values()) / len(totals)
    print(f"\nAverage across zkVMs, excluding OpenVM: {avg:.1f} LoC")


if __name__ == "__main__":
    main()
