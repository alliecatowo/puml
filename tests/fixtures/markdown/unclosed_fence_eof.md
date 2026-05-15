# Unterminated markdown fence should ingest through EOF

```puml
@startuml
Alice -> Bob: eof-unclosed
@enduml
