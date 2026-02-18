# Knox Memory Model

## Direction (post-MVP)

- **Ownership:** Values are owned by one binding or one aggregate. No implicit copying of non-Copy types (to be defined).
- **Borrowing:** References (`&T`, `&mut T`) and borrowing rules (shared xor mutable) are a goal; not implemented in MVP.
- **Move semantics:** Assignment and argument passing move by default for non-Copy types.

## MVP

- MVP does not implement ownership or borrowing. All types are treated as copyable or as opaque values for codegen.
- Memory model docs exist to lock in the intended direction so the compiler and IR can evolve without contradicting future rules.
