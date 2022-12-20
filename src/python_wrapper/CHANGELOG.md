# Changelog
## [1.0.0] - 2022-12-21
### Changed
- adding upload to production in ci script.
- 
## [0.6.4] - 2022-12-14
### Fixed
- Bugfix in core

## [0.6.2] - 2022-12-13
### Changed
- Bugfix in core

## [0.6.1] - 2022-12-13
### Changed
- Comptability with parse API change

## [0.6.0] - 2022-12-07
### Changed
- Updated to support new parsing engine
- Breaking changes to data structure API
### Removed
- `serde` support removed for now

## [0.5.0] - 2022-11-16
### Added
- Expose wrapper classes to Python, so their types can be named 


## [0.4.2] - 2022-10-02
### Fixed
- Sync with `cvldoc_parser_core` version `0.4.2`.

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