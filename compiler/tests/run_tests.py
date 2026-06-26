#!/usr/bin/env python3
"""Midori compiler test suite. Compiles .mi files, runs them, checks output."""

import subprocess
import sys
import os
import tempfile
from pathlib import Path

HERE = os.path.dirname(os.path.abspath(__file__))
COMPILER_DIR = os.path.dirname(HERE)
PROJECT_DIR = os.path.dirname(COMPILER_DIR)
EXAMPLES_DIR = os.path.join(PROJECT_DIR, "examples")
MIDORI = os.path.join(COMPILER_DIR, "target", "release", "midori")


# Helpers to write readable test code
def mi(code: str) -> str:
    """Strip leading indent from multi-line string."""
    lines = code.strip().splitlines()
    if not lines:
        return ""
    indent = min((len(l) - len(l.lstrip()) for l in lines if l.strip()), default=0)
    return "\n".join(l[indent:] for l in lines) + "\n"


# Example file tests: (name, file, expected_stdout)
EXAMPLE_TESTS = [
    ("hello", "hello.mi", "Hello, Midori!\n"),
    ("variables", "variables.mi", "Midori\n"),
    ("factorial", "factorial.mi", "3628800\n"),
    ("functions", "functions.mi", "20\n"),
]

# Roundtrip tests: (name, code, expected_stdout)
ROUNDTRIP_TESTS = [
    ("expr-basic", mi("""
        fn main() {
            println(str(1 + 2))
        }
    """), "3\n"),

    ("expr-order", mi("""
        fn main() {
            println(str(2 * 3 + 4))
        }
    """), "10\n"),

    ("if-true", mi("""
        fn main() {
            if true { println("y") } else { println("n") }
        }
    """), "y\n"),

    ("if-false", mi("""
        fn main() {
            if false { println("y") } else { println("n") }
        }
    """), "n\n"),

    ("elif-chain", mi("""
        fn main() {
            let x = 2
            if x == 1 {
                println("one")
            } elif x == 2 {
                println("two")
            } else {
                println("other")
            }
        }
    """), "two\n"),

    ("while-loop", mi("""
        fn main() {
            var i = 0
            while i < 3 {
                println(str(i))
                i = i + 1
            }
        }
    """), "0\n1\n2\n"),

    ("string-concat", mi('''
        fn main() {
            println("a" + "b" + "c")
        }
    '''), "abc\n"),

    ("string-len", mi('''
        fn main() {
            println(str(len_str("hello")))
        }
    '''), "5\n"),

    ("string-idx", mi('''
        fn main() {
            println(str_char("abc"[1]))
        }
    '''), "b\n"),

    ("bool", mi("""
        fn main() {
            println(str_bool(true))
            println(str_bool(false))
        }
    """), "true\nfalse\n"),

    ("nil-return", mi("""
        fn f() {}
        fn main() {
            f()
            println("ok")
        }
    """), "ok\n"),

    ("for-range", mi("""
        fn main() {
            var s = ""
            for i in range(3) {
                s = s + str(i)
            }
            println(s)
        }
    """), "012\n"),

    ("for-range-start", mi("""
        fn main() {
            var s = ""
            for i in range(1, 4) {
                s = s + str(i)
            }
            println(s)
        }
    """), "123\n"),

    ("var-mut", mi("""
        fn main() {
            var x = 0
            x = x + 1
            println(str(x))
        }
    """), "1\n"),

    ("immutable", mi("""
        fn main() {
            let x = 5
            println(str(x))
        }
    """), "5\n"),

    ("str-interp", mi('''
        fn main() {
            let n = 42
            println("n={n}")
        }
    '''), "n=42\n"),

    ("struct-basic", mi("""
        type P { x: Int, y: Int }
        fn main() {
            let p = P { x: 1, y: 2 }
            println(str(p.x) + "," + str(p.y))
        }
    """), "1,2\n"),

    ("method-dispatch", mi("""
        type P { x: Int, y: Int }
        impl P {
            fn desc(this: P) -> Str {
                str(this.x)
            }
        }
        fn main() {
            let p = P { x: 99, y: 0 }
            println(p.desc())
        }
    """), "99\n"),

    ("str-method-len", mi('''
        fn main() {
            let s = "hello"
            println(str(s.len()))
        }
    '''), "5\n"),

    ("str-method-empty", mi("""
        fn main() {
            println(str_bool("".is_empty()))
            println(str_bool("x".is_empty()))
        }
    """), "true\nfalse\n"),

    ("match-basic", mi("""
        fn main() {
            let x = 2
            match x {
                1: { println("one") }
                2: { println("two") }
                _: { println("other") }
            }
        }
    """), "two\n"),

    ("recursion", mi("""
        fn fib(n: Int) -> Int {
            if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
        }
        fn main() {
            println(str(fib(10)))
        }
    """), "55\n"),

    ("file-exists", mi("""
        fn main() {
            println(str_bool(file_exists(".")))
        }
    """), "true\n"),

    ("file-not-exists", mi("""
        fn main() {
            println(str_bool(file_exists("/nonexistent_xyz_123")))
        }
    """), "false\n"),

    ("fn-return", mi("""
        fn add(a: Int, b: Int) -> Int {
            a + b
        }
        fn main() {
            println(str(add(3, 4)))
        }
    """), "7\n"),

    ("fn-early-return", mi("""
        fn f(x: Int) -> Int {
            if x < 0 { return 0 }
            x * 2
        }
        fn main() {
            println(str(f(-1)) + "," + str(f(5)))
        }
    """), "0,10\n"),

    ("array-basic", mi("""
        fn main() {
            let a = [1, 2, 3]
            for i in range(3) {
                println(str(a[i]))
            }
        }
    """), "1\n2\n3\n"),

    ("nested-block", mi("""
        fn main() {
            let x = {
                let y = 5
                y + 3
            }
            println(str(x))
        }
    """), "8\n"),

    ("block-expr-if", mi("""
        fn main() {
            let x = if true { 10 } else { 20 }
            println(str(x))
        }
    """), "10\n"),

    ("float-basic", mi("""
        fn main() {
            let x = 3.5
            let y = 2.0
            println(str_float(x + y))
        }
    """), "5.5\n"),

    ("char-literal", mi("""
        fn main() {
            println(str_char('A'))
        }
    """), "A\n"),

    ("char-cmp", mi("""
        fn main() {
            println(str_bool('a' == 'a'))
            println(str_bool('a' == 'b'))
        }
    """), "true\nfalse\n"),

    ("comment", mi("""
        // this is a comment
        fn main() {
            println("ok")
        }
    """), "ok\n"),

    ("multiline-str", mi('''
        fn main() {
            println("hello\\nworld")
        }
    '''), "hello\nworld\n"),

    ("os-args", mi("""
        fn main() {
            let n = len(os_args())
            println(str(n))
        }
    """), "1\n"),

    ("print-nl", mi("""
        fn main() {
            print("a")
            print("b")
            println("c")
        }
    """), "abc\n"),

    ("gt-lt", mi("""
        fn main() {
            println(str_bool(3 > 2))
            println(str_bool(2 < 3))
        }
    """), "true\ntrue\n"),

    ("eq-neq", mi("""
        fn main() {
            println(str_bool(5 == 5))
            println(str_bool(5 != 3))
        }
    """), "true\ntrue\n"),

    ("and-or", mi("""
        fn main() {
            println(str_bool(true && true))
            println(str_bool(false || true))
        }
    """), "true\ntrue\n"),
]

