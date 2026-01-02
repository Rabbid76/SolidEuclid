# Comprehensive Rust Language Feature Summary

## 1. General Rules & Principles

- **Ownership:** Values have exactly one owner. When the owner goes out of scope, the value is dropped.
- **Borrowing:** Data can be borrowed mutably once **OR** immutably multiple times simultaneously.
- **Lexical Scoping:** Variables are valid from declaration until the end of the enclosing block `{}`.
- **Expression-based:** Most constructs (`if`, `match`, blocks) return a value. Semicolons suppress the return value (turning it into unit `()`).

---

## 2. Variables & Constants

- **Immutable Binding:** `let x = 5;`
- **Mutable Binding:** `let mut x = 5;`
- **Type Annotation:** `let x: i32 = 5;`
- **Shadowing:** `let x = 5; let x = "str";`
- **Constants (Inlined):** `const MAX: u32 = 100;` (Type required, computed at compile time)
- **Static Variables (Fixed address):** `static GLOBAL: i32 = 5;`
- **Mutable Static (Unsafe):** `static mut COUNT: i32 = 0;`

---

## 3. Data Types

### Scalar

- **Integers:** `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, `i128`, `u128`, `isize`, `usize`
- **Floats:** `f32`, `f64`
- **Boolean:** `bool` (`true`, `false`)
- **Character:** `char` ('a', '😻')
- **Literals:** `1_000` (separator), `0xff` (hex), `0o77` (octal), `0b11` (binary), `b'A'` (byte).

### Compound

- **Tuples:** `let t: (i32, f64) = (500, 6.4);`
  - **Access:** `t.0`
  - **Destructuring:** `let (x, y) = t;`
- **Arrays (Fixed size):** `let a: [i32; 3] = [1, 2, 3];`
  - **Repeat Init:** `let a = [0; 5];` (five zeros)
  - **Access:** `a[1]`
- **Slices (Dynamically sized view):** `&[T]` or `&mut [T]`
  - **Syntax:** `&a[1..3]`

---

## 4. Functions

- **Definition:** `fn foo(x: i32) -> i32 { x + 1 }`
- **Unit Return:** `fn foo() { ... }` (Implicitly returns `()`)
- **Diverging Functions:** `fn foo() -> ! { panic!() }` (Never returns)
- **Function Pointers:** `fn(i32) -> i32`
- **Const Functions:** `const fn new() -> Self { ... }` (Callable in const context)

---

## 5. Structs & Unions

- **Definition:** `struct User { name: String, active: bool }`
- **Instantiation:** `let u = User { name: String::from("A"), active: true };`
- **Field Init Shorthand:** `User { name, active }` (if variable names match fields)
- **Struct Update Syntax:** `User { active: false, ..u1 }` (Copy remaining fields from `u1`)
- **Tuple Structs:** `struct Color(i32, i32, i32);`
- **Unit Structs:** `struct AlwaysEqual;`
- **Unions (Unsafe, C-interop):** `union MyUnion { f1: u32, f2: f32 }`

---

## 6. Enums

- **Definition:**

    ```rust
    enum Message {
        Quit,                       // Unit variant
        Move { x: i32, y: i32 },    // Struct-like variant
        Write(String),              // Tuple-like variant
    }
    ```

- **Explicit Discriminant:** `enum Color { Red = 1, Blue = 2 }`
- **Instantiation:** `Message::Write(String::from("Hi"))`

---

## 7. Pattern Matching Syntax

- **Match Expression:** `match val { pat => expr, ... }`
- **Catch-all:** `_ => ...`
- **Literals:** `1 => ...`, `"foo" => ...`
- **Multiple Patterns:** `1 | 2 => ...`
- **Ranges:** `1..=5 => ...` (Inclusive)
- **Destructuring Structs:** `Point { x, y } => ...`
  - **Renaming:** `Point { x: x1, y: y1 } => ...`
  - **Ignore rest:** `Point { x, .. } => ...`
- **Destructuring Tuples:** `(a, b) => ...`
- **Destructuring Enums:** `Message::Move { x, y } => ...`
- **Ignore Variable:** `_x` (suppresses unused warning) or `_` (completely ignored)
- **Slice Patterns:**
  - `[a, b] => ...` (Exact length)
  - `[head, tail @ ..] => ...` (Head and rest)
  - `[first, .., last] => ...` (Middle ignored)
- **Match Guards:** `Some(x) if x < 5 => ...`
- **@ Bindings:** `id @ 3..=7 => ...` (Bind matched value to `id` while testing range)
- **Reference Patterns:**
  - `&val => ...` (Matches a reference)
  - `ref val => ...` (Binds by reference instead of moving)
  - `ref mut val => ...`

---

## 8. Control Flow

- **If/Else:** `if x < 5 { ... } else if y { ... } else { ... }`
- **Let If (Expression):** `let x = if c { 1 } else { 2 };`
- **Loop:** `loop { ... }` (Infinite)
  - **Break Value:** `break 5;`
- **While:** `while condition { ... }`
- **For:** `for x in iter { ... }`
- **Ranges in For:** `for i in 0..10 { ... }` (0 to 9), `for i in 0..=10 { ... }`
- **Loop Labels:** `'outer: loop { break 'outer; }`
- **If Let:** `if let Some(x) = opt { ... }`
- **While Let:** `while let Some(x) = iter.next() { ... }`
- **Let Else (Refutable pattern binding):** `let Some(x) = opt else { return; };`

---

## 9. Generics & Traits

- **Generic Functions:** `fn foo<T>(x: T) -> T { ... }`
- **Generic Structs:** `struct Point<T> { x: T, y: T }`
- **Trait Definition:** `trait Summary { fn summarize(&self) -> String; }`
- **Default Implementation:** `fn summarize(&self) { ... }`
- **Implementation:** `impl Summary for Tweet { ... }`
- **Trait Bounds:** `fn notify<T: Summary>(item: &T)`
- **Syntax Sugar:** `fn notify(item: &impl Summary)`
- **Where Clauses:** `fn foo<T, U>(...) where T: Display + Clone, U: Clone`
- **Multiple Bounds:** `T: Display + PartialOrd`
- **Return impl Trait:** `fn make() -> impl Summary` (Opaque return type)
- **Trait Objects:** `Box<dyn Summary>` (Dynamic dispatch)
- **Supertraits:** `trait Sub: Super { ... }`
- **Associated Types:** `trait Iterator { type Item; ... }`
- **Generic Associated Types (GATs):** `type Item<'a>;`
- **Const Generics:** `struct Array<T, const N: usize> { data: [T; N] }`
- **Turbofish (Explicit types):** `vec.iter().collect::<Vec<i32>>()`

