#!/usr/bin/env python3
"""Extract test snippets from Rust test files for fuzzing corpus."""

import re
import hashlib
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
TEST_DIR = PROJECT_ROOT / "tests" / "interpreter"
CORPUS_DIR = SCRIPT_DIR.parent / "corpus"


def extract_snippets(content: str) -> list[str]:
    """Extract all eval(), eval_result(), and throws_error() snippets."""
    snippets = []

    # Pattern 1: eval("...") - single-line with regular string
    # Matches eval(" followed by content (handling escapes) followed by ")
    for match in re.finditer(r'eval\("((?:[^"\\]|\\.)*)"\)', content):
        code = match.group(1)
        # Unescape common escape sequences
        code = code.replace('\\"', '"')
        code = code.replace('\\n', '\n')
        code = code.replace('\\t', '\t')
        code = code.replace('\\r', '\r')
        code = code.replace('\\\\', '\\')
        snippets.append(code)

    # Pattern 2: eval_result("...") - single-line
    for match in re.finditer(r'eval_result\("((?:[^"\\]|\\.)*)"\)', content):
        code = match.group(1)
        code = code.replace('\\"', '"')
        code = code.replace('\\n', '\n')
        code = code.replace('\\t', '\t')
        code = code.replace('\\r', '\r')
        code = code.replace('\\\\', '\\')
        snippets.append(code)

    # Pattern 3: throws_error("...", ...) - first argument only
    for match in re.finditer(r'throws_error\("((?:[^"\\]|\\.)*)"\s*,', content):
        code = match.group(1)
        code = code.replace('\\"', '"')
        code = code.replace('\\n', '\n')
        code = code.replace('\\t', '\t')
        code = code.replace('\\r', '\r')
        code = code.replace('\\\\', '\\')
        snippets.append(code)

    # Pattern 4: eval(r#"..."#) - multiline raw strings
    for match in re.finditer(r'eval\(r#"(.*?)"#\)', content, re.DOTALL):
        snippets.append(match.group(1))

    # Pattern 5: eval_result(r#"..."#) - multiline raw strings
    for match in re.finditer(r'eval_result\(r#"(.*?)"#\)', content, re.DOTALL):
        snippets.append(match.group(1))

    # Pattern 6: throws_error(r#"..."#, ...) - multiline raw strings
    for match in re.finditer(r'throws_error\(r#"(.*?)"#\s*,', content, re.DOTALL):
        snippets.append(match.group(1))

    return snippets


def hash_content(content: str) -> str:
    """Generate a short hash for deduplication."""
    return hashlib.sha256(content.encode()).hexdigest()[:16]


def main():
    # Create corpus directories
    for name in ["lexer", "parser", "interpreter"]:
        (CORPUS_DIR / name).mkdir(parents=True, exist_ok=True)

    all_snippets: list[str] = []

    # Process all test files
    if not TEST_DIR.exists():
        print(f"Test directory not found: {TEST_DIR}")
        return

    for test_file in sorted(TEST_DIR.glob("*.rs")):
        content = test_file.read_text()
        snippets = extract_snippets(content)
        all_snippets.extend(snippets)
        if snippets:
            print(f"{test_file.name}: {len(snippets)} snippets")

    # Deduplicate by content hash
    seen: set[str] = set()
    unique: list[tuple[str, str]] = []
    for snippet in all_snippets:
        # Skip empty or whitespace-only snippets
        if not snippet.strip():
            continue
        h = hash_content(snippet)
        if h not in seen:
            seen.add(h)
            unique.append((h, snippet))

    # Write corpus files to all directories (same corpus for lexer/parser/interpreter)
    for name in ["lexer", "parser", "interpreter"]:
        corpus_dir = CORPUS_DIR / name
        for h, snippet in unique:
            (corpus_dir / h).write_text(snippet)

    print(f"\nTotal: {len(all_snippets)} snippets extracted")
    print(f"Unique: {len(unique)} after deduplication")
    print(f"Written to: {CORPUS_DIR}/{{lexer,parser,interpreter}}/")


if __name__ == "__main__":
    main()
