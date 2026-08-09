# Contributing to PURSUE OS

Thank you for your interest in PURSUE OS.

PURSUE is an open-source investigation-focused operating system currently in active development.

## Before Contributing

Please understand the project's current status:

- Planning is complete.
- The repository foundation is established.
- Core implementation is beginning.
- There is no stable public release yet.

Read the relevant documentation before making architectural changes.

## Contributions

Useful contributions may include:

- bug reports
- security research
- documentation improvements
- testing
- investigation workflow feedback
- accessibility feedback
- UI/UX improvements
- code improvements
- tool integration
- plugin development
- performance improvements

## Pull Requests

Before opening a pull request:

1. Keep the change focused.
2. Explain what changed and why.
3. Include relevant tests where applicable.
4. Do not introduce unrelated refactors.
5. Do not silently change locked product or security decisions.
6. Do not add dependencies without justification.
7. Do not commit secrets, credentials, private data, or investigation evidence.

Large architectural changes should be discussed before implementation.

## Security-Sensitive Changes

Changes involving:

- evidence handling
- cryptography
- authentication
- privileges
- networking
- Tor
- sandboxing
- isolation
- case storage
- provenance
- AI boundaries

require additional review.

Do not disclose security vulnerabilities through public issues.

See [`SECURITY.md`](SECURITY.md).

## Code Quality

PURSUE prioritizes:

- correctness
- security
- reliability
- maintainability
- clear architecture
- testability
- user control

Complexity should have a concrete reason to exist.

## License

By contributing, you agree that your contribution may be distributed under the project's Apache License 2.0 unless otherwise stated for the specific contribution.

Copyright remains with the respective contributors for their contributions.
