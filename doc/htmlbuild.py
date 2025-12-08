from collections.abc import Iterable
import os
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import assert_never

from ._glob import globify, unglob

base_url = os.getenv("INTEL_BASE_URL", "https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html")
base_url = base_url.strip()
assert base_url.startswith(("http://", "https://", "file://")),\
    f"base_url {base_url!r} doesn't look right"

intrinsics_glob_path = Path(__file__).parent / "intrinsics.glob"
doc_path = Path(__file__).parent / "doc.in"

tech_to_intrinsics: dict[str, list[str]] = defaultdict(list)
tech = "?"

for line in intrinsics_glob_path.open(encoding="utf-8"):
    line = line.strip()
    if line.startswith("#"):
        tech = line[1:].strip()
    else:
        tech_to_intrinsics[tech].extend(unglob(line))

# Parsing doc.in:

@dataclass
class InsTechs:
    techs: list[str]

@dataclass
class InsPattern:
    title: str
    pat: str
    lines: list[str]

@dataclass
class InsExhaust:
    pat: re.Pattern[str]


type Instruction = InsTechs | InsPattern | InsExhaust


@dataclass
class ParseError(Exception):
    lineno: int
    message: str

    def __str__(self) -> str:
        return f"at line {self.lineno}: {self.message}"

@dataclass
class VerifyError(Exception):
    messages: list[tuple[int, str]]

    def __str__(self) -> str:
        return "\n".join([
            f"at line {lineno}: {message}"
            for lineno, message in self.messages])


def parse_instructions(lines: Iterable[str]) -> list[tuple[int, Instruction]]:
    instructions: list[tuple[int, Instruction]] = []
    current_pattern: InsPattern | None = None

    techs_seen = False

    for lineno, line in enumerate(lines, start=1):
        line = line.strip()
        if not line:
            continue

        if line.startswith("@techs"):
            if techs_seen:
                raise ParseError(lineno, "duplicate @techs directive")
            techs_seen = True
            selected_techs = line.split(" ", 1)[1].split(",")
            instructions.append((lineno, InsTechs(selected_techs)))
        elif line.startswith("@"):
            current_pattern = None
            if not techs_seen:
                raise ParseError(lineno, "expected @techs directive first")

            if line.startswith("@pattern"):
                title = line.split(" ", 1)[1].strip()
                pattern = title.split("(",1)[0].strip()
                ins = InsPattern(title, pattern, [])
                instructions.append((lineno, ins))
                current_pattern = ins
            elif line.startswith("@exhaust"):
                regex = re.compile(line.split(" ", 1)[1].strip())
                instructions.append((lineno, InsExhaust(regex)))
            else:
                raise ParseError(lineno, "unknown directive")
        else:
            if not line.startswith("#"):
                if current_pattern is None:
                    raise ParseError(lineno, "unexpected plain line")
                current_pattern.lines.append(line)

    return instructions


# Rendering doc:

@dataclass
class Paragraph:
    title: str
    expansions: list[str]
    lines: list[str]


# returns (techs, paragraphs)
def produce_doc(instructions: Iterable[tuple[int, Instruction]]) -> tuple[list[str], list[Paragraph]]:
    paragraphs: list[Paragraph] = []

    errors: list[tuple[int, str]] = []

    selected_techs: list[str] = []
    selected_intrinsics: set[str] = {*()}
    handled_intrinsics: set[str] = {*()}

    for lineno, instruction in instructions:
        match instruction:
            case InsTechs(techs):
                if unknown_techs := set(techs) - tech_to_intrinsics.keys():
                    raise VerifyError([(lineno, f"unknown techs: {unknown_techs}")])
                selected_techs = techs
                selected_intrinsics = set(i for tech in techs for i in tech_to_intrinsics[tech])

            case InsExhaust(regex):
                if not any(regex.fullmatch(i) for i in selected_intrinsics):
                    errors.append((lineno, f"regex {regex} doesn't match any intrinsics"))

                unhandled = [i for i in selected_intrinsics - handled_intrinsics if regex.fullmatch(i)]
                patterns = globify(unhandled)
                for pat in patterns:
                    errors.append((lineno, f"{pat} not handled"))

            case InsPattern(title, pattern, lines):
                intrinsics = unglob(pattern)
                expansions = selected_intrinsics.intersection(intrinsics)
                if not expansions:
                    errors.append((lineno, f"pattern {pattern} doesn't match anything in {selected_techs}"))
                handled_intrinsics.update(intrinsics)
                paragraphs.append(Paragraph(title, sorted(expansions), lines))

            case other:
                assert_never(other)

    if errors:
        raise VerifyError(errors)
    else:
        return selected_techs, paragraphs


try:
    instructions = parse_instructions(doc_path.open(encoding="utf-8"))
except ParseError as e:
    sys.stderr.write(f"htmlbuild.py: parse error:\n{e}\n")
    sys.exit(1)

try:
    techs, paragraphs = produce_doc(instructions)
except VerifyError as e:
    sys.stderr.write(f"htmlbuild.py: did not verify doc:\n{e}\n")
    sys.exit(1)

base_url_with_techs = base_url + "#techs=" + ",".join(techs)


def backticks_to_html(line: str) -> str:
    return re.sub(r"`([^`]+)`", r"<code>\1</code>", line)


print("<!doctype html><html>")
print("<!-- auto-generated by `python -m doc.htmlbuild`, do not edit directly -->")
print("""
<head><style>
* {
    margin: 0;
    padding: 0;
    font-family: system-ui, sans-serif;
}

hr { margin-bottom: 1rem;  }
html { padding: 0.5rem 1rem; }
h2 { font-size: 1.2rem; }
li { margin-left: 1.5rem; }
summary { font-size: 1rem; color: #315FA0; cursor: pointer; }
.pattern { margin-bottom: 1rem; }
.pattern-description { max-width: 50em; }

.tt, code {
    font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', Menlo, Consolas, 'DejaVu Sans Mono', monospace;
}

code {
    background: #f0f0f0;
}

</style></head>
""")
print("<body>")
print("<p>Techs covered: " + ",".join(techs) + "</p>")
print("<hr>")

for para in paragraphs:
    print(f"<h2 class='tt'>{para.title}</h2>")
    print("<div class=pattern>")

    if len(para.expansions) > 1:
        print("<details>")
        print("<summary>Covered instructions</summary>")
        print("<ul>")
        for intr in para.expansions:
            print(f'<li><a class="tt" href="{base_url_with_techs}&text={intr}">{intr}</a></li>')
        print("</ul>")
        print("</details>")
    else:
        [exp] = para.expansions
        print(f'<p><a class="tt" href="{base_url_with_techs}&text={exp}">{exp}</a></p>')

    print("<p class=pattern-description>")
    for line in para.lines:
        print(backticks_to_html(line))
    print("</p>")
    print("</div>")
print("</body></html>")
