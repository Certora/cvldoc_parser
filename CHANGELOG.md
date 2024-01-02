# Changelog
## [2.0.0] - 2023-09-13
### Added
- Support parsing of `hook`, `use`, `using`, `import` statements.
- Python module API has been simplified.
- Support `persistent ghost`.
### Changed
- CVL2 syntax is now enforced. Parsing of code which is not compliant with CVL2 has been removed.
### Fixed
- Some issues with `Invariant` parsing have been fixed.

## [1.0.3] - 2023-05-27
### Fixed
- Fix issue that could cause infinite recursion on certain input
### Changed
- Upgrade dependencies 
- Merged Python wrapper and core changelogs to a single file

## [1.0.1] - 2023-01-09 
### Fixed
- Use span-based capturing for `Definition`

## [1.0.0] - 2022-12-20
- Release Candidate for first public version

## [0.6.4] - 2022-12-14
### Fixed
- Correctly report span for raw code of starred documentation blocks
- Incorrect detection of code blocks
- Handle variable-length characters
- Python wrapper: Comptability with parse API change

## [0.6.0] - 2022-12-07
### Changed
- Parsing engine re-written
### Changed
- Data structure of AST changed, first class member is now `CvlElement`
- Can now handle elements without documentation
- Python wrapper: Breaking changes to data structure API
### Fixed
- Fix detection of invariants and other elements
- Improve parser recovery and robustness
### Removed
- `serde` support removed for now

## [0.5.0] - 2022-11-16
### Added
- Python wrapper: Expose wrapper classes to Python, so their types can be named 
### Fixed
- Handle lines of the form /*****/ (for any amount of *)
- Ignore /*****/ (for any amount of *) when it is a separator between elements
- Fixed element span to contain both the documentation block and the associated element

## [0.4.2] - 2022-10-02
### Fixed
- Fix issue with recognition of `rule` blocks.

## [0.4.1] - 2022-09-28
### Fixed
- No longer ignore line terminator kind, so CRLF is now parsed correctly.

## [0.4.0] - 2022-09-21
### Changed
- Changed all references from "NatSpec" to the new name, "CVLDoc"
- Internal restructure of documentation struct to refacator redundant fields
### Added
- `raw` field is now captured, containing the entire text from beginning of capture

### Changed
- Python wrapper: Changed all references from "NatSpec" to the new name, "CVLDoc"
- Python wrapper: `range` is no longer printed in `repr` of classes that have them, to reduce noise
### Added

## [0.3.3] - 2022-09-15
### Changed
- Python wrapper: Now using `abi3` bindings, to (hopefully!) support all Python versions with a single build
- Python wrapper: Renamed crate (exported python module's name did not change)
- Updated dependencies

## [0.3.0] - 2022-09-13
### Added
- Multi-line freeform comments with `////` are now supported. They concatenate into a single comment.
### Changed
- Rules without parameters were not detected
- Merge `SingleLineFreeForm` and `MultiLineFreeForm` into a single enum. 
- No longer distinguish between the header and the body of a freeform comment. Now just grab everything into a single String.
- Don't special-case `#` in the grammar.
- Don't trim `#` from headers.
- Stopped removing lines that are just whitespace. 
- Wrapper: Definition of `FreeForm` was simplified to match the definition in `natspec_parser`: Replaced `header` and `block` with `text`.

## [0.2.1] - 2022-08-29
### Changed
- More informative message for documentation block with no associated element
### Fixed
- Don't require whitespace after keyword `methods` in a `methods` block declaration
- Allow any type in a `GhostMapping`, not just `mapping` types
- Do not attempt to parse NatSpecs on lines that begin with just `//`