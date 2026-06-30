# Required-content checklist

Use this list in Review mode. Go through it against the document and report
every item that is absent or incomplete. Be specific: name the message that
lacks error responses, not just the category.

## Transport assumptions

- The medium: TCP socket, UART, CAN, SPI, or similar.
- What the medium guarantees: ordering, reliability, message boundaries.
- What the protocol must add itself: framing, ordering, retransmission.
- Maximum message size.
- Maximum frame size.
- Byte order, stated once for the whole protocol.

## Session lifecycle

- How a session opens: the handshake steps and who speaks first.
- How it closes cleanly.
- How it ends on failure: timeout, reset, transport drop.
- Keepalive or heartbeat interval, and the meaning of a miss.
- A state diagram of the connection.
- If the protocol is connectionless, a plain statement that it is, in place of
  the above.

## Message layer, per message

- Message type.
- Direction and initiator; whether it can be sent unsolicited.
- Parameters, each with a type and a meaning.
- Valid parameter ranges, and what counts as invalid.
- Pre-requisite, or "none".
- Correlation: how a response matches the request.
- Normal responses.
- Error responses specific to this message, kept separate from normal
  responses.
- Timeout and retry rules.
- Idempotency: is resending safe.
- A sequence diagram.
- A prose description.

## Encoding layer

- How a message splits into frames, when the medium needs framing.
- Encoding rules: how each field maps to bytes.
- Frame markers: start, end, or sync pattern, or a clear "none".
- Length field: position, width, and what it counts.
- Integrity check: CRC type or checksum, and which bytes it covers.
- Byte stuffing or escaping, when a marker byte can appear in the payload.
- Alignment and padding.
- Variable-length fields: how length is signalled.
- A byte-layout picture of the frame (recommended, not required).

## Error handling

- The full list of error codes with meanings.
- What the receiver does with an unknown message type.
- What it does with a malformed frame: bad length.
- What it does with a failed integrity check: bad CRC.
- What happens when a timeout fires.

## Versioning and negotiation

- The version field and its location.
- How the two sides agree on a version.
- The rule for unknown fields or message types, so old and new peers can talk.

## Security

- How each side proves its identity.
- Which message types require authorization.
- Whether confidentiality and integrity come from the transport or per message.
- Replay protection: nonce, sequence number, or timestamp.

## Constants and registries

- A single table of message type values.
- Tables for every other enumeration with its numeric value.
- Reserved and forbidden values.
