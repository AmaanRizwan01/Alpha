"""Strategy Lab Command Line Interface.

Usage:
  python -m python.research.lab setup [--force] [--skip-tests]
  python -m python.research.lab extract <youtube_url> [name]
  python -m python.research.lab cemetery
"""

import sys
from pathlib import Path

from python.research.lab.extractor import extract_and_save


def main():
    if len(sys.argv) < 2:
        print("Usage:")
        print("  python -m python.research.lab setup [--force] [--skip-tests]")
        print("  python -m python.research.lab extract <youtube_url> [slug_name]")
        print("  python -m python.research.lab cemetery")
        return

    command = sys.argv[1].lower()

    if command == "setup":
        from python.research.setup import run_setup
        force = "--force" in sys.argv
        skip_tests = "--skip-tests" in sys.argv
        sys.exit(run_setup(force=force, skip_tests=skip_tests))

    elif command == "extract":
        if len(sys.argv) < 3:
            print("Error: Missing YouTube URL.")
            print("Example: python -m python.research.lab extract https://youtube.com/watch?v=xyz [name]")
            return
        url = sys.argv[2]
        name = sys.argv[3] if len(sys.argv) > 3 else None
        try:
            out_dir = extract_and_save(url, name)
            print(f"[+] Successfully extracted transcript.")
            print(f"[+] Draft files created at: {out_dir}")
            print(f"[+] Review 'extraction_prompt.txt' to generate strategy code.")
        except Exception as exc:
            print(f"[!] Extraction failed: {exc}")

    elif command == "cemetery":
        killed_doc = Path("docs/backtests/KILLED_STRATEGIES.md")
        if killed_doc.exists():
            print(killed_doc.read_text(encoding="utf-8"))
        else:
            print("No killed strategies recorded yet.")
    else:
        print(f"Unknown command: {command}")


if __name__ == "__main__":
    main()
