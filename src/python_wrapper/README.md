# NatSpec Parser: Python module
Exposes a Python API for the NatSpec module. WIP.

## Usage
Clone [the entire repo](https://github.com/Certora/natspec_parser), then from the project base:
```bash
$ cd src/python_wrapper
$ python -m venv .env
$ source .env/bin/activate
$ pip install maturin
$ maturin develop
```

This should only be done once. It creates a virtual environment and builds the module. The only necessary step in future runs is to `source` the `env`.
It is now possible to
```bash
$ python
>>> import natspec_parser
```

## API
Currently exposes a single function, `natspec_parser.parse(paths)`. It takes a list of file paths as strings, and returns a list of parsed NatSpec objects for each path.