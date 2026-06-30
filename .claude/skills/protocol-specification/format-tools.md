# Machine-readable format tools

When a protocol has a fixed binary layout, you can keep the layout in one
machine-readable file as the source of truth, then generate parsers and
documentation from it. The AsciiDoc spec then references that file instead of
restating every byte offset by hand, which removes the risk of the prose and
the code drifting apart.

This is an offer to the user, not a requirement. A spec built from prose,
tables, and diagrams is complete on its own. Reach for one of these tools when
the format is stable, binary, and implemented in more than one language.

## The options

### Kaitai Struct

You describe a binary format in a `.ksy` file (YAML). Kaitai generates parsers
in C++, Rust, Python, Java, and others, and produces documentation and a hex
visualizer. This is the closest fit to "describe the wire format once, get
readers everywhere." Best when you are parsing an existing binary format and
want the description to be the single source.

When it fits: a fixed binary frame with a known layout, consumed by tools in
several languages. A good match for reading device telemetry or a sensor
frame.

When it does not: it describes how to read a format, not how to write one. For
a protocol you both send and receive, you still hand-write the encoder, or pair
it with another tool.

### CDDL (Concise Data Definition Language)

Defines the shape of CBOR and JSON data. If the payload is CBOR, CDDL states
which fields are present, their types, and which are optional. It validates
data against the definition.

When it fits: a CBOR or JSON payload. This is the natural choice when the
encoding layer is already CBOR.

When it does not: it describes the data shape, not the framing or the byte
layout of a custom binary header.

### Protocol Buffers, Cap'n Proto, FlatBuffers

Schema-first formats. You write a schema, and the tool generates both encoder
and decoder in many languages. They are efficient and well supported.

When they fit: you control both ends and are willing to adopt the tool's own
wire format. Good for a new internal protocol where you do not need to match an
existing layout.

When they do not: they impose their framing and encoding. You cannot use them
to document a layout that someone else already defined.

### ASN.1

A mature standard for describing data structures, with several defined
encodings (DER, BER, PER). Common in telecom and security protocols, including
certificate formats.

When it fits: you must interoperate with a standard that is already specified
in ASN.1, or you need a formally specified encoding.

When it does not: the tooling is heavier and the learning cost is higher than
the alternatives above for a small in-house protocol.

## How to combine a format file with the AsciiDoc spec

1. Keep the format file (`.ksy`, `.cddl`, `.proto`) in the repository next to
   the spec.
2. In the encoding layer of the spec, reference the file by path and state that
   it is the source of truth for the byte layout.
3. Keep the message catalog, sequence diagrams, error handling, security, and
   versioning in the AsciiDoc spec. A format file describes bytes; it does not
   describe when a message is allowed, what it means, or how the session
   behaves.
4. If the build can run the format tool, have it check real captured frames
   against the definition as part of CI, so a drift between the spec and the
   wire is caught early.
