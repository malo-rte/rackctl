# Diagram guidance

AsciiDoc renders both PlantUML and mermaid through asciidoctor-diagram. Pick
one syntax for sequence and state diagrams and use it consistently in one
document. Use a mermaid packet diagram for byte layouts regardless of which you
chose for the others, because PlantUML has no clean byte-layout form.

## Which diagram for which job

- Connection state: a state diagram. Either tool works.
- Message exchange: a sequence diagram. Either tool works.
- Frame byte layout: a mermaid packet diagram, or a separate tool such as
  bytefield-svg if you need richer bit-field pictures.

## PlantUML sequence

```
[plantuml]
....
@startuml
Controller -> Node : ReadRegister(addr=0x10)
Node --> Controller : ReadReply(value=0x1234)
@enduml
....
```

## PlantUML state

```
[plantuml]
....
@startuml
[*] --> Closed
Closed --> Open : handshake complete
Open --> Closed : close or failure
@enduml
....
```

## mermaid sequence

```
[mermaid]
....
sequenceDiagram
    Controller->>Node: ReadRegister(addr=0x10)
    Node-->>Controller: ReadReply(value=0x1234)
....
```

## mermaid state

```
[mermaid]
....
stateDiagram-v2
    [*] --> Closed
    Closed --> Open: handshake complete
    Open --> Closed: close or failure
....
```

## mermaid packet (byte layout)

The packet diagram shows fields laid out across byte positions. Each row gives
a byte range and a field name. This is the picture that replaces a wall of
prose about offsets.

```
[mermaid]
....
packet-beta
    0-0: "SOF (0x7E)"
    1-1: "Length"
    2-2: "Type"
    3-6: "Payload"
    7-8: "CRC-16"
    9-9: "EOF (0x7E)"
....
```

The numbers are byte offsets, inclusive. Adjust the ranges to the real field
widths. If the rendering toolchain is on an older mermaid that does not support
`packet-beta`, fall back to the frame-layout table in the template and note
that the picture needs a mermaid upgrade.
