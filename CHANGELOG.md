# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Rules can have an empty or any center, so the tile goes into empty spots or into both. Right-click the center sets it to empty, left-click switches between solid and any.
- Rules can have a neighborhood larger than 3×3, with rows and columns added on any side.
- Groups can have rotated and flipped copies and their own neighborhood, like single tile rules. Parts of a group can be empty.
- Groups can have any shape. Shift+drag adds tiles to the selected group, Ctrl+drag removes them.
- Append the rulesets of another automapper file to the current one (ruleset ⏷ menu).
- Custom preview map: draw your own shape to test your rules with.
- Undo and redo with Ctrl+Z and Ctrl+Y (or Ctrl+Shift+Z).
- The app warns you about unsaved changes before quitting. Not in the web version :(
- Hovering a tile in the tileset view shows its index and all its rules.
- Press Esc or click the selected tile again to deselect it and see an overview in the sidebar.
- Reworked welcome screen and Help window, with step by step instructions.
- App icon for the welcome screen, the desktop window and the web favicon.
- Various small UI/UX improvements

### Changed

- Saving (Ctrl+S) and opening now use automapper files with an embedded blueprint.
- If the neighbors look the same after rotating or flipping, the rotated and flipped tiles are now mixed in randomly and share the rule's chance.
- New defaults (Split view, Minimal preview map, first ruleset called "default").
- New blueprint format. Older blueprints no longer load. But blueprints saved from this version on will keep working in future versions.
- Warnings now pop up as notifications in the top right, with shorter texts.
- Various small UI/UX improvements (export window, popups, menu bar, preview view, rule menu for duplicate, copy and paste, clearer and more consistent texts).

### Removed

- The bundle .zip export. Saved automappers now include an embedded blueprint.
- Small cleanups (tile index above the rules, "Always show orientation buttons" toggle).

### Fixed

- Clicking a tile in the preview now opens the correct rule, even if a tile has multiple rules.
- In the browser, links open in a new tab.

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
