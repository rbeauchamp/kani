#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Fail-closed artifact and consumer gate for the downstream core-v1 profile."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import re
import signal
import subprocess
import sys
from typing import Any


SUMMARY_RE = re.compile(
    r"^Complete - ([0-9]+) successfully verified harnesses, "
    r"([0-9]+) failures, ([0-9]+) total\.$",
    re.MULTILINE,
)
CHECK_RE = re.compile(
    r"^(?:Thread [0-9]+: )?Checking harness ([A-Za-z0-9_:]+)\.\.\.$",
    re.MULTILINE,
)
FAILED_CHECK_RE = re.compile(
    r"^ \*\* ([0-9]+) of ([0-9]+) failed(?: \(([0-9]+) unreachable\))?$",
    re.MULTILINE,
)
COVER_RE = re.compile(
    r"^ \*\* ([0-9]+) of ([0-9]+) cover properties satisfied$",
    re.MULTILINE,
)
WARNING_RE = re.compile(r"^warning: (.+)$", re.MULTILINE)
UNSUPPORTED_HEADER = "warning: Found the following unsupported constructs:"
UNSUPPORTED_RE = re.compile(r"^\s+- ([a-zA-Z0-9_ -]+) \([0-9]+\)$")

PROHIBITED_EXACT_ARGS = {
    "--export-json",
    "--fail-fast",
    "--jobs",
    "--no-codegen",
    "--only-codegen",
    "--quiet",
    "--restrict-vtable",
    "-j",
    "-q",
    "autoharness",
    "playback",
}
PROHIBITED_ARG_PREFIXES = (
    "--concrete-playback",
    "--export-json=",
    "--jobs=",
    "--quiet=",
    "--restrict-vtable=",
    "-j",
)
PROHIBITED_UNSTABLE_FEATURES = {
    "autoharness",
    "c-ffi",
    "concrete-playback",
    "function-contracts",
    "loop-contracts",
    "quantifiers",
    "restrict-vtable",
    "uninit-checks",
    "valid-value-checks",
}
GIT_OVERRIDE_ENV = {
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_WORK_TREE",
}


class GateError(Exception):
    """A release-contract violation."""


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: pathlib.Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise GateError(f"cannot read valid JSON from {path}: {error}") from error
    if not isinstance(value, dict):
        raise GateError(f"{path} must contain a JSON object")
    return value


def run(
    command: list[str],
    cwd: pathlib.Path,
    *,
    timeout_seconds: int,
    display: bool = False,
) -> subprocess.CompletedProcess[str]:
    try:
        process = subprocess.Popen(
            command,
            cwd=cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
    except OSError as error:
        raise GateError(f"could not execute {command[0]}: {error}") from error
    try:
        output, _ = process.communicate(timeout=timeout_seconds)
    except subprocess.TimeoutExpired as error:
        try:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=5)
        except (OSError, subprocess.TimeoutExpired):
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except OSError:
                pass
            process.wait()
        raise GateError(
            f"command timed out after {timeout_seconds}s: {' '.join(command)}"
        ) from error
    result = subprocess.CompletedProcess(command, process.returncode, output)
    if display:
        print(output, end="" if output.endswith("\n") else "\n")
    return result


def command_output(command: list[str], cwd: pathlib.Path) -> str:
    result = run(command, cwd, timeout_seconds=60)
    if result.returncode != 0:
        raise GateError(
            f"command exited nonzero ({result.returncode}): {' '.join(command)}\n"
            f"{result.stdout[-4000:]}"
        )
    return result.stdout.strip()


def require_keys(document: dict[str, Any], expected: set[str], label: str) -> None:
    if set(document) != expected:
        raise GateError(
            f"{label} schema changed: expected {sorted(expected)}, "
            f"found {sorted(document)}"
        )


