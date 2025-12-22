<img src="https://github.com/user-attachments/assets/17712760-2f94-4302-9a08-8910d5fd3541" alt="ctl dash icon" width="120" />

# CTL Dash

A systemd service manager for the COSMIC desktop environment that displays system and user services with their status. Allows you to enable/disable, start/stop/restart services and view basic details and logs.

## Screenshots

| Services List | Service Details |
|-|-|
| <img width="1333" height="783" alt="Screenshot_2025-12-14_00-15-05" src="https://github.com/user-attachments/assets/7c70f68b-9cf0-42e8-99fb-16c1ddfd9a9e" /> | <img width="1333" height="783" alt="Screenshot_2025-12-14_00-15-54" src="https://github.com/user-attachments/assets/70ded06c-9c2d-44cb-8ac8-5f586477b566" /> |

## Features

- **Service List**: Display all systemd services with their current status
- **System and User Services**: Displays the system-wide and user services
- **Service Details**: View detailed information about individual services
- **Service Control**: Start, stop, restart, enable and disable services from the UI

## Installation

A [justfile](./justfile) is included by default for the [casey/just](https://github.com/casey/just) command runner.

- `just` builds the application with the default `just build-release` recipe
- `just run` builds and runs the application
- `just install` installs the project into the system
- `just vendor` creates a vendored tarball
- `just build-vendored` compiles with vendored dependencies from that tarball
- `just check` runs clippy on the project to check for linter warnings
- `just check-json` can be used by IDEs that support LSP

## Translators

[Fluent](https://projectfluent.org/) is used for localization of the software. Fluent's translation files are found in the [i18n directory](./i18n). New translations may copy the [English (en) localization](./i18n/en) of the project, rename `en` to the desired [ISO 639-1 language code](https://en.wikipedia.org/wiki/List_of_ISO_639_language_codes), and then translations can be provided for each message. If no translation is necessary, the message may be omitted.
