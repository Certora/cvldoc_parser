# Changelog

## [0.2.1] - 2022-08-29
### Changed
- More informative message for documentation block with no associated element
### Fixed
- Don't require whitespace after keyword `methods` in a `methods` block declaration
- Allow any type in a `GhostMapping`, not just `mapping` types
- Do not attempt to parse NatSpecs on lines that begin with just `//`