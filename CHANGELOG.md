# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- App icon for the desktop window and the web favicon.

## [0.2.0] - 2026-10-21

### Added

- Custom orientation picker: serves both as a visualization and custom selection for tile for all possible orientations combining flips and rotations. Updated blueprint format accordingly.
- Zoom control in the Options menu (native only, the web uses browser zoom).
- Keybinds overview in the Help window.

### Changed

- Improve intuitiveness and robustness of various UI elements 
- Right-click deactivates rule instead of deleting it (keeping the configuration).
- Improve neighborhood editor: left and right click now toggle the states more intuitively. Initial rule is tiles being Any, this is no longer guessed so its more predicable to work with.
- Remove darkening of unused tiles, strongly reduce for deactivated tiles.
- Tile 0 is reserved for air and can no longer get a rule.

### Removed

- Tile 255 is no longer reserved as rpp mask, highest unused tile is chosen dynamically instead

### Fixed

- Ctrl + / Ctrl - zoomed twice in the web version.
