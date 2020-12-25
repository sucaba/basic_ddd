# Plan

0. Abstract away Change
0. Problem: `Changes` is used in `Changable` trait. Fix by making apply
   generic with `TChange` parameter
0. Abstract away Record
0. Abstract away Changes
1. `Change` abstraction should satisfy both undo-only and redo+undo
   cases.
0. Change abstraction should be driven from Record 
4. apply should return result which is Err if event is inconsistent with
   current implementation.
5. Wrapper around type to ensure 'not deleted' on a compile time.
Use in 'aggregate' example.
6. Result of event processing functions
7. support transactional boundaries in event processing
8. macros to simplify usage


