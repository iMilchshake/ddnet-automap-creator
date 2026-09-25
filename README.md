# ddnet-automap-creator

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

## TODO

- .map export (via ddnet-rs / twmap ?)
- undo and redo for tile and group edits
- (doodad rules) on empty cells, re-runnable via transparent marker tile
- autosave the project (in browser storage)
- several rule sets in one .rules file
- (live) .r source preview -> could be used as an interactive tool to learn r++?
- DDNet install integration (native only)
- drag-n-drop tilesets to load it
- keyboard navigation / shortcuts?
- rpp runs on the main thread, a hang could kill the tab
- use an empty tileset tile as mask, not always 255
- do we want to support modulo rules? how are they used?

## Acknowledgements 

This project is a Rust rewrite and extension of a PyQt6 [project](https://github.com/AssassinTee/SimpleDDNetAutomapper) **by Assa**, to enable good web support and integration with existing tooling such as twmap. Export of rules is based on [r++](https://github.com/Aerll/rpp) **by HiPulsar** to support more complex features such as grouped tiles.

The preview map `dm1` is from [teeworlds-maps](https://github.com/teeworlds/teeworlds-maps), under [CC-BY-SA 3.0](http://creativecommons.org/licenses/by-sa/3.0/).

For more information about DDNet automappers check out the [wiki](https://wiki.ddnet.org/wiki/Automapper).

