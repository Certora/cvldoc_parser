from enum import Enum
from os import PathLike
from typing import Any, Dict, List, Optional, Union

class DocumentationTag:
    kind: str
    description: str
    def param_name_and_description(self) -> Optional[tuple[str, str]]: ...

class AstKind(Enum):
    FreeFormComment = 0
    Rule = 1
    Invariant = 2
    Function = 3
    Definition = 4
    GhostFunction = 5
    GhostMapping = 6
    Methods = 7
    Import = 8
    Using = 9
    UseRule = 10
    UseBuiltinRule = 11
    UseInvariant = 12
    HookSload = 13
    HookSstore = 14
    HookCreate = 15
    HookOpcode = 16

class TagKind(Enum):
    Title = 0
    Notice = 1
    Dev = 2
    Param = 3
    Return = 4
    Formula = 5

class Span:
    start: int
    end: int

class Ast:
    kind: AstKind
    data: Dict[str, Any]

class CvlElement:
    doc: List[DocumentationTag]
    ast: Ast
    def span(self) -> Span: ...
    def raw(self) -> str: ...
    def element_name(self) -> Optional[str]: ...
    def element_returns(self) -> Optional[str]: ...
    def element_params(self) -> Optional[List[tuple[str, str]]]: ...

def parse(path: Union[str, PathLike]) -> List[CvlElement]: ...
def parse_string(src: str) -> List[CvlElement]: ...
