# Plan


0. Remove AllDeleted event because there is no existing

anti-event and even if exists, it seems too heavy.
0. Atomic undo:
    - begin is called on Changable and not on changes
    - Change entry should contain pair event and anti-event for
        compensation or undo logic
1. aggregate example: add_new_item should be atomic
4. apply should return result which is Err if event is inconsistent with
   current implementation.
5. Wrapper around type to ensure 'not deleted' on a compile time.
Use in 'aggregate' example.
6. Result of event processing functions
7. support transactional boundaries in event processing
8. macros to simplify usage


