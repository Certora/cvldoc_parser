# NatSpec Parser: Python module
Exposes a Python API for the NatSpec module. WIP.

## Usage
Clone the repo (including this module's parent directories)
```bash
$ cd src/python_wrapper
$ python -m venv .env
$ source .env/bin/activate
$ pip install maturin
```

## API
Currently exposes a single function, `natspec_parser.parse(paths)`. It takes a list of file paths as strings, and returns a list of parsed NatSpec objects for each path.