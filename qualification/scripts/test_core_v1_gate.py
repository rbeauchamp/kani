#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Unit tests for the core-v1 qualification gate."""

from __future__ import annotations

import importlib.util
import pathlib
import unittest


MODULE_PATH = pathlib.Path(__file__).with_name("core_v1_gate.py")
SPEC = importlib.util.spec_from_file_location("core_v1_gate", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class InvocationPolicyTests(unittest.TestCase):
    def test_accepts_the_gate_invocation(self) -> None:
        GATE.validate_invocation(
            [
                "cargo",
                "kani",
                "--exact",
                "--output-format=terse",
                "--harness",
                "proofs::example",
            ]
        )

    def test_rejects_prohibited_flags_and_subcommands(self) -> None:
        examples = [
            ["--quiet"],
            ["-q"],
            ["--export-json=result.json"],
            ["--fail-fast"],
            ["--jobs=2"],
            ["-j2"],
            ["autoharness"],
            ["--output-format=old"],
            ["--output-format=regular"],
            ["-Z", "quantifiers"],
        ]
        for arguments in examples:
            with self.subTest(arguments=arguments):
                with self.assertRaises(GATE.GateError):
                    GATE.validate_invocation(arguments)

    def test_rejects_a_split_non_terse_output_format(self) -> None:
        with self.assertRaisesRegex(GATE.GateError, "output-format=terse"):
            GATE.validate_invocation(["--output-format", "regular"])


class ParserTests(unittest.TestCase):
    def test_parses_exact_diagnostic_sets(self) -> None:
        output = """warning: use of an unstable feature
warning: Found the following unsupported constructs:
             - caller_location (1)
             - foreign function (2)

         Verification will fail if one or more of these constructs is reachable.
warning: use of an unstable feature
"""
        warnings, unsupported = GATE.parse_diagnostics(output)
        self.assertEqual(warnings, ["use of an unstable feature"])
        self.assertEqual(unsupported, ["caller_location", "foreign function"])

    def test_accepts_a_complete_successful_result(self) -> None:
        output = """Checking harness proofs::one...

VERIFICATION RESULT:
 ** 0 of 4 failed (1 unreachable)

 ** 2 of 2 cover properties satisfied

VERIFICATION:- SUCCESSFUL
Checking harness proofs::two...

VERIFICATION RESULT:
 ** 0 of 3 failed

VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 2 successfully verified harnesses, 0 failures, 2 total.
"""
        result = GATE.parse_verification_output(
            output,
            ["proofs::one", "proofs::two"],
            expected_covers=2,
            expected_unreachable={"proofs::one": 1},
        )
        self.assertEqual(result["summary"]["total"], 2)
        self.assertEqual(result["cover_properties"], 2)
        self.assertEqual(result["unreachable_checks"]["total"], 1)

    def test_rejects_partial_or_ambiguous_results(self) -> None:
        output = """Checking harness proofs::one...
 ** 0 of 4 failed
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"""
        with self.assertRaises(GATE.GateError):
            GATE.parse_verification_output(
                output,
                ["proofs::one", "proofs::two"],
                expected_covers=0,
                expected_unreachable={},
            )

    def test_parses_a_standard_inventory(self) -> None:
        document = {
            "kani-version": "0.67.0",
            "file-version": "0.1",
            "standard-harnesses": {"src/lib.rs": ["proofs::one", "proofs::two"]},
            "contract-harnesses": {},
            "contracts": [],
            "totals": {
                "standard-harnesses": 2,
                "contract-harnesses": 0,
                "functions-under-contract": 0,
            },
        }
        self.assertEqual(
            GATE.inventory_harnesses(document, "0.67.0"),
            ["proofs::one", "proofs::two"],
        )


class CheckedInManifestTests(unittest.TestCase):
    def test_synthetic_application_manifest_validates(self) -> None:
        manifest = {
            "schema": 1,
            "profile": "core-v1",
            "status": "bootstrap",
            "consumer": "synthetic-example",
            "repository": "https://example.invalid/repository",
            "source_commit": "0" * 40,
            "source_tree": "1" * 40,
            "project_dir": ".",
            "cargo_manifest": "Cargo.toml",
            "cargo_config": None,
            "kani_flags": [],
            "expected_harnesses": ["proofs::example"],
            "expected_cover_properties": 0,
            "expected_unreachable_checks": {},
            "unreachable_disposition": None,
            "diagnostics": {
                "warnings": [],
                "unsupported_constructs": [],
            },
        }
        GATE.validate_consumer_manifest(manifest)

    def test_all_checked_in_manifests_validate(self) -> None:
        manifests = MODULE_PATH.parents[1] / "manifests" / "core-v1"
        toolchain = GATE.load_json(manifests / "toolchain.json")
        GATE.validate_toolchain_manifest(toolchain)
        for path in sorted(manifests.glob("*.json")):
            if path.name == "toolchain.json":
                continue
            with self.subTest(path=path.name):
                consumer = GATE.load_json(path)
                GATE.validate_consumer_manifest(consumer)


if __name__ == "__main__":
    unittest.main()
