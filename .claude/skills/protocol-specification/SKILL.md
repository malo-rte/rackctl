---
name: protocol-spec
description: >-
  Write or review a specification for a message-based communication protocol:
  a wire format, a frame format, a request/response or publish/subscribe
  protocol over TCP, UART, CAN, SPI, or similar. Use this whenever the user is
  designing a protocol, defining a message format, documenting how two devices
  or services talk to each other on the wire, or asking for a review of an
  existing protocol document. Trigger it even when the user only mentions
  "messages," "frames," "packet format," "wire format," "the protocol between
  X and Y," or hands over a draft protocol document to check, as long as the
  subject is the structure and exchange of messages rather than application
  business logic. Do not trigger for REST API design over HTTP where the
  framing and transport are already fixed by HTTP, unless the user is defining
  a custom binary or framed layer underneath.
---

# Protocol specification

This skill produces and reviews specifications for message-based protocols.
A protocol spec written this way is a Design document in the four-document
framework (Requirement, Architecture, Design, User Manual). It sits below an
Architecture document and above the code.

The output is AsciiDoc, because AsciiDoc renders both PlantUML and mermaid
diagram blocks through asciidoctor-diagram.

## Two modes

Decide which mode you are in from what the user asks.

### Author mode

The user wants a new protocol spec, or wants to extend an existing one.

1. Copy `template.adoc` into the user's documentation tree. Ask where if it
   is not obvious. Do not write the spec into this skill directory.
2. Walk the sections in the order the template lists them. Fill what you can
   from the conversation. For the rest, interview the user. The questions a
   template cannot answer for you are:
   - What is the medium, and what does it guarantee? TCP gives ordered,
     reliable bytes with no message boundaries. UART gives a raw byte stream
     with no delivery guarantee. CAN gives short framed messages with
     priority. The medium decides whether you need your own framing, your own
     ordering, and your own retransmission.
   - Can any message be sent without a request first (unsolicited)? This
     decides whether the protocol is pure request/response or also has
     notifications.
   - How does a receiver match a response to the request that caused it? By a
     message ID, by a sequence number, or by the rule that only one request is
     open at a time.
   - Where do identity, confidentiality, and integrity come from? From the
     transport (for example mTLS), or added per message (a signature, a MAC)?
   - Does resending the same message twice cause harm? This is idempotency,
     and it decides whether blind retransmission is safe.
3. Fill the tables as the answers arrive. Leave the literal marker `TODO` in
   any cell or block you cannot complete yet, so the gap stays visible and the
   linter can find it.
4. For each message, write the sequence diagram and the prose description.
   Choose PlantUML or mermaid per `references/diagrams.md`. Be consistent
   within one document.
5. When the draft is filled in, run the linter (see below) and close every
   gap it reports, or tell the user why a gap is intentional.

### Review mode

The user points at an existing protocol document and wants it checked.

1. Read the document.
2. Check it against the full required-content list in
   `references/checklist.md`. That file holds the complete list so this file
   stays short.
3. Report what is missing, grouped by section, concrete and specific. Name the
   message that is missing its error responses, not just "some messages lack
   errors."
4. If the document is AsciiDoc and follows the template's structure, run the
   linter to back up the manual review with a mechanical pass.

The gaps that show up most often, in order:
- error responses listed per message, not just one global error section
- timeout and retry rules
- idempotency of each message
- byte order stated once for the whole protocol
- what the receiver does with a malformed frame (bad length, bad CRC)
- what the receiver does with an unknown message type

## The linter

`lint_protocol.py` is a hand-rolled checker with no third-party dependencies.
It parses the AsciiDoc and reports two kinds of gap: a required section that is
absent, and a required field that still holds the `TODO` marker. It exits
non-zero when it finds a gap, so it drops into CI next to a `bump-version`
step.

Run it:

```
python3 lint_protocol.py path/to/protocol.adoc
```

Add `--json` for machine-readable output if a CI job needs to parse it.

The linter checks structure, not meaning. It cannot tell you that a CRC covers
the wrong bytes. It tells you that the CRC field was filled in at all. The
human review in Review mode is what catches wrong content.

## When the format is binary and fixed

If the protocol has a fixed binary layout, offer to keep the byte layout in one
machine-readable file as the source of truth, and have the AsciiDoc spec
reference it instead of restating every offset by hand. See
`references/format-tools.md` for the options (Kaitai Struct, CDDL, Protocol
Buffers, and others) and when each one fits. This is an offer, not a
requirement. A prose-and-table spec is complete on its own.

## Files in this skill

- `template.adoc` - the empty spec, sections and tables ready to fill.
- `examples/framed-uart-protocol.adoc` - a small worked example: a
  request/response protocol over UART with start/end framing and a CRC.
- `references/checklist.md` - the full required-content list, used in Review
  mode.
- `references/diagrams.md` - how to choose and write PlantUML, mermaid
  sequence/state, and mermaid packet (byte layout) diagrams.
- `references/format-tools.md` - machine-readable format tools and when to use
  one.
- `lint_protocol.py` - the structural linter.
