#!/usr/bin/env python3
"""Structural linter for protocol specification documents.

Checks an AsciiDoc protocol spec for two kinds of gap:
  - a required section is absent
  - a required field still holds the TODO marker, or a required field label
    is missing from a message

It checks structure, not meaning. It can confirm that a CRC field was filled
in; it cannot confirm the CRC covers the right bytes. Pair it with a human
read.

Exit status is 0 when no errors are found, 1 otherwise. Warnings do not change
the exit status.

Usage:
    python3 lint_protocol.py path/to/protocol.adoc
    python3 lint_protocol.py path/to/protocol.adoc --json
"""

import argparse
import json
import re
import signal
import sys

# Behave like a normal command-line tool when output is piped to head/tail.
try:
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)
except (AttributeError, ValueError):
    pass  # SIGPIPE is not available on every platform

REQUIRED_SECTIONS = [
    "Overview",
    "Transport assumptions",
    "Session lifecycle",
    "Message catalog",
    "Encoding layer",
    "Error handling",
    "Versioning and negotiation",
    "Security",
    "Constants and registries",
]

REQUIRED_MESSAGE_FIELDS = [
    "Message type",
    "Direction and initiator",
    "Parameters",
    "Valid parameter ranges",
    "Pre-requisite",
    "Correlation",
    "Responses",
    "Error responses",
    "Timeout and retry",
    "Idempotency",
]

DIAGRAM_BLOCK = re.compile(r"^\[(plantuml|mermaid)\]\s*$")
HEADING = re.compile(r"^(=+)\s+(.*\S)\s*$")
MESSAGE_HEADING = re.compile(r"^===\s+Message:\s+(.*\S)\s*$")
CELL = re.compile(r"^\|\s*(.*?)\s*$")


def normalize(text):
    return re.sub(r"\s+", " ", text).strip().lower()


class Gap:
    def __init__(self, severity, line, section, message):
        self.severity = severity
        self.line = line
        self.section = section
        self.message = message

    def as_dict(self):
        return {
            "severity": self.severity,
            "line": self.line,
            "section": self.section,
            "message": self.message,
        }


def split_sections(lines):
    """Return a list of (level, title, start_line, end_line) for == and === headings."""
    sections = []
    for i, line in enumerate(lines):
        m = HEADING.match(line)
        if not m:
            continue
        level = len(m.group(1))
        title = m.group(2)
        sections.append([level, title, i, len(lines)])
    # set each section's end to the next heading of the same or higher level
    for idx in range(len(sections)):
        level = sections[idx][0]
        end = len(lines)
        for j in range(idx + 1, len(sections)):
            if sections[j][0] <= level:
                end = sections[j][2]
                break
        sections[idx][3] = end
    return sections


def check_required_sections(sections, gaps):
    present = {normalize(title) for level, title, _, _ in sections if level == 2}
    for required in REQUIRED_SECTIONS:
        if normalize(required) not in present:
            gaps.append(Gap("error", None, required,
                            "required section is missing: " + required))


def message_field_values(lines, start, end):
    """Map each cell label to (label_line, value_text) inside a message block.

    The template writes each field as one cell holding the label, followed by
    one or more cells holding the value, until the next label cell. This reads
    the label cell, then collects following lines until the next cell that
    matches a known field label or the table ends.
    """
    labels_norm = {normalize(f): f for f in REQUIRED_MESSAGE_FIELDS}
    result = {}
    i = start
    current_label = None
    current_label_line = None
    value_parts = []

    def flush():
        if current_label is not None:
            result[current_label] = (current_label_line, " ".join(value_parts).strip())

    while i < end:
        line = lines[i]
        m = CELL.match(line)
        if m and line.strip().startswith("|"):
            content = m.group(1).strip()
            if normalize(content) in labels_norm:
                flush()
                current_label = labels_norm[normalize(content)]
                current_label_line = i + 1
                value_parts = []
            else:
                if content:
                    value_parts.append(content)
        else:
            stripped = line.strip()
            if stripped and not stripped.startswith("|===") and current_label is not None:
                value_parts.append(stripped)
        i += 1
    flush()
    return result


def has_diagram(lines, start, end):
    for i in range(start, end):
        if DIAGRAM_BLOCK.match(lines[i]):
            return True
    return False


def check_messages(lines, sections, gaps):
    message_sections = []
    for level, title, start, end in sections:
        if level == 3 and MESSAGE_HEADING.match(lines[start]):
            name = MESSAGE_HEADING.match(lines[start]).group(1)
            message_sections.append((name, start, end))

    if not message_sections:
        gaps.append(Gap("error", None, "Message catalog",
                        "no messages found; expected one '=== Message: ...' per message"))
        return

    for name, start, end in message_sections:
        values = message_field_values(lines, start, end)
        for field in REQUIRED_MESSAGE_FIELDS:
            if field not in values:
                gaps.append(Gap("error", start + 1, "Message: " + name,
                                "missing field '" + field + "'"))
            else:
                line_no, text = values[field]
                if not text or "TODO" in text:
                    gaps.append(Gap("error", line_no, "Message: " + name,
                                    "field '" + field + "' is not filled in"))
        if not has_diagram(lines, start, end):
            gaps.append(Gap("error", start + 1, "Message: " + name,
                            "no sequence diagram (expected a [plantuml] or [mermaid] block)"))


def check_session_diagram(lines, sections, gaps):
    for level, title, start, end in sections:
        if level == 2 and normalize(title) == normalize("Session lifecycle"):
            if not has_diagram(lines, start, end):
                gaps.append(Gap("warning", start + 1, "Session lifecycle",
                                "no state diagram; acceptable only if the protocol is connectionless"))
            return


def check_remaining_todos(lines, sections, gaps):
    # Report any TODO marker left anywhere, with its nearest preceding heading.
    def section_for(line_index):
        name = "(document header)"
        for level, title, start, _ in sections:
            if start <= line_index:
                name = title
            else:
                break
        return name

    in_comment_block = False
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped == "////":
            in_comment_block = not in_comment_block
            continue
        if in_comment_block:
            continue
        if stripped.startswith("//"):
            continue
        if "TODO" in line:
            gaps.append(Gap("error", i + 1, section_for(i),
                            "unfilled TODO: " + line.strip()))


def lint(path):
    with open(path, "r", encoding="utf-8") as handle:
        lines = handle.read().splitlines()

    sections = split_sections(lines)
    gaps = []
    check_required_sections(sections, gaps)
    check_messages(lines, sections, gaps)
    check_session_diagram(lines, sections, gaps)
    check_remaining_todos(lines, sections, gaps)
    return gaps


def main():
    parser = argparse.ArgumentParser(description="Lint a protocol specification document.")
    parser.add_argument("path", help="path to the AsciiDoc protocol spec")
    parser.add_argument("--json", action="store_true", help="emit JSON output")
    args = parser.parse_args()

    gaps = lint(args.path)
    errors = [g for g in gaps if g.severity == "error"]
    warnings = [g for g in gaps if g.severity == "warning"]

    if args.json:
        print(json.dumps([g.as_dict() for g in gaps], indent=2))
    else:
        if not gaps:
            print("OK: no gaps found in " + args.path)
        else:
            for g in gaps:
                where = "line " + str(g.line) if g.line else "section"
                print("[" + g.severity + "] " + where + " (" + g.section + "): " + g.message)
            print("")
            print(str(len(errors)) + " error(s), " + str(len(warnings)) + " warning(s)")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
