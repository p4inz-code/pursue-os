# Security Policy

PURSUE OS is security-sensitive software intended for investigation and research workflows.

Security issues should be reported responsibly.

## Supported Versions

PURSUE does not currently have a stable public release.

During early development, security reports concerning the current development branch are welcome.

Once stable releases exist, this section will define the supported versions.

## Reporting a Vulnerability

Please do **not** open a public GitHub issue for an undisclosed security vulnerability.

Use GitHub's private vulnerability reporting mechanism when available for the repository.

If private reporting is unavailable, contact the project maintainer privately before public disclosure.

Include:

- affected component
- affected commit or version
- reproduction steps
- expected behavior
- actual behavior
- security impact
- proof of concept where safe

## Do Not Include

Do not include:

- passwords
- API keys
- private keys
- authentication tokens
- private investigation data
- personal information
- unrelated sensitive information

## Responsible Disclosure

Please allow reasonable time for investigation and remediation before publicly disclosing an undisclosed vulnerability.

Security researchers will be credited where appropriate and where they wish to be identified.

## Scope

Security reports may concern:

- privilege boundaries
- authentication
- cryptography
- evidence integrity
- case storage
- provenance
- network isolation
- Tor integration
- sandboxing
- plugin security
- AI security boundaries
- malicious package or update behavior
- data leakage
- command execution
- OS-level vulnerabilities introduced by PURSUE

## Security Philosophy

PURSUE treats security as a foundational property of the operating system rather than an optional feature.

Security must not depend on AI behavior.

Evidence must remain traceable to its source and investigator actions.
