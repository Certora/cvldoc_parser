import cvldoc_parser

parsed = cvldoc_parser.parse("definition_test.spec")
assert len(parsed) == 3, "should parse to 3 elements"
