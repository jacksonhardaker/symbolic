#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"
mkdir -p .tmp
build_dir="$(mktemp -d "$PWD/.tmp/build.XXXXXX")"
trap 'rm -rf "$build_dir"; rmdir .tmp' EXIT

xcrun swiftc -parse-as-library -emit-object -O -gline-tables-only \
    -module-name OptimizedClosure \
    -debug-prefix-map "$PWD=/symbolic/fixtures/optimized-closure" \
    -o "$build_dir/OptimizedClosure.o" OptimizedClosure.swift
xcrun swiftc -emit-library "$build_dir/OptimizedClosure.o" \
    -o "$build_dir/libOptimizedClosure.dylib"
xcrun dsymutil --flat "$build_dir/libOptimizedClosure.dylib" \
    -o optimized-closure.debug
