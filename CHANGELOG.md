# Change Log

All notable changes to this project will be documented in this file.

## 1.4.0 (2024-05-14)

Settings editor, saved servers, more stats and filters.

### Added

- You can now edit the settings for your single-player game.
- Single-player backup can now be exported to or imported from a file anywhere on the filesystem.
- BUGLE now lets you save the password when joining a password-protected server. It can also use the
password you've previously saved inside the game.
- Server browser now shows how many servers there are in total, how many servers fit your current
filter, how many players are connected to all servers, and how many players are connected to the
servers that match the current filter.
- More criteria can now be configured in the server browser filter.
- You can now save server information from the server browser to your local BUGLE installation. This
allows you to connect to a server after it reboots even if Funcom's server list doesn't show it yet.
- You can also manually add servers to the saved servers list, so you don't have to use the "Direct
Connect" option when you want to connect to an unlisted server.

### Changed

- You now have to pick a difficulty when starting a new single-player game. You can either choose
one of the three preset difficulties (civilized, decadent, or barbaric) or configure your own custom
settings.

### Fixed

- If the `Mods` directory is missing, BUGLE will try to create it.

## 1.3.2 (2024-03-07)

Hotfix for a mod library bug.

### Fixed

- If the `Mods` directory is missing, BUGLE will no longer treat that as a problem with the game
installation.

## 1.3.1 (2024-02-24)

Hotfix for a GUI bug.

### Fixed

- Drop down list widgets now render glyphs properly.

## 1.3.0 (2024-01-30)

Mod manager QoL features and a few visual improvements.

### Added

- You can now place mod files anywhere in the `Mods` directory, nested as deep as you want, and
BUGLE will detect them and display them in the list of available mods so you can add them to your
mod list.
- The list of mods in the mod manager now shows the provenance of each mod: Steam or local library.
- The mod manager now has a table that displays the details of the selected mod, such as filename,
size, Steam IDs (if applicable), and similar.
- If you want to host a server using the Dedicated Server Launcher, you can now copy your mod list
to clipboard to paste it into the server launcher.
- If you have a mod list with invalid entries, BUGLE can try to fix it by going through the list
and trying to match the entries with the mods you have installed on your system. This can be useful
if you moved your Conan Exiles installation to a different location, or if you got your mod list
from a friend who has their game installed on a path different from yours.
- BUGLE now detects whether you have BattlEye installed and displays that information on its home
screen. If BattlEye is not installed and your BUGLE is configured to launch the game with BattlEye
enabled, BUGLE will warn you and offer to change that setting to disabled.

### Changed

- The main menu buttons on the left have been shrunk to leave more space for the rest of the UI.
- BUGLE window can now be resized and maximized.
- Mod manager now has separate buttons for displaying the mod description and the mod change notes.
- The BUGLE logo on the home screen is now a smidge more colorful.
- The glyphs used by BUGLE are now all sourced or derived from Bootstrap Icons project.

### Fixed

- Having a mod file that BUGLE cannot parse will no longer make BUGLE complain about a "problem
with your installation of Conan Exiles". Affected mod files will be displayed among available mods,
but with an error icon and no details other than filename and size.
- Dragging BUGLE between displays with different scaling will no longer mess up the UI.
- The README no longer says that BUGLE won't save the server name or map name in the persistent
server filter.
- BUGLE should no longer mix backslashes and forward slashes in paths.

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
