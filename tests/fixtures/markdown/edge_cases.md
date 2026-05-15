# Markdown fence edge cases

    ```puml
    @startuml
    Ignored -> Because: four-space indent means this is not a fence
    @enduml
    ```

~~~PuMl
@startuml
Alice -> Bob: tilde-puml
@enduml
~~~

```MERMAID
sequenceDiagram
participant Bob
participant Carol
Bob->>Carol: uppercase-mermaid
```

   ```puml
@startuml
Carol -> Dave: three-space-indent
@enduml
   ```
