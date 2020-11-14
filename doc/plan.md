# Plan

1. Throw away optimization. It can be implemented in
   `Streamable::stream_to` or optimization can happen even before
   applying optimizations or event as a `StreamEvents` middleware.
2. all mutation methods should return `Result<Changes>` as alternative
   to storing changes in the `Primary` or `OwnedCollection` itself. This
   way we can implement mutation methods as a trait and we can store
   changes in single place.
3. Trait for Event Storage
4. apply should return result which is Err if event is inconsistent with
   current implementation.
5. Wrapper around type to ensure 'not deleted' on a compile time.
Use in 'aggregate' example.
6. Result of event processing functions
7. support transactional boundaries in event processing
8. macros to simplify usage

