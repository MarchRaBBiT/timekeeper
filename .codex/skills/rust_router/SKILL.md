---
name: Rust Router
description: Routes Rust programming questions to the appropriate specialized skill or provides general guidance.
---

# Rust Question Router

Use this skill when the user asks general questions about Rust concepts or when you need to decide which specialized Rust skill to apply.

## Routing Logic

### 1. Ownership & Borrowing
If the question is about:
- "value used here after move"
- "borrowed value does not live long enough"
- reference rules
- lifetimes ('a)

-> **Use `coding_guidelines`** (as temporary placeholder for m01-ownership) or explain using the "Ownership Rules" below.

### 2. Smart Pointers & Memory
If the question is about:
- `Box`, `Rc`, `Arc`, `RefCell`
- Heap allocation
- Interior mutability

-> **Use `coding_guidelines`** (placeholder for m02-resource) or refer to "Smart Pointers" section below.

### 3. Error Handling
- `Result`, `Option`
- `?` operator vs `unwrap()`
- Custom error types

-> **Use `coding_guidelines`** (placeholder for m06-error-handling).

### 4. Unsafe Code
- `unsafe` blocks
- Raw pointers
- FFI

-> **Use `unsafe_checker`**.

## General Principles (Quick Reference)

### Ownership Rules
1. Each value in Rust has a variable that's called its owner.
2. There can only be one owner at a time.
3. When the owner goes out of scope, the value will be dropped.

### Smart Pointers Cheat Sheet
- **`Box<T>`**: Heap allocation, single owner.
- **`Rc<T>`**: Multiple owners, single thread.
- **`Arc<T>`**: Multiple owners, thread-safe.
- **`RefCell<T>`**: Mutability inside an immutable structure (interior mutability), runtime checks.
- **`Mutex<T>`**: Thread-safe interior mutability.

### Concurrency
- `Send`: Safe to transfer ownership to another thread.
- `Sync`: Safe to share references between threads (`&T` is `Send`).
