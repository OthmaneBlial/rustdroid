# Versioning Policy

RustDroid uses semantic versioning.

## Major

Cut a new major version when the CLI, config behavior, or documented workflow contract changes in a breaking way.

Examples:

- removing a command
- renaming a stable flag
- changing default behavior in a way that breaks scripts or team configs

## Minor

Cut a minor version when RustDroid adds meaningful new capability without breaking the stable contract.

Examples:

- new commands
- new supported APK input shapes
- new artifact outputs
- new install paths

## Patch

Cut a patch release for bug fixes, CI fixes, recovery fixes, and documentation corrections that do not break users.

## Rule

If the safest user upgrade path is “drop in the new binary and keep working,” it is probably a patch or minor release, not a major one.
