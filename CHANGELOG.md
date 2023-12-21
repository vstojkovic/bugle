# Change Log

All notable changes to this project will be documented in this file.

## 1.2.1 (2023-12-21)

Bugfix for problems with "Tot!" mods.

### Fixed

- BUGLE will no longer complain about a "problem with your installation of Conan Exiles" when you
have certain mods installed.

## 1.2.0 (2023-10-22)

More configuration options and QoL features.

### Added

- BUGLE will now inform you if the game needs to be updated.
- Outdated mods can now be updated in the mod manager.
- Mods can now be activated and deactivated by double-clicking in the mod manager.
- Added an option to tell the game to try using all available CPU cores.
- You can now specify additional command line arguments to be used when launching the game.
- Added an option to disable mod mismatch checks.

### Changed

- Server name and map name in the server browser filter are now persisted.
- Read-only text fields are now slightly shaded to avoid confusion.

### Fixed

- When clicking the "Launch" button, the launcher will check if any mods need to be updated.
- If BUGLE cannot write its .ini file in the same folder as the executable, it will try to create
  one in the appropriate user profile directory.

## 1.1.0 (2023-07-25)

Miscellaneous QoL features.

### Added

- Display total number of connected players, in the server browser.
- Show the names of up to 10 Steam mods, in the server details.
- Show which mods need to be updated, in the mod manager.
- Offer to update outdated mods from your mod list when starting a game.

### Changed

- Moved the last session information above the "Continue" button.
- Some server details are not displayed if absent.
- Server name column is now left-justified in the server browser.

### Fixed

- If the last session was online, it will also be hidden by the "Hide Private Information" button.

## 1.0.1 (2023-06-03)

Hotfix for connecting to password-protected servers.

### Added

- Ask for password when connecting to a password-protected server or via direct connect.

### Changed

- Made the BattlEye usage configuration on the home tab easier to understand.
