# Changelog
## [0.4.1] - 2022-09-28
### Fixed
- No longer ignore line terminator kind, so CRLF is now parsed correctly.

## [0.4.0] - 2022-09-21
### Changed
- Changed all references from "NatSpec" to the new name, "CVLDoc"
- `range` is no longer printed in `repr` of classes that have them, to reduce noise
### Added
- `raw` field is now captured, containing the entire text from beginning of capture

## [0.3.3] - 2022-09-15
### Changed
- Now using `abi3` bindings, to (hopefully!) support all Python versions with a single build
- Renamed crate (exported python module's name did not change)
- Updated dependencies

## [0.3.0] - 2022-09-13
### Changed
- Definition of `FreeForm` was simplified to match the definition in `natspec_parser`: Replaced `header` and `block` with `text`.