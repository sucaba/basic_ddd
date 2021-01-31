# Plan

0. NestedEvent should contain only NesteObjEvent and not Id. This is 
useful because when loading events from DB, there will be only NesteObjEvent
0. Details should not be indexable by `usize` but by `Id`
1. `Change` abstraction should satisfy both undo-only and redo+undo
   cases.
4. apply should return result which is Err if event is inconsistent with
   current implementation.
5. Wrapper around type to ensure 'not deleted' on a compile time.
Use in 'aggregate' example.
8. macros to simplify usage