def validate_toolchain_manifest(document: dict[str, Any]) -> None:
    require_keys(
        document,
        {
            "schema",
            "profile",
            "kani_base",
            "kani_tree",
            "cargo_kani_version",
            "cbmc_version",
            "kissat_version",
            "platforms",
        },
        "toolchain manifest",
    )
    if document["schema"] != 1 or document["profile"] != "core-v1":
        raise GateError("unsupported toolchain manifest schema or profile")
    if not re.fullmatch(r"[0-9a-f]{40}", document["kani_base"]):
        raise GateError("toolchain Kani base must be a full 40-character SHA")
    if not re.fullmatch(r"[0-9a-f]{40}", document["kani_tree"]):
        raise GateError("toolchain Kani tree must be a full 40-character SHA")
    if not isinstance(document["platforms"], dict) or not document["platforms"]:
        raise GateError("toolchain manifest must define at least one platform")


def ledger_values(entries: Any, *, field: str, label: str) -> list[str]:
    if not isinstance(entries, list):
        raise GateError(f"{label} ledger must be a list")
    values: list[str] = []
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {field, "disposition"}:
            raise GateError(f"{label} ledger entry schema changed")
        value = entry[field]
        disposition = entry["disposition"]
        if not isinstance(value, str) or not value:
            raise GateError(f"{label} ledger has an empty {field}")
        if not isinstance(disposition, str) or not disposition.strip():
            raise GateError(f"{label} ledger has an empty disposition")
        values.append(value)
    if len(values) != len(set(values)):
        raise GateError(f"{label} ledger contains duplicates")
    return sorted(values)


def validate_consumer_manifest(document: dict[str, Any]) -> None:
    require_keys(
        document,
        {
            "schema",
            "profile",
            "status",
            "consumer",
            "repository",
            "source_commit",
            "source_tree",
            "project_dir",
            "cargo_manifest",
            "cargo_config",
            "kani_flags",
            "expected_harnesses",
            "expected_cover_properties",
            "expected_unreachable_checks",
            "unreachable_disposition",
            "diagnostics",
        },
        "consumer manifest",
    )
    if document["schema"] != 1 or document["profile"] != "core-v1":
        raise GateError("unsupported consumer manifest schema or profile")
    if document["status"] not in {"bootstrap", "qualified"}:
        raise GateError("consumer status must be bootstrap or qualified")
    for key in ("source_commit", "source_tree"):
        if not re.fullmatch(r"[0-9a-f]{40}", document[key]):
            raise GateError(f"consumer {key} must be a full 40-character SHA")
    harnesses = document["expected_harnesses"]
    if (
        not isinstance(harnesses, list)
        or not harnesses
        or not all(isinstance(name, str) and name for name in harnesses)
        or len(harnesses) != len(set(harnesses))
    ):
        raise GateError("expected harnesses must be a nonempty unique string list")
    if not isinstance(document["kani_flags"], list) or not all(
        isinstance(flag, str) and flag for flag in document["kani_flags"]
    ):
        raise GateError("Kani flags must be a string list")
    if document["kani_flags"]:
        raise GateError(
            "core-v1 currently qualifies the default verification flags only"
        )
    validate_invocation(document["kani_flags"])
    if (
        not isinstance(document["expected_cover_properties"], int)
        or document["expected_cover_properties"] < 0
    ):
        raise GateError("consumer expected_cover_properties must be nonnegative")
    unreachable = document["expected_unreachable_checks"]
    if not isinstance(unreachable, dict) or any(
        name not in harnesses or not isinstance(count, int) or count <= 0
        for name, count in unreachable.items()
    ):
        raise GateError(
            "expected unreachable checks must map known harnesses to positive counts"
        )
    disposition = document["unreachable_disposition"]
    if not unreachable and disposition is not None:
        raise GateError("zero unreachable checks requires a null disposition")
    if unreachable and (not isinstance(disposition, str) or not disposition.strip()):
        raise GateError("unreachable checks require a nonempty disposition")
    diagnostics = document["diagnostics"]
    if not isinstance(diagnostics, dict) or set(diagnostics) != {
        "warnings",
        "unsupported_constructs",
    }:
        raise GateError("diagnostic ledger schema changed")
    ledger_values(diagnostics["warnings"], field="message", label="warning")
    ledger_values(
        diagnostics["unsupported_constructs"],
        field="construct",
        label="unsupported construct",
    )


