# Security Policy

## Supported Versions

| Branch / Tag | Supported |
|---|---|
| `main` | Yes — receives security patches |
| Release tags (latest) | Yes — patched on a best-effort basis |
| Older release tags | No — please upgrade |

## Reporting a Vulnerability

**Please do NOT open a public GitHub issue for security vulnerabilities.**

To report a security issue, e-mail the maintainers at:

```
security@kingfrankhood.dev
```

Or use [GitHub's private vulnerability reporting](https://github.com/KingFRANKHOOD/contracts/security/advisories/new) feature.

Include as much of the following information as possible to help us triage and resolve the issue quickly:

- Affected contract(s) / crate(s) (e.g. `contracts/patient-registry`)
- Type of vulnerability (e.g. unauthorized access, integer overflow, data exposure)
- Step-by-step reproduction instructions or a proof-of-concept
- Potential impact and severity assessment
- Any suggested remediation

## Response SLAs

| Severity | Acknowledgement | Status update | Target fix |
|---|---|---|---|
| Critical | 24 hours | 48 hours | 7 days |
| High | 48 hours | 72 hours | 14 days |
| Medium | 5 business days | 7 business days | 30 days |
| Low | 10 business days | 14 business days | 90 days |

We will notify you when a fix is released and credit you in the release notes unless you prefer to remain anonymous.

## Disclosure Policy

We follow **coordinated / responsible disclosure**:

1. Reporter submits a vulnerability report privately.
2. Maintainers acknowledge and begin investigation.
3. A patch is developed and reviewed on a private branch.
4. A fix release is published on `main`.
5. A public GitHub Security Advisory is created after the patch is deployed.

We ask that reporters **not publicly disclose the vulnerability** until we have released a fix or 90 days have elapsed since the initial report, whichever comes first.

## Security Architecture

This repository contains Soroban smart contracts for a decentralised healthcare system. Key security properties:

- **Authentication**: All state-mutating operations require `require_auth()` on the relevant account.
- **Access control**: Patient-controlled permission grants; no cross-patient data leakage.
- **Input validation**: All entry points validate argument ranges and types before any state change.
- **No panics**: Contracts return typed error variants; no `expect`/`unwrap` in production paths.
- **Audit trail**: Sensitive operations emit Soroban events forming an immutable audit log.
- **Upgrade governance**: Contract upgrades require multi-sig approval (`contracts/multisig-governance`, `contracts/upgrade-governance`).

For contract-specific security notes, see the `SECURITY.md` inside each crate (e.g. [`contracts/allergy-management/SECURITY.md`](contracts/allergy-management/SECURITY.md)).

## Scope

All contracts under `contracts/` are in scope. The following are **out of scope**:

- Third-party dependencies (report upstream)
- Issues requiring physical access to a deployer's machine
- Social-engineering attacks

## Bug Bounty

There is currently no formal bug bounty programme. Significant findings may be recognised with a public acknowledgement or a discretionary reward at the maintainers' discretion.
