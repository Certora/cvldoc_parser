# NatSpec Parser: Python module
Exposes a Python API for the NatSpec module. WIP.

## Usage
First, make sure **Rust 1.62** or newer is installed.
Clone [the entire repo](https://github.com/Certora/natspec_parser), then from the project base:
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
>>> import natspec_parser
```

## API
Currently exposes a single function, `natspec_parser.parse(paths)`. It takes a list of file paths as strings, and returns a list of parsed NatSpec objects for each path.
A parsed NatSpec object is either a `Documentation` or a `FreeForm`. They both support a `diagnostics()` method, that returns a list of warnings or errors associated with that object.