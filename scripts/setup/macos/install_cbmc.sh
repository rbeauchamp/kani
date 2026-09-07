#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Source kani-dependencies to get CBMC_VERSION
source kani-dependencies

if [ -z "${CBMC_VERSION:-}" ]; then
  echo "$0: Error: CBMC_VERSION is not specified"
  exit 1
fi

# Install the fully qualified formula so Homebrew taps and trusts only this item.
# A separate `brew tap` can evaluate other untrusted formulae on CI runners.
# https://docs.brew.sh/Tap-Trust
# https://github.com/diffblue/cbmc/blob/develop/doc/ADR/homebrew_tap.md
brew install --overwrite "diffblue/cbmc/cbmc@${CBMC_VERSION}"