def validate_invocation(arguments: list[str]) -> None:
    for index, argument in enumerate(arguments):
        if argument in PROHIBITED_EXACT_ARGS or argument.startswith(PROHIBITED_ARG_PREFIXES):
            raise GateError(f"core-v1 prohibits Kani argument {argument!r}")
        if argument.startswith("--output-format=") and argument != "--output-format=terse":
            raise GateError("core-v1 requires --output-format=terse")
        if argument == "--output-format":
            if index + 1 >= len(arguments) or arguments[index + 1] != "terse":
                raise GateError("core-v1 requires --output-format=terse")
        feature = None
        if argument == "-Z":
            if index + 1 >= len(arguments):
                raise GateError("-Z is missing its feature name")
            feature = arguments[index + 1]
        elif argument.startswith("-Z="):
            feature = argument[3:]
        elif argument.startswith("-Z") and len(argument) > 2:
            feature = argument[2:]

        if feature is not None and feature in PROHIBITED_UNSTABLE_FEATURES:
            raise GateError(
                f"core-v1 prohibits unstable feature {feature!r}"
            )


def verify_git_identity(root: pathlib.Path, manifest: dict[str, Any]) -> dict[str, str]:
    overrides = sorted(name for name in GIT_OVERRIDE_ENV if os.environ.get(name))
    if overrides:
        raise GateError(f"unexpected Git environment overrides: {', '.join(overrides)}")
    if not (root / ".git").exists():
        raise GateError(f"consumer checkout has no Git metadata: {root}")
    head = command_output(["git", "rev-parse", "HEAD"], root)
    tree = command_output(["git", "rev-parse", "HEAD^{tree}"], root)
    status = command_output(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"], root
    )
    replacements = command_output(["git", "replace", "-l"], root)
    if head != manifest["source_commit"] or tree != manifest["source_tree"]:
        raise GateError(
            f"consumer identity mismatch: observed {head}/{tree}, expected "
            f"{manifest['source_commit']}/{manifest['source_tree']}"
        )
    if status:
        raise GateError(f"consumer checkout is not clean before execution:\n{status}")
    if replacements:
        raise GateError(f"consumer checkout has replacement refs:\n{replacements}")
    return {"commit": head, "tree": tree}


def verify_artifacts(
    toolchain: dict[str, Any], platform: str, artifact_dir: pathlib.Path
) -> dict[str, dict[str, Any]]:
    platforms = toolchain["platforms"]
    if platform not in platforms:
        raise GateError(f"platform {platform!r} is not declared in the toolchain manifest")
    platform_manifest = platforms[platform]
    if not isinstance(platform_manifest, dict) or set(platform_manifest) != {
        "artifacts",
        "runner",
    }:
        raise GateError(f"platform manifest schema changed for {platform}")
    artifacts = platform_manifest["artifacts"]
    if not isinstance(artifacts, list) or not artifacts:
        raise GateError(f"platform {platform} has no artifacts")
    observed: dict[str, dict[str, Any]] = {}
    for artifact in artifacts:
        if not isinstance(artifact, dict) or set(artifact) != {"name", "sha256"}:
            raise GateError(f"artifact schema changed for {platform}")
        name = artifact["name"]
        expected = artifact["sha256"]
        if not isinstance(name, str) or not re.fullmatch(r"[0-9a-f]{64}", expected):
            raise GateError(f"invalid artifact entry for {platform}")
        path = artifact_dir / name
        if not path.is_file():
            raise GateError(f"required artifact is missing: {path}")
        actual = sha256_file(path)
        if actual != expected:
            raise GateError(
                f"artifact hash mismatch for {name}: observed {actual}, expected {expected}"
            )
        observed[name] = {"sha256": actual, "size": path.stat().st_size}
    return observed


