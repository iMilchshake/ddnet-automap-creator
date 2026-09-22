# ddnet-automap-creator

Visual editor to easily create DDNet [automappers](https://wiki.ddnet.org/wiki/Automapper).

Web version is available at [https://imilchshake.github.io/ddnet-automap-creator/](https://imilchshake.github.io/ddnet-automap-creator/).

## Building

r++ is a submodule and is built from source, so clone with:

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

- rpp runs on the main thread, a hang could kill the tab
- preview which tiles a rule set actually covers
- several rule sets in one .rules file
- rules for cells whose center is empty
- .map export and DDNet install integration (native only)

## Acknowledgements 

This project is based on a PyQt6-based [project](https://github.com/AssassinTee/SimpleDDNetAutomapper) **by Assa**, doing the obligatory rust rewrite to enable good web support and integration with existing tooling such as twmap. Export of rules is based on [r++](https://github.com/Aerll/rpp) **by HiPulsar** to support more complex features such as grouped tiles.
