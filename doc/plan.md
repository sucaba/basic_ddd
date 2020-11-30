# Plan

0. apply() releases old resources which can be stored in undo event
1. aggregate example: add_new_item should be atomic
4. apply should return result which is Err if event is inconsistent with
   current implementation.
5. Wrapper around type to ensure 'not deleted' on a compile time.
Use in 'aggregate' example.
6. Result of event processing functions
7. support transactional boundaries in event processing
8. macros to simplify usage


