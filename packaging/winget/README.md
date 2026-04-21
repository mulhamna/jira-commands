# Winget packaging source

This directory stores the source manifests for publishing `jirac` to the Windows Package Manager ecosystem.

## Files

- `jirac.yaml` — version manifest
- `jirac.locale.en-US.yaml` — locale metadata
- `jirac.installer.yaml` — installer metadata for the Windows release archive

## Update flow

1. Publish a GitHub release with the Windows archive asset.
2. Compute the archive SHA256 from the released `jirac-windows-x86_64.zip` file.
3. Update `PackageVersion`, `InstallerUrl`, `InstallerSha256`, and `ReleaseDate`.
4. Validate the manifest set with `winget validate` or the Windows Package Manager validation tooling.
5. Submit the updated manifests to the Windows Package Manager community repository.

## Contributor note

These files are source manifests kept in this repository for repeatable release operations. GitHub Actions can derive the released Windows SHA256, but submission to the community repository is still a manual follow-up.

## Notes

- The supported Windows binary is `jirac.exe`.
- Release artifacts no longer ship the legacy `jira.exe` binary.
- Keep this directory as the source of truth for future Winget automation.
