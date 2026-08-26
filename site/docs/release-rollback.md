# RustDroid Release Rollback

Use this when a tagged release is broken or the release assets are wrong.

## Procedure

1. Mark the GitHub release as a bad release in the release notes or delete it if the assets are unusable.
2. Remove the published tag from the remote: `git push origin :refs/tags/<tag>`.
3. Create a follow-up fix on `main`, verify CI, and rebuild the release assets locally.
4. Re-tag from the corrected commit and push the tag again.
5. Post a short follow-up note explaining the rollback and replacement tag.
