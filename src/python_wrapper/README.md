# CVLDoc Parser: Python module
Exposes a Python API for the CVLDoc module. 
Currently exposes a single function, `cvldoc_parser.parse(path)`. It takes a single path, and returns a list of parsed CVLDoc objects.

## Install
Up-to-date builds (built by the CircleCI script) are available on Test `PyPI`: 

`pip install --pre --index-url https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple cvldoc_parser`

## Build
First, [install Rust](https://rustup.rs/). 
Clone [the entire repo](https://github.com/Certora/cvldoc_parser), then from the project base:
```bash
$ cd src/python_wrapper
$ python -m venv .env
$ source .env/bin/activate
$ pip install maturin
```
It creates a virtual environment for development, and should only be done once. 
Now, while the `.env` is sourced, it is possible to run
```bash
$ maturin build
```
in order to build the module, which can then be used in the `virtualenv` or installed globally with `pip install {generated .whl file}`. It is also possible to run `maturin develop` in order to generate a temporary module.

It is now possible to import the module:
```bash
$ python
>>> import cvldoc_parser
```
