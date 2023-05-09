# BUGLE: Butt-Ugly Game Launcher for Exiles

BUGLE is an unofficial, third-party game launcher for Funcom's videogame Conan Exiles. It aims to
bring certain QOL features to players, at the expense of good looks and smooth user interface.

It's ugly as sin, it's poorly tested, and it's my hobby project that I came up with for fun and
relaxation, so don't expect the stuff you would find in a more serious development project (e.g.
well-commented code, automated tests, etc.)

BETA VERSION: This is a beta version of BUGLE. Several other people have been kind enough to test
it and help me find a few bugs. The software works, but it might still have some minor bugs.

For a list of known issues, scroll to the last section of this document. If you run into an issue
that isn't there, feel free to open a GitHub issue about it, or contact me on Funcom Forums.

## Installation

Go to the v1.0.0-beta.1 release and download the `bugle-v1.0.0-beta.1-x86_64-pc-windows-msvc.zip`
file. Unpack it into a directory where you're allowed to write files. It's a good idea to put it in
its own directory, because it will write a couple of files there (`bugle.ini` and `bugle.log`).

There's no installer, you just run `bugle.exe`.

## Features

In no particular order, the following are the features BUGLE brings to players who decide to try it:

* **Efficiency.** It's quicker to start and takes up less memory than Funcom launcher.
* **Server browser.** Instead of starting the game, selecting online play, waiting for the server
list to load up, and selecting the server you want to play, you can do it straight from the
launcher itself.
* **Persistent server filter.** You don't have to reconfigure the filter every time you open the
server browser. If you filtered the server list to look at only PVE-C server in Oceania region, the
next time you open BUGLE, the filter will be in effect. The only filters BUGLE does not persist are
server name and map name.
* **Ping an individual server.** If your ping seems to be too high or you want to see whether the
number of connected players changed, you can ping the selected server again and get updated results,
without having to restart the whole server browser.
* **Single-player game list.** For each map you have installed, you can see when was the last time
you played it in single-player, the name of your character and clan, and what level your character
is.
* **Back up and restore single-player games.** You can create backups for your single-player games,
and restore them whenever you want.
* **Mod list management.** You can not only activate and deactivate mods, and change their order,
but you can also save a mod list and open it again later. This should make it easier to switch
between the mods you use in your single-player game and those on your favorite server; or you can
use this to switch easily between mod lists for different servers.
* **Single-player mod mismatch detection.** The launcher will do its best to detect whether there
are any mods missing in your list when you decide to continue your single-player game. It cannot
detect if mods are in the correct order, but at least it can warn you about missing mods. It also
warns you when you have mods in your list that you haven't previously used with your single-player
game.
* **Flexible BattlEye usage.** Just like with Funcom launcher, you can configure BUGLE to enable or
disable BattlEye. However, you can also tell BUGLE to use it "only when required". In this mode,
BattlEye will be enabled only if you join a server that requires it.

## Roadmap

There's a lot more that can be added to BUGLE, and some if it is already in my plans. Bear in mind,
though, that this is my ***hobby***, so don't expect me to add stuff quickly and tirelessly.

Here are some things that I'm planning to (try to) add to BUGLE:
* **Co-op.** Right now, the co-op button in the launcher informs you that this feature is "not yet
implemented". The truth is that I've never even played Conan Exiles in co-op mode, ever, and I don't
really have anyone to try it with. I intend to implement this, but first I'll need some help from
a volunteer.
* **Localization.** BUGLE is currently available only in English. I need to add support for other
languages.
* **Tooltips.** It's not always obvious what some of the user interface elements mean. I need to add
tooltips that explain them.
* **Support for other platforms.** Right now, I'm building BUGLE only for Windows, and it works only
with Steam. Ideally, I would like it to support Conan Exiles when installed from a different game
store, and I would also like to offer support for Linux. However, I'll need help from volunteers to
make that happen.
* **Online mod mismatch detection.** I would love to make BUGLE detect whether your mod list matches
the server you're trying to join. Unfortunately, the information about a server's mod list is part
of the protocol the game uses to let you play. There is no information on this protocol. Unlike the
one used by the server browser, this one will be a much tougher nut to crack, and I'm honestly not
sure whether I'll have the time, patience, or skill to do it.

For a more detailed view of what ideas I'm investigating and what features I'm planning to add, you
can visit the [BUGLE Roadmap Trello board](https://trello.com/b/zjDYQsq8/roadmap).

## Known Issues

* **Mod mismatch warning if you stop using a mod.** If you use a mod in your single-player or co-op
game and then decide to stop using it, it will leave traces in your game database. BUGLE will detect
those traces, see that the mod isn't in your mod list, and warn you about mod mismatch.