---

## 10. Ownership & Pointers

- **References:** `&T` (Shared), `&mut T` (Exclusive)
- **Dereferencing:** `*x`
- **Raw Pointers:** `*const T`, `*mut T` (Unsafe to dereference)
- **Box:** `Box::new(v)` (Heap allocation)
- **Rc:** `Rc::new(v)` (Ref counting, single-threaded)
- **Arc:** `Arc::new(v)` (Atomic ref counting, thread-safe)
- **Interior Mutability:** `RefCell<T>`, `Mutex<T>`, `RwLock<T>`

---

## 11. Lifetimes

- **Explicit Annotation:** `&'a i32`
- **Function Syntax:** `fn foo<'a>(x: &'a str) -> &'a str`
- **Struct Syntax:** `struct Ref<'a> { part: &'a str }`
- **Static Lifetime:** `'static` (Lives for entire program duration)
- **Lifetime Bounds:** `T: 'a` (Type T lives at least as long as 'a)

---

## 12. Closures

- **Syntax:** `|x| x + 1`
- **Block Syntax:** `|x| { let y = 1; x + y }`
- **Type Annotation:** `|x: i32| -> i32 { ... }`
- **Move Closure:** `move || println!("{}", x)` (Takes ownership of capture)

---

## 13. Modules & Visibility

- **Module Definition:** `mod network { ... }`
- **File Module:** `mod network;` (looks in `network.rs` or `network/mod.rs`)
- **Visibility:**
  - `fn private()` (Private to module, default)
  - `pub fn public()` (Public everywhere)
  - `pub(crate) fn internal()` (Public to crate)
  - `pub(super) fn parent()` (Public to parent module)
- **Use:** `use std::collections::HashMap;`
- **Renaming (Alias):** `use std::io as Io;`
- **Re-export:** `pub use crate::module::Item;`
- **Glob Import:** `use std::io::prelude::*;`

---

## 14. Error Handling

- **Panic:** `panic!("msg")`
- **Question Mark Operator:** `let f = File::open("f")?;` (Returns Err immediately if failed)
- **Try Block (Unstable/Nightly):** `try { ... }`

---

## 15. Macros & Attributes

- **Declarative Macros:** `macro_rules! name { (pattern) => { expansion }; }`
  - **Captures:** `$x:expr`, `$i:ident`, `$t:ty`
  - **Repetition:** `$(...)*` or `$(...)+`
- **Procedural Macros:**
  - **Derive:** `#[derive(Serialize)]`
  - **Attribute:** `#[route(GET, "/")]`
  - **Function-like:** `sql!("SELECT *")`
- **Common Attributes:**
  - `#[derive(...)]`
  - `#[cfg(target_os = "linux")]`
  - `#[test]`, `#[bench]`
  - `#[inline]`, `#[inline(always)]`
  - `#[allow(unused_variables)]`

---

## 16. Asynchronous Rust

- **Async Function:** `async fn foo() { ... }`
- **Async Block:** `async { ... }`
- **Await:** `foo().await`

---

## 17. Unsafe Rust

- **Unsafe Block:** `unsafe { ... }`
- **Unsafe Function:** `unsafe fn dangerous() {}`
- **Unsafe Trait:** `unsafe trait Scary {}`
- **Capabilities:** Dereference raw pointers, call unsafe functions, implement unsafe traits, access mutable statics, access union fields.

---

## 18. Cargo Commands

- `cargo new <name> (--lib)`: Create binary (or library).
- `cargo init`: Initialize existing directory.
- `cargo build (--release)`: Compile debug (or optimized).
- `cargo run`: Compile and execute.
- `cargo check`: fast compile (no code gen).
- `cargo test`: Run unit and integration tests.
- `cargo bench`: Run benchmarks (nightly only generally).
- `cargo doc --open`: Build and view docs.
- `cargo clean`: Remove `target/`.
- `cargo update`: Update dependencies in `Cargo.lock`.
- `cargo install <crate>`: Install binary crate locally.
- `cargo add <crate>`: Add dependency to `Cargo.toml`.
- `cargo clippy`: Run linter.
- `cargo fmt`: Format code.
