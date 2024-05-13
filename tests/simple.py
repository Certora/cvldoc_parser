from pathlib import Path

import cvldoc_parser


def as_list(
    elements: list[cvldoc_parser.CvlElement],
) -> list[tuple[str, str | None, str | None, list[tuple[str, str]] | None]]:
    return [
        (
            x.raw(),
            x.element_name(),
            x.element_returns(),
            x.element_params(),
        )
        for x in elements
    ]


spec_file = Path(__file__).parent / "definition_test.spec"

p_file = cvldoc_parser.parse(spec_file)

assert len(p_file) == 3, "should parse to 3 elements"

p_file_as_string = cvldoc_parser.parse(spec_file.as_posix())
p_from_string = cvldoc_parser.parse_string(spec_file.read_text())

p = map(as_list, [p_file, p_file_as_string, p_from_string])
assert all(
    x == y for x in p for y in p
), "all three parsers should parse the same elements"
