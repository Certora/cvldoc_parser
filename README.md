# cvldoc_parser
This program parses Certora's `CVLDoc` comments. It contains the following modules:
* [`parse`](/src/parse), which lexes and parses the subset of `CVL` required to be compatible with `CVLDoc`, including the `CVLDoc` documentation blocks
* [`python_wrapper`](/src/python_wrapper), which exports the Python package `cvldoc_parser` using [`PyO3`](https://pyo3.rs). This is also used by [`cvldocTool`](https://github.com/Certora/cvldocTool).