# Plan

0. Event optimizations:
    - Create + Update ... -> Create // MUST due to invariants
    - Update + Update ... -> Update // MUST due to invariants
1. Result of modification functions
2. Result of event processing functions
3. support transactional boundaries in event processing
4. macros to simplify usage
5. Event optimizations:
    - Create + ... + Delete -> []   // good to have
    - Update + ... + Delete -> Delete // good to have
