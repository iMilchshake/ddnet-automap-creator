<p align="center">
  <img src="assets/ddnet-automap-creator.ico" height="96" alt="">
</p>

<h1 align="center">ddnet-automap-creator</h1>

Visual editor to easily create DDNet automappers. 
The goal of this tool is to help _normal_ users (without coding skills) to create complete automappers for _most_ tilesets.
However, some special edge-cases or more complex rules might not be supported yet.
In future versions i might support more rules, but hide them in some "advanced" mode, so the tool remains easy to use for regular users.

Interested? You dont need to install anything, just check out the [web tool](https://imilchshake.github.io/ddnet-automap-creator/).

## Building

Want to build it yourself? r++ is a submodule and is built from source, so clone this repo with:

```sh
git clone --recurse-submodules https://github.com/iMilchshake/ddnet-automap-creator
cd ddnet-automap-creator
```

### Desktop / Native

Required: `cmake` with C++ compiler.

Running `cargo run` will automatically compile r++ using `cmake`.

### Web / WASM

Required: [trunk](https://trunkrs.dev) and `emscripten`.

Running `trunk serve` will automatically compile r++ to wasm using `emscripten`.

## Potential future features 

### Application
- autosave the project (in browser storage)
- .map export (via ddnet-rs / twmap ?)
- (live) .r source preview -> could be used as an interactive tool to learn r++?
- DDNet install integration (native only) -> we could use twstorage

### Ruleset features
_Some of these offer little value but require very complex ui additions, so idk which are worth to add. If you think some of them are particularly important, feel free to reach out and let me know._

- support modulo operations
- allow rules to require a neighbor to not be a specific tile
- let a neighbor match one of several specific tiles
- match specific tile neighbors in one orientation only
- option to skip a rule at the map edge (as DDNet repeats it, which can look odd)
- rules that replace specific tiles instead of solid ones
- option to never place a tile/group next to itself or within some radius of specific tiles/groups
- user-defined run order and repeated runs

## Acknowledgements 

This project is a Rust rewrite and extension of a PyQt6 [project](https://github.com/AssassinTee/SimpleDDNetAutomapper) **by Assa**, to enable good web support and integration with existing tooling such as twmap. Export of rules is based on [r++](https://github.com/Aerll/rpp) **by HiPulsar** to support more complex features such as grouped tiles.

The preview map `dm1` is from [teeworlds-maps](https://github.com/teeworlds/teeworlds-maps), under [CC-BY-SA 3.0](http://creativecommons.org/licenses/by-sa/3.0/).

For more information about DDNet automappers check out the [wiki](https://wiki.ddnet.org/wiki/Automapper).

