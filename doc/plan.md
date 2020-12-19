# Plan

1. 2 strategies for event sourcing of Undoable:
    - undo_all and write redos
    - keep all redos by cloning applied event
4. apply should return result which is Err if event is inconsistent with
   current implementation.
5. Wrapper around type to ensure 'not deleted' on a compile time.
Use in 'aggregate' example.
6. Result of event processing functions
7. support transactional boundaries in event processing
8. macros to simplify usage


