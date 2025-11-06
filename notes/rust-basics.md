---
title: Rust Programming Basics
---

# Rust Programming Basics

Rust is a systems programming language focused on safety, speed, and concurrency.

## Key Concepts

### Ownership
- Each value has a single owner
- When the owner goes out of scope, the value is dropped
- Prevents memory leaks and data races

### Borrowing
- References allow you to refer to a value without taking ownership
- `&T` is an immutable reference
- `&mut T` is a mutable reference

### Lifetimes
- Ensure references are valid for as long as they're used
- Prevent dangling references

## Example Code

```rust
fn main() {
    let s = String::from("hello");
    let len = calculate_length(&s);
    println!("The length of '{}' is {}.", s, len);
}

fn calculate_length(s: &String) -> usize {
    s.len()
}
```

## Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
