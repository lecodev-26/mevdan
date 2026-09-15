# Governance

MEVDAN is currently a small, early-stage open-source project. This
document describes how decisions are made.

## Maintainer

**lecodev-26** (@lecodev-26 on GitHub) is the current maintainer.

Responsibilities:

- Review and merge pull requests.
- Triage issues.
- Make final decisions when consensus is not reached.
- Manage releases.

## Decision making

### Small changes

- Bug fixes, documentation, tests, small features → merged directly by
  the maintainer after review.

### Large changes

- New crates, architectural changes, breaking API changes → require an
  RFC (see below).

## RFC process

For substantial changes, we use a lightweight RFC process:

1. **Open an issue** with the `[RFC]` prefix in the title.
2. **Describe the proposal:**
   - What problem it solves
   - Alternatives considered
   - Impact on existing code and data
   - Migration path (if applicable)
3. **Discussion period:** at least 7 days.
4. **Decision:** the maintainer either accepts, requests changes, or
   rejects the RFC. Reasoning is documented.
5. **Implementation:** once accepted, the RFC becomes an issue or
   milestone.

RFCs will be stored in `docs/RFC/` once the folder is created.

## Versioning

We follow [Semantic Versioning](https://semver.org/):

- `MAJOR.MINOR.PATCH`
- `0.x.y` → pre-release. API may change at any time.
- `1.0.0` → first stable release. Backward compatibility commitment from
  this point.

## Breaking changes

- **Before 1.0.0:** breaking changes are allowed but must be documented in
  `CHANGELOG.md`.
- **After 1.0.0:** breaking changes require a major version bump, a
  migration path, and deprecation warnings for at least one minor
  version.

## Releases

- Releases are tagged on `main`.
- GitHub Releases include the relevant section from `CHANGELOG.md`.
- No fixed release schedule. Releases happen when a version's scope is
  complete.

## Security

Security issues follow a separate process. See [SECURITY.md](SECURITY.md).

## Amendments

This document may be amended by the maintainer. Substantial changes will
be announced in the repository.
