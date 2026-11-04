# Security

## Reporting a problem

Please report security problems privately, not in a public issue: use
GitHub's "Report a vulnerability" on the repository's Security tab (the
only private channel: the project has no e-mail address). Say what you
found, how to reproduce it and which version you used. You will get an
answer within two weeks; a fix comes with a new release and a note in
`CHANGELOG.md`.

## What Archeion protects, and what it does not

The full threat model is in [docs/threat-model.md](docs/threat-model.md).
In short:

- An **encrypted destination** keeps file names and contents secret from
  whoever reads the destination, and Archeion notices backups changed,
  swapped or put back from outside, as long as the password and the
  recovery key stay secret.
- An **unencrypted destination** is trusted: anyone who can write to it
  can change what a restore brings back (Archeion still refuses to write
  outside the place chosen for a restore).
- Nobody who can write to the destination is stopped from **deleting**
  the backups. Keep a second copy elsewhere (another drive kept apart, a
  copy away from home).
- Archeion runs as the signed-in user and never asks for administrator
  rights; it does not protect against software that already runs as
  that user.

## Releases

Each release on GitHub comes with `SHA256SUMS.txt`, a software bill of
materials (`Archeion-<version>.cdx.json`, CycloneDX: every crate inside
the executable, with its version) and build provenance attestations:
`gh attestation verify <zip> --repo MR1999develop/Archeion` checks that
the zip was built by this repository's workflow from the tagged commit.
The executable is signed with Authenticode when the release was built
with a code-signing certificate (see the README).