def verify_tool_versions(toolchain: dict[str, Any], root: pathlib.Path) -> dict[str, str]:
    cargo_kani = command_output(["cargo-kani", "--version", "--verbose"], root)
    if cargo_kani != toolchain["cargo_kani_version"]:
        raise GateError(
            "cargo-kani identity mismatch:\n"
            f"observed:\n{cargo_kani}\nexpected:\n{toolchain['cargo_kani_version']}"
        )
    match = re.search(r"Kani Rust Verifier ([0-9]+\.[0-9]+\.[0-9]+)", cargo_kani)
    if match is None:
        raise GateError("could not derive Kani version from cargo-kani output")
    kani_home_text = os.environ.get("KANI_HOME")
    if not kani_home_text:
        raise GateError("KANI_HOME must be set explicitly")
    binary_dir = pathlib.Path(kani_home_text) / f"kani-{match.group(1)}" / "bin"
    cbmc = command_output([str(binary_dir / "cbmc"), "--version"], root)
    kissat = command_output([str(binary_dir / "kissat"), "--version"], root)
    if cbmc != toolchain["cbmc_version"]:
        raise GateError(
            f"runtime CBMC mismatch: observed {cbmc!r}, expected {toolchain['cbmc_version']!r}"
        )
    if kissat != toolchain["kissat_version"]:
        raise GateError(
            f"runtime Kissat mismatch: observed {kissat!r}, expected {toolchain['kissat_version']!r}"
        )
    return {"cargo_kani": cargo_kani, "cbmc": cbmc, "kissat": kissat}


def inventory_harnesses(document: dict[str, Any], expected_version: str) -> list[str]:
    require_keys(
        document,
        {
            "kani-version",
            "file-version",
            "standard-harnesses",
            "contract-harnesses",
            "contracts",
            "totals",
        },
        "Kani inventory",
    )
    if document["kani-version"] != expected_version or document["file-version"] != "0.1":
        raise GateError("Kani inventory version changed")
    tables = document["standard-harnesses"]
    if not isinstance(tables, dict) or not tables:
        raise GateError("Kani standard-harness inventory is empty or malformed")
    if document["contract-harnesses"] != {} or document["contracts"] != []:
        raise GateError("contract harnesses require a different qualified profile")
    harnesses: list[str] = []
    for path, names in tables.items():
        if not isinstance(path, str) or not isinstance(names, list):
            raise GateError("Kani standard-harness inventory is malformed")
        if not names or not all(isinstance(name, str) and name for name in names):
            raise GateError("Kani standard-harness inventory is malformed")
        harnesses.extend(names)
    if len(harnesses) != len(set(harnesses)):
        raise GateError("Kani harness names are not unique")
    totals = document["totals"]
    if not isinstance(totals, dict) or totals.get("standard-harnesses") != len(harnesses):
        raise GateError("Kani inventory total does not match its harness names")
    if totals.get("contract-harnesses") != 0 or totals.get("functions-under-contract") != 0:
        raise GateError("Kani inventory totals require unsupported contract handling")
    return sorted(harnesses)


def parse_diagnostics(output: str) -> tuple[list[str], list[str]]:
    warnings = sorted(
        set(
            message
            for message in WARNING_RE.findall(output)
            if not message.startswith("Found the following unsupported constructs")
        )
    )
    unsupported: set[str] = set()
    in_unsupported = False
    for line in output.splitlines():
        if line == UNSUPPORTED_HEADER:
            in_unsupported = True
            continue
        if in_unsupported:
            match = UNSUPPORTED_RE.match(line)
            if match is not None:
                unsupported.add(match.group(1))
                continue
            if "Verification will fail if" in line:
                in_unsupported = False
    return warnings, sorted(unsupported)


