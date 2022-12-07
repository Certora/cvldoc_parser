# Changelog
## [0.6.0] - 2022-12-07
### Changed
- Parsing engine re-written
### Changed
- Data structure of AST changed, first class member is now `CvlElement`
- Can now handle elements without documentation
### Fixed
- Fix detection of invariants and other elements
- Improve parser recovery and robustness
### Removed
- `serde` support removed for now


## [0.5.0] - 2022-11-16
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

## [0.3.2] - 2022-09-15
### Changed
- Updated dependencies

## [0.3.0] - 2022-09-13
### Added
- Multi-line freeform comments with `////` are now supported. They concatenate into a single comment.
### Changed
- Merge `SingleLineFreeForm` and `MultiLineFreeForm` into a single enum. 
- No longer distinguish between the header and the body of a freeform comment. Now just grab everything into a single String.
- Don't special-case `#` in the grammar.
- Don't trim `#` from headers.
- Stopped removing lines that are just whitespace. 

## [0.2.2] - 2022-09-13
### Fixed
- Rules without parameters were not detected

## [0.2.1] - 2022-08-29
### Changed
- More informative message for documentation block with no associated element
### Fixed
- Don't require whitespace after keyword `methods` in a `methods` block declaration
- Allow any type in a `GhostMapping`, not just `mapping` types
- Do not attempt to parse NatSpecs on lines that begin with just `//`
