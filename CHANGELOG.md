# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Custom preview map: draw your own shape to test the rules on.
- Rules can place tiles into empty cells: right-click the center tile.

## [0.3.0] - 2026-10-24

### Added

- Split view: tileset and preview side by side.
- Click a tile in the preview to select its rule.
- Rules can now require a specific neighbor tile (add via CTRL+Left Click on a neighbor tile).
- Support for multiple individual rules per tile
- Copy and paste rules (via UI or or C/V keys).
- Hold Space to peek at the other view.

### Changed

- Rotate and flip buttons below the grid turn the neighbors. Buttons on the center tile change only the center tile.
- Re-organized and improved some UI elements
- New blueprint format. Older blueprints no longer load.

### Fixed

- Chances adding up to 100 % no longer show a wrong share.

## [0.2.0] - 2026-10-21

### Added

- Custom orientations: pick any combination of rotations and flips for a tile.
- Zoom control in the Options menu (native only, the web uses browser zoom).
- Keybinds overview in the Help window.

### Changed

- Improve intuitiveness and robustness of various UI elements 
- Right-click deactivates rule instead of deleting it (keeping the configuration).
- Clicks in the neighborhood editor toggle states more intuitively. New rules start with all neighbors set to Any.
- Remove darkening of unused tiles, strongly reduce for deactivated tiles.
- Tile 0 is reserved for air and can no longer get a rule.

### Removed

- Tile 255 is no longer reserved as rpp mask, highest unused tile is chosen dynamically instead

### Fixed

- Ctrl + / Ctrl - zoomed twice in the web version.