# Error tests: (name, code, expected_error_snippet)
ERROR_TESTS = [
    ("err-undefined", mi("""
        fn main() {
            println(str(x))
        }
    """), "x"),
]


class TestResult:
    def __init__(self, name: str, passed: bool, message: str = ""):
        self.name = name
        self.passed = passed
        self.message = message


def compile_and_run(midori: str, source_file: str) -> tuple[int, str, str]:
    """Compile and run a .mi file. Returns (exit_code, stdout, stderr)."""
    proc = subprocess.run(
        [midori, source_file],
        capture_output=True, text=True, timeout=30
    )
    return proc.returncode, proc.stdout, proc.stderr


def run_example_test(midori: str, name: str, file_rel: str,
                     expected: str) -> TestResult:
    src = os.path.join(EXAMPLES_DIR, file_rel)
    if not os.path.exists(src):
        return TestResult(name, False, f"file not found: {src}")
    rc, out, err = compile_and_run(midori, src)
    if rc != 0:
        return TestResult(name, False, f"exit code {rc}\n{err}")
    if expected is not None and out != expected:
        return TestResult(name, False, f"expected {expected!r}, got {out!r}")
    return TestResult(name, True)


def run_roundtrip(midori: str, name: str, code: str, expected: str) -> TestResult:
    with tempfile.TemporaryDirectory() as tmpdir:
        src = os.path.join(tmpdir, f"{name}.mi")
        with open(src, "w") as f:
            f.write(code)
        rc, out, err = compile_and_run(midori, src)
        if rc != 0:
            return TestResult(name, False, f"exit {rc}\n{err}")
        if out != expected:
            return TestResult(name, False, f"expected {expected!r}, got {out!r}")
    return TestResult(name, True)


def run_error_test(midori: str, name: str, code: str,
                   expected_error: str) -> TestResult:
    with tempfile.TemporaryDirectory() as tmpdir:
        src = os.path.join(tmpdir, f"{name}.mi")
        with open(src, "w") as f:
            f.write(code)
        rc, out, err = compile_and_run(midori, src)
        if rc == 0:
            return TestResult(name, False, "expected error but succeeded")
        if expected_error not in err:
            return TestResult(name, False,
                              f"expected error containing {expected_error!r}, got:\n{err}")
    return TestResult(name, True)


def main():
    print(f"Midori Test Suite")
    print(f"{'='*60}")
    print()

    midori = MIDORI
    if not os.path.exists(midori):
        print(f"Building compiler...")
        rc = subprocess.run(
            ["cargo", "build", "--release"],
            cwd=COMPILER_DIR, capture_output=True
        ).returncode
        if rc != 0:
            print("Build failed.")
            sys.exit(1)

    results: list[TestResult] = []

    # Example tests
    print("--- Example Tests ---")
    for name, file_rel, expected in EXAMPLE_TESTS:
        r = run_example_test(midori, name, file_rel, expected)
        results.append(r)
        tok = "PASS" if r.passed else "FAIL"
        print(f"  [{tok}] {name}")
        if not r.passed:
            print(f"         {r.message}")

    # Roundtrip tests
    print("--- Roundtrip Tests ---")
    for name, code, expected in ROUNDTRIP_TESTS:
        r = run_roundtrip(midori, name, code, expected)
        results.append(r)
        tok = "PASS" if r.passed else "FAIL"
        print(f"  [{tok}] {name}")
        if not r.passed:
            print(f"         {r.message}")

    # Error tests
    print("--- Error Tests ---")
    for name, code, expected_error in ERROR_TESTS:
        r = run_error_test(midori, name, code, expected_error)
        results.append(r)
        tok = "PASS" if r.passed else "FAIL"
        print(f"  [{tok}] {name}")
        if not r.passed:
            print(f"         {r.message}")

    print()
    passed = sum(1 for r in results if r.passed)
    total = len(results)
    print(f"{'='*60}")
    print(f"Results: {passed}/{total} passed")
    if passed < total:
        sys.exit(1)


if __name__ == "__main__":
    main()