def parse_verification_output(
    output: str,
    expected_harnesses: list[str],
    expected_covers: int,
    expected_unreachable: dict[str, int],
) -> dict[str, Any]:
    count = len(expected_harnesses)
    summaries = SUMMARY_RE.findall(output)
    if summaries != [(str(count), "0", str(count))]:
        raise GateError(f"Kani emitted no unique all-green {count}-harness summary")
    checked = CHECK_RE.findall(output)
    if len(checked) != len(set(checked)) or sorted(checked) != sorted(expected_harnesses):
        raise GateError("checked-harness output does not equal the requested set")
    if output.count("VERIFICATION:- SUCCESSFUL") != count:
        raise GateError("successful-verification markers do not equal the harness count")
    failed_checks = FAILED_CHECK_RE.findall(output)
    if len(failed_checks) != count or any(failed != "0" for failed, _, _ in failed_checks):
        raise GateError("failed-check summaries are missing or nonzero")
    if len(checked) != len(failed_checks):
        raise GateError("harness and failed-check summaries have different lengths")
    observed_unreachable = {
        harness: int(value)
        for harness, (_, _, value) in zip(checked, failed_checks)
        if value and int(value) > 0
    }
    if observed_unreachable != expected_unreachable:
        raise GateError(
            "unreachable-check distribution changed: "
            f"observed {observed_unreachable}, expected {expected_unreachable}"
        )
    cover_results = COVER_RE.findall(output)
    satisfied = sum(int(value) for value, _ in cover_results)
    covers = sum(int(value) for _, value in cover_results)
    if satisfied != covers or covers != expected_covers:
        raise GateError(
            f"cover obligations changed or failed: observed {satisfied}/{covers}, "
            f"expected {expected_covers}/{expected_covers}"
        )
    forbidden_markers = ("UNDETERMINED", "VERIFICATION:- FAILED", "VERIFICATION ERROR")
    present = [marker for marker in forbidden_markers if marker in output]
    if present:
        raise GateError(f"forbidden result marker(s): {', '.join(present)}")
    return {
        "summary": {"successful": count, "failed": 0, "total": count},
        "checked_harnesses": sorted(checked),
        "cover_properties": covers,
        "unreachable_checks": {
            "total": sum(observed_unreachable.values()),
            "by_harness": dict(sorted(observed_unreachable.items())),
        },
    }


def cargo_prefix(project: pathlib.Path, manifest: dict[str, Any]) -> list[str]:
    command = ["cargo"]
    config = manifest["cargo_config"]
    if config is not None:
        config_path = project / config
        if not config_path.is_file():
            raise GateError(f"declared Cargo config is missing: {config_path}")
        command.extend(["--config", str(config_path)])
    cargo_manifest = project / manifest["cargo_manifest"]
    if not cargo_manifest.is_file():
        raise GateError(f"declared Cargo manifest is missing: {cargo_manifest}")
    command.extend(["kani", "--manifest-path", str(cargo_manifest)])
    return command


