# Contributing to PixelCraft

Thanks for your interest in improving PixelCraft. Contributions should be
focused, clearly described, and consistent with the existing Rust and egui
architecture.

## Before making a change

- For larger changes, open an issue or discussion first to agree on the
  intended behavior.
- Keep changes scoped; avoid mixing unrelated cleanup with a feature or fix.
- Preserve the separation between document operations in `src/app/` and UI
  state and rendering in `src/ui/`.
- Route application actions through `Command` and `CommandEngine` where
  appropriate so the GUI and headless workflows remain consistent.
- Add or update tests for behavior changes.

## Local checks

Before submitting a change, run:

```sh
cargo fmt --check
cargo test
cargo check
git diff --check
```

If a change affects only a small area, a relevant targeted test can be run
while iterating, but please run the full test suite before submitting.

## Submitting a contribution

Provide a pull request with:

- A short summary of the problem and solution
- Relevant issue references, if any
- Tests and checks performed
- Screenshots or a brief recording for visible UI changes, when useful

Be respectful and constructive in issues, reviews, and discussions. By
submitting a contribution, you confirm that you have the right to submit it;
this repository does not currently publish a contributor license agreement or
a project license. See [LICENSING.md](./LICENSING.md).
