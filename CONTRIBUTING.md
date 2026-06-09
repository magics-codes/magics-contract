# Contributing

Thanks for reading — this project values restraint, so let's keep the bar
high.

## Before you touch a program

- Run `anchor test` clean on `main` first. (Needs a local validator; on Windows
  that means Developer Mode, since the validator creates symlinks.)
- If your change touches the public surface — instruction signatures, account
  layouts, events, or error codes — it is a **major** bump. Say so in the PR
  title. The IDL is part of the wire format the SDK decodes against.
- If your change touches an account's layout, write the migration plan in the
  PR description. Account drift is the one mistake we don't get to undo — a
  redeployed program still reads the old bytes.

## Style

- Rust, Anchor 0.31. Custom errors via `magics_common::MagicsError` over ad-hoc
  strings, so the SDK decodes one stable set of codes.
- `cargo fmt` is canonical. CI runs it.
- Comment the *why*, not the *what*. The code already says what.
- Keep program-to-program dependencies behind `features = ["cpi"]` — never add
  `default-features = false` on a sibling program, or two entrypoints fight over
  the global allocator.

## Tests

- A new code path without a test is not a code path.
- Revert tests should assert the **specific** error, not just that something
  threw. A bare "it failed" hides regressions.
- The full cast lifecycle — verify, budget, strategy, close — lives in
  `tests/passive-yield.ts`. Anything that changes the seam should run there.

## Filing an issue

If you think you've found a vulnerability, **do not** open a public issue.
Email `security@magics.codes` with a PoC and we'll triage within 48 hours.
