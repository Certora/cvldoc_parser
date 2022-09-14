# Changelog
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
