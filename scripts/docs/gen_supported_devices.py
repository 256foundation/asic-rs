#!/usr/bin/env python3
"""Generate the supported devices documentation page from Rust source."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_OUTPUT = REPO_ROOT / "docs" / "supported-devices.md"

SUPPORT_COLUMNS = [
    ("SupportsPoolsConfig", "supports_pools_config", "Pools Config"),
    ("SupportsScalingConfig", "supports_scaling_config", "Scaling Config"),
    ("SupportsTuningConfig", "supports_tuning_config", "Tuning Config"),
    ("SupportsFanConfig", "supports_fan_config", "Fan Config"),
    ("SetFaultLight", "supports_set_fault_light", "Light"),
    ("SetPowerLimit", "supports_set_power_limit", "Power Limit"),
    ("Restart", "supports_restart", "Restart"),
    ("Resume", "supports_resume", "Pause/Resume"),
    ("UpgradeFirmware", "supports_upgrade_firmware", "Upgrade FW"),
    ("ChangePassword", "supports_change_password", "Change PWD"),
    ("FactoryReset", "supports_factory_reset", "Factory Reset"),
    ("ReadLogs", "supports_read_logs", "Read Logs"),
]

SUPPORT_TRAITS = {trait for trait, _, _ in SUPPORT_COLUMNS}


@dataclass(frozen=True)
class SupportRow:
    make: str
    firmware: str
    backend: str
    support: dict[str, str]


@dataclass(frozen=True)
class FirmwareSupportRow:
    firmware: str
    support: dict[str, str]


@dataclass(frozen=True)
class ModelEntry:
    variant: str
    names: tuple[str, ...]


@dataclass(frozen=True)
class MakeModels:
    make: str
    models: tuple[ModelEntry, ...]


@dataclass(frozen=True)
class ModelFamily:
    name: str
    models: tuple[ModelEntry, ...]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def find_matching_brace(text: str, open_brace: int) -> int:
    depth = 0
    in_string = False
    escape = False
    line_comment = False
    block_comment = False

    for idx in range(open_brace, len(text)):
        char = text[idx]
        next_char = text[idx + 1] if idx + 1 < len(text) else ""

        if line_comment:
            if char == "\n":
                line_comment = False
            continue

        if block_comment:
            if char == "*" and next_char == "/":
                block_comment = False
            continue

        if in_string:
            if escape:
                escape = False
            elif char == "\\":
                escape = True
            elif char == '"':
                in_string = False
            continue

        if char == "/" and next_char == "/":
            line_comment = True
            continue
        if char == "/" and next_char == "*":
            block_comment = True
            continue
        if char == '"':
            in_string = True
            continue
        if char == "{":
            depth += 1
            continue
        if char == "}":
            depth -= 1
            if depth == 0:
                return idx

    raise ValueError("unmatched brace")


def strip_rust_comments(text: str) -> str:
    text = re.sub(r"//.*", "", text)
    return re.sub(r"/\*.*?\*/", "", text, flags=re.S)


def normalized_bool_body(body: str) -> str:
    body = strip_rust_comments(body).strip()
    body = body.removesuffix(";").strip()
    return " ".join(body.split())


def classify_support_body(body: str) -> str:
    normalized = normalized_bool_body(body)
    if normalized in {"true", "return true"}:
        return "Yes"
    if normalized in {"false", "return false"}:
        return "No"
    return "Conditional"


def extract_fn_body(block: str, method: str) -> str | None:
    match = re.search(
        rf"\bfn\s+{re.escape(method)}\s*\([^)]*\)\s*->\s*bool\s*\{{",
        block,
        flags=re.S,
    )
    if not match:
        return None

    open_brace = match.end() - 1
    close_brace = find_matching_brace(block, open_brace)
    return block[open_brace + 1: close_brace]


def iter_impl_blocks(text: str) -> tuple[tuple[str, str, str], ...]:
    blocks: list[tuple[str, str, str]] = []
    pattern = re.compile(
        r"\bimpl\s+(?P<trait>[A-Za-z_][A-Za-z0-9_]*)\s+for\s+"
        r"(?P<target>[A-Za-z_][A-Za-z0-9_]*)\s*\{"
    )

    for match in pattern.finditer(text):
        open_brace = match.end() - 1
        close_brace = find_matching_brace(text, open_brace)
        blocks.append(
            (
                match.group("trait"),
                match.group("target"),
                text[open_brace + 1: close_brace],
            )
        )

    return tuple(blocks)


def collect_backend_supports() -> dict[str, dict[str, str]]:
    supports: dict[str, dict[str, str]] = {}
    firmwares_dir = REPO_ROOT / "asic-rs-firmwares"

    for path in firmwares_dir.rglob("*.rs"):
        text = read_text(path)
        for trait, target, block in iter_impl_blocks(text):
            if trait not in SUPPORT_TRAITS:
                continue

            method = next(
                method_name
                for trait_name, method_name, _ in SUPPORT_COLUMNS
                if trait_name == trait
            )
            body = extract_fn_body(block, method)
            value = "No" if body is None else classify_support_body(body)
            supports.setdefault(target, {})[trait] = value

    return supports


def parse_display_string(path: Path) -> str | None:
    if not path.exists():
        return None

    match = re.search(r'write!\(\s*f\s*,\s*"([^"]+)"\s*\)', read_text(path))
    return match.group(1) if match else None


def parse_make_crate_names(firmware_rs: Path) -> tuple[str, ...]:
    text = read_text(firmware_rs)
    names = re.findall(r"asic_rs_makes_([A-Za-z0-9_]+)::make::", text)
    names.extend(
        re.findall(r"asic_rs_makes_([A-Za-z0-9_]+)::\{[^}]*\bmake::", text, flags=re.S)
    )
    return tuple(dict.fromkeys(names))


def make_display_name(make_crate: str) -> str:
    path = REPO_ROOT / "asic-rs-makes" / make_crate / "src" / "make.rs"
    return parse_display_string(path) or make_crate.replace("_", " ").title()


def make_display_names(make_crates: tuple[str, ...]) -> str:
    if not make_crates:
        return "Unknown"

    return ", ".join(make_display_name(make_crate) for make_crate in make_crates)


def concrete_backend_structs(
        firmware_crate: Path, supports: dict[str, dict[str, str]]
) -> list[str]:
    backend_dir = firmware_crate / "src" / "backends"
    if not backend_dir.exists():
        return []

    structs: list[str] = []
    for path in sorted(backend_dir.rglob("*.rs")):
        if path == backend_dir / "mod.rs":
            continue

        text = read_text(path)
        for match in re.finditer(
                r"\bpub(?:\(crate\))?\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{",
                text,
        ):
            name = match.group(1)
            if name in supports:
                structs.append(name)

    return sorted(set(structs), key=str.casefold)


def collect_support_rows() -> tuple[SupportRow, ...]:
    supports = collect_backend_supports()
    rows: list[SupportRow] = []

    for crate in sorted((REPO_ROOT / "asic-rs-firmwares").iterdir()):
        firmware_rs = crate / "src" / "firmware.rs"
        if not firmware_rs.exists():
            continue

        make = make_display_names(parse_make_crate_names(firmware_rs))
        firmware = parse_display_string(firmware_rs) or crate.name

        for backend in concrete_backend_structs(crate, supports):
            values = {
                trait: supports.get(backend, {}).get(trait, "No")
                for trait, _, _ in SUPPORT_COLUMNS
            }
            rows.append(
                SupportRow(make=make, firmware=firmware, backend=backend, support=values)
            )

    return tuple(
        sorted(rows, key=lambda r: (r.make.casefold(), r.firmware.casefold(), r.backend))
    )


def support_icon(values: tuple[str, ...]) -> str:
    if all(value == "Yes" for value in values):
        return ":lucide-check-check:"
    if all(value == "No" for value in values):
        return ":lucide-x:"
    return ":lucide-list-todo:"


def collect_firmware_support_rows() -> tuple[FirmwareSupportRow, ...]:
    grouped: dict[str, list[SupportRow]] = {}

    for row in collect_support_rows():
        grouped.setdefault(row.firmware, []).append(row)

    rows: list[FirmwareSupportRow] = []
    for firmware, support_rows in grouped.items():
        rows.append(
            FirmwareSupportRow(
                firmware=firmware,
                support={
                    trait: support_icon(
                        tuple(row.support[trait] for row in support_rows)
                    )
                    for trait, _, _ in SUPPORT_COLUMNS
                },
            )
        )

    return tuple(sorted(rows, key=lambda r: r.firmware.casefold()))


def enum_body(text: str) -> str | None:
    match = re.search(r"\bpub\s+enum\s+[A-Za-z_][A-Za-z0-9_]*\s*\{", text)
    if not match:
        return None

    open_brace = match.end() - 1
    close_brace = find_matching_brace(text, open_brace)
    return text[open_brace + 1: close_brace]


def parse_models(models_rs: Path) -> tuple[ModelEntry, ...]:
    body = enum_body(read_text(models_rs))
    if body is None:
        return ()

    models: list[ModelEntry] = []
    pending_attrs: list[str] = []

    for raw_line in body.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("//"):
            continue

        if line.startswith("#["):
            pending_attrs.append(line)
            continue

        match = re.match(r"([A-Za-z_][A-Za-z0-9_]*)(?:\([^)]*\))?\s*,", line)
        if not match:
            continue

        variant = match.group(1)
        attrs = "\n".join(pending_attrs)
        pending_attrs.clear()

        if variant == "Unknown":
            continue

        names = tuple(
            dict.fromkeys(
                re.findall(r'(?:alias|rename)\s*=\s*"([^"]+)"', attrs) or [variant]
            )
        )
        models.append(ModelEntry(variant=variant, names=names))

    return tuple(models)


def collect_make_models() -> tuple[MakeModels, ...]:
    makes: list[MakeModels] = []

    for crate in sorted((REPO_ROOT / "asic-rs-makes").iterdir()):
        models_rs = crate / "src" / "models.rs"
        if not models_rs.exists():
            continue

        make = parse_display_string(crate / "src" / "make.rs") or crate.name.replace(
            "_", " "
        ).title()
        models = parse_models(models_rs)
        if models:
            makes.append(MakeModels(make=make, models=models))

    return tuple(sorted(makes, key=lambda item: item.make.casefold()))


def table_escape(value: str) -> str:
    return value.replace("|", r"\|")


def natural_sort_key(value: str) -> tuple[object, ...]:
    parts = re.split(r"(\d+)", value.casefold())
    return tuple(int(part) if part.isdigit() else part for part in parts)


def slugify(value: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", value.casefold()).strip("-")
    return slug or "models"


def support_matrix_markdown(rows: tuple[FirmwareSupportRow, ...]) -> str:
    headers = ["Firmware type"]
    for column in SUPPORT_COLUMNS:
        _trait, _prop, label = column
        headers.append(label)

    lines = [
        "| " + " | ".join(table_escape(header) for header in headers) + " |",
        "| " + " | ".join("---" for _ in headers) + " |",
    ]
    for row in rows:
        values = [row.firmware]
        for trait, _, _ in SUPPORT_COLUMNS:
            values.append(row.support[trait])
        lines.append("| " + " | ".join(table_escape(value) for value in values) + " |")

    return "\n".join(lines)


def model_family_name(model: ModelEntry) -> str:
    text = model.names[0].replace("_", " ").upper()
    tokens = re.findall(r"[A-Z]*\d[A-Z0-9+.-]*|[A-Z]+", text)
    digit_index = next(
        (idx for idx, token in enumerate(tokens) if any(ch.isdigit() for ch in token)),
        None,
    )
    token = tokens[digit_index] if digit_index is not None else ""
    if not token:
        return tokens[0] if tokens else model.variant

    token = token.strip(".-")
    if match := re.match(r"^([A-Z]+)(\d+)", token):
        return f"{match.group(1)}{match.group(2)}"
    if match := re.match(r"^(\d+)", token):
        if digit_index and (previous := tokens[digit_index - 1]).isalpha():
            if len(previous) <= 5:
                return previous
        digits = match.group(1)
        if len(digits) >= 3:
            return f"{digits[:-2]}xx"
        return digits

    return token


def group_model_families(models: tuple[ModelEntry, ...]) -> tuple[ModelFamily, ...]:
    grouped: dict[str, list[ModelEntry]] = {}

    for model in models:
        grouped.setdefault(model_family_name(model), []).append(model)

    return tuple(
        ModelFamily(
            name=name,
            models=tuple(sorted(items, key=lambda item: natural_sort_key(item.names[0]))),
        )
        for name, items in sorted(grouped.items(), key=lambda item: natural_sort_key(item[0]))
    )


def family_summary_text(families: tuple[ModelFamily, ...]) -> str:
    family_label = "family" if len(families) == 1 else "families"
    visible = ", ".join(f"`{family.name}`" for family in families[:8])
    if len(families) > 8:
        visible = f"{visible}, ..."
    return f"{len(families)} {family_label}: {visible}"

def model_display(model: ModelEntry) -> str:
    primary, *aliases = model.names
    if aliases:
        alias_text = ", ".join(f"`{alias}`" for alias in aliases)
        return f"`{primary}` (also: {alias_text})"
    return f"`{primary}`"


def wrapped_inline_models(models: tuple[ModelEntry, ...], line_width: int = 120) -> str:
    lines: list[str] = []
    for model in models:
        lines.append(f"\t\t - [x] {model_display(model)}")
    return "\n".join(lines)


def exact_models_markdown(makes: tuple[MakeModels, ...]) -> str:
    lines: list[str] = []

    for make in makes:
        families = group_model_families(make.models)
        count = len(make.models)
        model_label = "model" if count == 1 else "models"
        family_label = "family" if len(families) == 1 else "families"
        lines.extend(
            [
                (
                    f"??? quote \"{make.make} ({count} {model_label} across "
                    f"{len(families)} {family_label})\""
                ),
                "",
            ]
        )

        for family in families:
            family_count = len(family.models)
            family_model_label = "model" if family_count == 1 else "models"
            lines.extend(
                [
                    (
                        f"\t??? note \"{family.name} family "
                        f"({family_count} {family_model_label})\""
                    ),
                    wrapped_inline_models(family.models),
                ]
            )

    return "\n".join(lines).rstrip()


def build_markdown() -> str:
    rows = collect_firmware_support_rows()
    makes = collect_make_models()

    return (
            "\n\n".join(
                [
                    "# Supported Devices",
                    (
                        "<!-- Generated by scripts/generate_supported_devices.py; "
                        "do not edit manually. -->"
                    ),
                    (
                        "This page is generated from the Rust source. The support matrix is "
                        "derived from each backend's `supports_...` methods; model lists are "
                        "derived from the make model enums."
                    ),
                    (
                        "Legend:\n\n"
                        " - :lucide-check-check: means every backend subtype for that firmware type supports the function\n"
                        " - :lucide-list-todo: means support is mixed or conditional\n"
                        " - :lucide-x: means no backend subtype supports it\n"
                    ),
                    "## Support Matrix\n\n" + support_matrix_markdown(rows),
                    "## Exact Supported Models\n\n" + exact_models_markdown(makes),
                    ]
            )
            + "\n"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=DEFAULT_OUTPUT,
        help=(
            "Markdown file to write. Defaults to "
            f"{DEFAULT_OUTPUT.relative_to(REPO_ROOT)}."
        ),
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Fail if the output file is not up to date.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    output = args.output if args.output.is_absolute() else REPO_ROOT / args.output
    content = build_markdown()

    if args.check:
        current = output.read_text(encoding="utf-8") if output.exists() else None
        if current != content:
            print(f"{output.relative_to(REPO_ROOT)} is not up to date", file=sys.stderr)
            return 1
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8", newline="\n")
    print(f"Wrote {output.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
