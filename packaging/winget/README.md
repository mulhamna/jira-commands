# Winget packaging source

This directory stores the source manifests for publishing `jirac` to the Windows Package Manager ecosystem.

## Files

- `jirac.yaml` — version manifest
- `jirac.locale.en-US.yaml` — locale metadata
- `jirac.installer.yaml` — installer metadata for the Windows release archive

## Update flow

1. Publish a GitHub release with the Windows archive asset.
2. Release CI refreshes the in-repo manifests under `packaging/winget/` using the published release version, URL, SHA256, and release date.
3. Validate the manifest set with `winget validate` or the Windows Package Manager validation tooling.
4. Submit the updated manifests to the Windows Package Manager community repository.

## Contributor note

These files are source manifests kept in this repository for repeatable release operations. GitHub Actions now refreshes them after each published release, but submission to the community repository is still a manual follow-up.

## Notes

- The supported Windows binary is `jirac.exe`.
- Release artifacts no longer ship the legacy `jira.exe` binary.
- Keep this directory as the source of truth for future Winget automation.
