# BUGLE: Butt-Ugly Game Launcher for Exiles

BUGLE is an unofficial, third-party game launcher for Funcom's videogame Conan Exiles. It aims to
bring certain QOL features to players, at the expense of good looks and smooth user interface.

It's ugly as sin, it's poorly tested, and it's my hobby project that I came up with for fun and
relaxation, so don't expect the stuff you would find in a more serious development project (e.g.
well-commented code, automated tests, etc.)

ALPHA VERSION WARNING: This is the 1.0.0-alpha.3 version of BUGLE. What that means is that it hasn't
been used thoroughly on any computer but mine. While I'm reasonably sure that it won't make your
computer catch fire or delete any of your files, expect it to not actually do its job properly.
It might crash, or it might mess up your modlist or your game settings. It ***shouldn't***, but it
***might***.

For a list of known issues, scroll to the last section of this document. If you run into an issue
that isn't there, feel free to open a GitHub issue about it, or contact me on Funcom Forums.

## Installation

Go to the v1.0.0-alpha.3 release and download the `bugle-v1.0.0-alpha.3-x86_64-pc-windows-msvc.zip`
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
BattlEye will be enabled only if you join a server that requires it. NOTE: For the moment, BUGLE
does not do this properly when you click on the "Continue" button on the main launcher page, and
will launch with BattlEye disabled.

## Roadmap

There's a lot more that can be added to BUGLE, and some if it is already in my plans. Bear in mind,
though, that this is my ***hobby***, so don't expect me to add stuff quickly and tirelessly.

Here are some things that I'm planning to (try to) add to BUGLE:
* **Co-op.** Right now, the co-op button in the launcher informs you that this feature is "not yet
implemented". The truth is that I've never even played Conan Exiles in co-op mode, ever, and I don't
really have anyone to try it with. I intend to implement this, but first I'll need some help from
a volunteer.
* **Support for other platforms.** Right now, I'm building BUGLE only for Windows, and it works only
with Steam. Ideally, I would like it to support Conan Exiles when installed from a different game
store, and I would also like to offer support for Linux. However, I'll need help from volunteers to
make that happen.
* **Online mod mismatch detection.** I would love to make BUGLE detect whether your mod list matches
the server you're trying to join. Unfortunately, the information about a server's mod list is part
of the protocol the game uses to let you play. There is no information on this protocol. Unlike the
one used by the server browser, this one will be a much tougher nut to crack, and I'm honestly not
sure whether I'll have the time, patience, or skill to do it.

## Known Issues

* **Does not check if you're logged into Steam.** BUGLE will happily run if you haven't started
Steam or logged into it. In fact, it will happily launch Conan Exiles and let you discover the hard
way that Steam isn't running. I'll fix that eventually.
* **BattlEye usage on "Continue".** If you've configured BUGLE to enable BattlEye "only when
required" and you press the "Continue" button on the main launcher screen, it will launch the game
with BattlEye disabled, even if you're connecting to a server that requires BattlEye. This will be
fixed in the next update.