def execute_gate(args: argparse.Namespace) -> dict[str, Any]:
    toolchain_path = args.toolchain.resolve()
    consumer_path = args.consumer.resolve()
    checkout = args.checkout.resolve()
    artifact_dir = args.artifact_dir.resolve()
    receipt_path = args.receipt.resolve()
    toolchain = load_json(toolchain_path)
    consumer = load_json(consumer_path)
    validate_toolchain_manifest(toolchain)
    validate_consumer_manifest(consumer)
    if consumer["profile"] != toolchain["profile"]:
        raise GateError("toolchain and consumer profiles differ")

    project = (checkout / consumer["project_dir"]).resolve()
    try:
        project.relative_to(checkout)
    except ValueError as error:
        raise GateError("consumer project directory escapes its checkout") from error
    git_identity = verify_git_identity(checkout, consumer)
    artifacts = verify_artifacts(toolchain, args.platform, artifact_dir)
    tools = verify_tool_versions(toolchain, project)
    version_match = re.search(
        r"Kani Rust Verifier ([0-9]+\.[0-9]+\.[0-9]+)", tools["cargo_kani"]
    )
    assert version_match is not None

    prefix = cargo_prefix(project, consumer)
    list_command = prefix + ["list", "--format", "json"]
    list_result = run(list_command, project, timeout_seconds=600, display=True)
    if list_result.returncode != 0:
        raise GateError(
            f"cargo kani list exited nonzero ({list_result.returncode})\n"
            f"{list_result.stdout[-4000:]}"
        )
    inventory_path = project / "kani-list.json"
    if not inventory_path.is_file():
        raise GateError("cargo kani list did not produce kani-list.json")
    actual_harnesses = inventory_harnesses(
        load_json(inventory_path), version_match.group(1)
    )
    inventory_path.unlink()
    expected_harnesses = sorted(consumer["expected_harnesses"])
    if actual_harnesses != expected_harnesses:
        raise GateError(
            "Kani harness inventory drifted\n"
            f"observed: {json.dumps(actual_harnesses, indent=2)}\n"
            f"expected: {json.dumps(expected_harnesses, indent=2)}"
        )

    verify_flags = consumer["kani_flags"]
    verify_command = prefix + ["--exact", "--output-format=terse"] + verify_flags
    for harness in expected_harnesses:
        verify_command.extend(["--harness", harness])
    validate_invocation(verify_command)
    verify_result = run(
        verify_command,
        project,
        timeout_seconds=args.timeout_seconds,
        display=True,
    )
    if verify_result.returncode != 0:
        raise GateError(
            f"cargo kani exited nonzero ({verify_result.returncode})\n"
            f"{verify_result.stdout[-4000:]}"
        )
    result = parse_verification_output(
        verify_result.stdout,
        expected_harnesses,
        consumer["expected_cover_properties"],
        consumer["expected_unreachable_checks"],
    )

    combined_output = list_result.stdout + "\n" + verify_result.stdout
    warnings, unsupported = parse_diagnostics(combined_output)
    expected_warnings = ledger_values(
        consumer["diagnostics"]["warnings"], field="message", label="warning"
    )
    expected_unsupported = ledger_values(
        consumer["diagnostics"]["unsupported_constructs"],
        field="construct",
        label="unsupported construct",
    )
    if warnings != expected_warnings:
        raise GateError(
            f"warning ledger mismatch: observed {warnings!r}, expected {expected_warnings!r}"
        )
    if unsupported != expected_unsupported:
        raise GateError(
            "unsupported-construct ledger mismatch: "
            f"observed {unsupported!r}, expected {expected_unsupported!r}"
        )

    receipt = {
        "schema": 1,
        "status": "pass",
        "qualification_status": consumer["status"],
        "executed_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "platform": args.platform,
        "runner": toolchain["platforms"][args.platform]["runner"],
        "profile": toolchain["profile"],
        "consumer": consumer["consumer"],
        "repository": consumer["repository"],
        "source": git_identity,
        "kani": {"commit": toolchain["kani_base"], "tree": toolchain["kani_tree"]},
        "manifest_sha256": {
            "toolchain": sha256_file(toolchain_path),
            "consumer": sha256_file(consumer_path),
        },
        "artifacts": artifacts,
        "tools": tools,
        "command": verify_command,
        "verification_output_sha256": hashlib.sha256(
            verify_result.stdout.encode("utf-8")
        ).hexdigest(),
        "warnings": warnings,
        "unsupported_constructs": unsupported,
        "result": result,
    }
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    receipt_path.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return receipt


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--toolchain", type=pathlib.Path, required=True)
    result.add_argument("--consumer", type=pathlib.Path, required=True)
    result.add_argument("--checkout", type=pathlib.Path, required=True)
    result.add_argument("--artifact-dir", type=pathlib.Path, required=True)
    result.add_argument("--platform", required=True)
    result.add_argument("--receipt", type=pathlib.Path, required=True)
    result.add_argument("--timeout-seconds", type=int, default=7200)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        receipt = execute_gate(args)
        print(
            f"CORE-V1 GATE: PASS ({receipt['consumer']}: "
            f"{receipt['result']['summary']['total']} harnesses)"
        )
        return 0
    except GateError as error:
        print(f"CORE-V1 GATE: VIOLATION: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
