# Release security checklist

Before publishing a RustDroid release:

1. Review `cargo deny check` and CodeQL results for the release commit.
2. Confirm Dependabot alerts/updates have been triaged; never dismiss an advisory without a recorded reason.
3. Run the release package, checksum, clean-container install, and provenance-attestation workflow.
4. Verify release artifacts were built from the intended tag and contain no credentials, local paths, or debug-only files.
5. Confirm installer architecture claims match the assets actually attached to the release.
6. Review the changelog for security-sensitive changes and update `SECURITY.md` or advisories where needed.
7. Keep signing, registry, and GitHub tokens in repository secrets; never echo them in logs or paste them into issues.
8. After release, verify checksums and attestation links from a clean checkout, then record any known limitation in the release notes.
