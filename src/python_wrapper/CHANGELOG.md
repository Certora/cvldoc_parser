# Changelog
## [0.3.3] - 2022-09-15
### Changed
- now using `abi3` bindings, to (hopefully!) support all Python versions with a single build
- Renamed crate (exported python module's name did not change)
- Updated dependencies

## [0.3.0] - 2022-09-13
### Changed
- Definition of `FreeForm` was simplified to match the definition in `natspec_parser`: Replaced `header` and `block` with `text`.