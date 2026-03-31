#!/usr/bin/env sh

set -eu

if [ "$#" -ne 4 ]; then
  printf '%s\n' "usage: $0 <version> <macos-arm64-sha256> <macos-x86_64-sha256> <linux-x86_64-sha256>" >&2
  exit 1
fi

version="$1"
macos_arm64_sha="$2"
macos_x86_64_sha="$3"
linux_x86_64_sha="$4"
template="packaging/homebrew/gshell.rb.in"

sed \
  -e "s/__VERSION__/${version}/g" \
  -e "s/__SHA256_AARCH64_APPLE_DARWIN__/${macos_arm64_sha}/g" \
  -e "s/__SHA256_X86_64_APPLE_DARWIN__/${macos_x86_64_sha}/g" \
  -e "s/__SHA256_X86_64_UNKNOWN_LINUX_GNU__/${linux_x86_64_sha}/g" \
  "$template"
