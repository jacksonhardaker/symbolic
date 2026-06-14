# Optimized Swift closure

`OptimizedClosure.swift` returns a capturing closure and invokes it from an exported function.
`optimized-closure.debug` is a flat ARM64 Mach-O dSYM built with Apple Swift 6.4, `-O`, and
`-gline-tables-only`. It contains real optimized closure symbols, inline records, and DWARF v5
line tables. Build it on macOS with Xcode using `bash build.sh`. Tests parse the checked-in
fixture on every platform and do not require Swift or Xcode.

The debug prefix map fixes the compilation directory at `/symbolic/fixtures/optimized-closure`.
The closure's arithmetic is on lines 3 and 4. Swift emits function records for this closure;
the symcache regression checks ordinary conversion, then hides attached rows or function records
through a test adapter while preserving real object-session range queries. Recovery-disabled
checks establish that each simulated missing-debug-info shape actually requires recovery.
