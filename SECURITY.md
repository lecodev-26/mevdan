# Security Policy

## Reporting a vulnerability

**Do not open a public issue for security vulnerabilities.**

Instead, please report privately:

1. Go to https://github.com/lecodev-26/mevdan/security/advisories
2. Click "Report a vulnerability"
3. Describe the issue with as much detail as possible:
   - What you found
   - How to reproduce it
   - Potential impact
   - Suggested fix (if you have one)

You will receive a response within **7 days**.

If the vulnerability is confirmed, we will:

1. Work with you on a fix.
2. Credit you in the release notes (unless you prefer to stay anonymous).
3. Publish a security advisory once the fix is released.

## Scope

In scope:

- `mevdan-core` — domain logic, ID generation, serialization
- `mevdan-storage` — SQLite persistence, migrations, event log
- `mevdan-cli` — command-line interface

Out of scope (for now):

- Providers (not implemented yet)
- Tools (not implemented yet)
- Agents (not implemented yet)

## Security principles

MEVDAN takes security seriously. Core principles:

1. **Local-first.** No data leaves your machine unless you explicitly
   configure a provider.
2. **No telemetry by default.** MEVDAN never phones home.
3. **Permission-gated actions.** Dangerous operations (shell, write,
   network, git) require explicit permission.
4. **Secrets never in Git.** API keys are stored in the platform's
   secure storage, not in project files.
5. **Append-only event log.** Actions cannot be hidden or deleted after
   the fact.
6. **Verification over claims.** Model output is not trusted by default.

## What we do NOT do

- We do not collect any user data.
- We do not send analytics.
- We do not require an account.
- We do not store API keys in plain text files.
- We do not execute arbitrary commands without permission.

## Supported versions

Only the latest version is supported for security fixes.

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅ Yes    |
| < 0.1   | ❌ No     |

## Acknowledgements

We thank the security researchers who help make MEVDAN safer.
Contributors who responsibly disclose vulnerabilities will be credited
in release notes unless they prefer otherwise.
