# CTLDash

A systemd service manager for the COSMIC desktop environment that displays system and user services with their status. Allows you to enable/disable, start/stop/restart services and view basic details and logs.

## Features

- **Service List**: Display all systemd services with their current status
- **System and User Services**: Displays the system-wide and user services
- **Service Details**: View detailed information about individual services
- **Service Control**: Start, stop, restart, enable and disable services from the UI

## Installation

A [justfile](./justfile) is included by default for the [casey/just][just] command runner.

- `just` builds the application with the default `just build-release` recipe
- `just run` builds and runs the application
- `just install` installs the project into the system
- `just vendor` creates a vendored tarball
- `just build-vendored` compiles with vendored dependencies from that tarball
- `just check` runs clippy on the project to check for linter warnings
- `just check-json` can be used by IDEs that support LSP

## Translators

[Fluent][fluent] is used for localization of the software. Fluent's translation files are found in the [i18n directory](./i18n). New translations may copy the [English (en) localization](./i18n/en) of the project, rename `en` to the desired [ISO 639-1 language code][iso-codes], and then translations can be provided for each [message identifier][fluent-guide]. If no translation is necessary, the message may be omitted.
