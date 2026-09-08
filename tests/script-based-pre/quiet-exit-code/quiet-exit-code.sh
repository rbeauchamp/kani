#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Regression test for https://github.com/model-checking/kani/issues/4745:
# `--quiet` must suppress output but never the exit status. A failing
# verification under `--quiet` must exit nonzero; a passing one must exit 0;
# both must produce no output.

set -eu

cd $(dirname $0)
rm -f quiet-fail.out quiet-pass.out

echo "Checking --quiet on a failing harness..."
if kani --quiet failing.rs > quiet-fail.out 2>&1; then
    echo "Error: kani --quiet on a failing harness exited 0 (failure masked as pass)."
    exit 1
fi
if [ -s quiet-fail.out ]; then
    echo "Error: kani --quiet produced output on the failing run:"
    cat quiet-fail.out
    exit 1
fi

echo "Checking --quiet on a passing harness..."
if ! kani --quiet passing.rs > quiet-pass.out 2>&1; then
    echo "Error: kani --quiet on a passing harness exited nonzero."
    exit 1
fi
if [ -s quiet-pass.out ]; then
    echo "Error: kani --quiet produced output on the passing run:"
    cat quiet-pass.out
    exit 1
fi

rm -f quiet-fail.out quiet-pass.out
echo "Finished quiet exit-code check successfully."
