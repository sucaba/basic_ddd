# Plan

0. Streamable InMemoryStorage for events for eventual consistency.
Will be useful for testing. Should be implemented in basic_ddd.testing
crate
1. all modifications through apply(event) method
2. Wrapper around type to ensure 'not deleted' on a compile time.
Use in 'aggregate' example
3. Result of event processing functions
4. support transactional boundaries in event processing
5. macros to simplify usage

