# Contributing

Thanks for reading — this project values restraint, so let's keep the bar
high.

## Before you touch a contract

- Run `forge test -vv` clean on `main` first.
- If your change touches the public ABI (events, function signatures, custom
  errors), it is a **major** bump. Say so in the PR title.
- If your change touches storage layout, write the migration plan in the PR
  description. Storage drift is the one mistake we don't get to undo.

## Style

- Solidity 0.8.26, Cancun. Custom errors over `require` strings.
- `forge fmt` is canonical. CI runs it.
- Comment the *why*, not the *what*. The code already says what.
- Function order in a contract: storage → constructor → external mutating →
  external view → internal. Matches `solhint`'s `func-order` rule but with
  one wrinkle: constructor comes first, before any other functions.

## Tests

- A new code path without a test is not a code path.
- Fuzz any function that takes a `uint256` you don't bound — Foundry's default
  fuzz runs are cheap, use them.
- Revert tests should assert the **exact** custom error and its arguments.
  `vm.expectRevert(abi.encodeWithSelector(Errors.Foo.selector, arg))`, not
  `vm.expectRevert()`. The latter hides regressions.

## Filing an issue

If you think you've found a vulnerability, **do not** open a public issue.
Email `security@magics.xyz` with a PoC and we'll triage within 48 hours.
