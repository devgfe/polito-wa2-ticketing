# Working agreement

## Branches
- `main` is the only long-lived branch and is protected: no direct pushes, every change goes through a PR.
- Branch names: `<type>/<short-description>`, same types as Conventional Commits. Lowercase, kebab-case, no personal names.
- Short-lived: one branch, one goal, deleted after merge.

Examples:

    feat/catalogue-list-events
    fix/inventory-hold-expiry
    docs/m1-scope-and-services

## Commits
We follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).

    <type>(<scope>): <description>

- Types: `feat`, `fix`, `docs`, `test`, `refactor`, `build`, `ci`, `chore`
- Scope: the service or folder, e.g. `catalogue`, `client`, `docs`
- Imperative, lowercase, no final period

Examples:

    feat(catalogue): add list events endpoint
    fix(inventory-orders): release hold on expiry
    docs(m2): add data ownership table

## Pull requests
- Squash merge only: the PR title becomes the commit message on `main`, so it must follow Conventional Commits.
- At least one approval from a teammate (docs included).
- CI must pass before merging.
- Keep PRs small and focused.
- Delete the branch after merge.

## Documentation
- Docs live in the repo as Markdown and are reviewed like code.
- Important decisions are recorded in `docs/adr/` (`NNNN-short-title.md`).