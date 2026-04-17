# Security Policy

## Supported Versions

Only the latest published version of `jira-commands` and `jira-core` receives security fixes.

| Version | Supported |
|---------|-----------|
| latest  | ✓         |
| older   | ✗         |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Please report security issues privately via one of these channels:

1. **GitHub private vulnerability reporting** (preferred):
   Go to [Security → Report a vulnerability](../../security/advisories/new) on this repo.

2. **Email**: Contact the maintainer directly. Find the email in the
   [git log](../../commits/main) or the crates.io package metadata.

### What to include

- Description of the vulnerability and potential impact
- Steps to reproduce (minimal proof of concept if possible)
- Affected versions
- Any suggested fix (optional but appreciated)

### What to expect

- **Acknowledgement**: within 3 business days
- **Initial assessment**: within 7 business days
- **Fix timeline**: depends on severity — critical issues are prioritized immediately

### Scope

This project is a CLI tool and library that communicates with **your own** Jira instance
using credentials you provide. The attack surface is:

- Credential storage (`~/.config/jira/config.toml`) — file permissions are set to 600 on Unix
- HTTP communication with Jira API — uses TLS via `rustls` (no OpenSSL dependency)
- Input parsing (JQL, manifest files, CLI flags)
- Dependencies — audited automatically in CI via `cargo audit` against the
  [RustSec Advisory Database](https://rustsec.org/)

Out of scope: vulnerabilities in your Jira instance itself, or issues requiring
physical access to the machine running the CLI.

## Dependency Auditing

All dependencies are checked against the RustSec Advisory Database on every CI run.
You can run the same check locally:

```bash
cargo install cargo-audit --locked
cargo audit
```
