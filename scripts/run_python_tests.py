from pathlib import Path
import subprocess
import sys


def run(*args: str) -> None:
    subprocess.run(
        args, check=True, cwd=Path(__file__).resolve().parents[1] / "python/pyasic-rs"
    )


def main() -> int:
    uv = (sys.executable, "-m", "uv")
    run(
        *uv,
        "run",
        "--with",
        "maturin",
        "--extra",
        "test",
        "maturin",
        "develop",
        "--features",
        "python",
        "--extras",
        "test",
    )
    run(*uv, "run", "--extra", "test", "pytest", "tests")
    return 0


if __name__ == "__main__":
    sys.exit(main())
