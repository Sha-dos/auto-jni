# auto-jni

Automatically generate Rust bindings for Java classes via JNI. Point it at your compiled `.class` files and it produces a type-safe Rust struct with methods that call straight through to Java.

## Setup

The crate has two roles, separated by a feature flag:

| Role | Cargo section | Feature |
|---|---|---|
| Runtime (generated code + macros) | `[dependencies]` | *(none)* |
| Build-time codegen | `[build-dependencies]` | `build` |

```toml
# Cargo.toml
[dependencies]
auto-jni = "0.0.3"

[build-dependencies]
auto-jni = { version = "0.0.3", features = ["build"] }
```

## Usage

**`build.rs`** call `generate_bindings_file` with your class names and classpath:

```rust
use std::{env, path::Path};
use auto_jni::generate_bindings_file;

fn main() {
    let out = env::var("OUT_DIR").unwrap();
    generate_bindings_file(
        vec!["com.example.Car"],
        Some("path/to/classes".into()),
        &Path::new(&out).join("bindings.rs"),
        Some(vec!["-Djava.class.path=path/to/classes".into()]),
    ).unwrap();
}
```

**`src/main.rs`** — include the generated file and use the structs directly:

```rust
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

fn main() {
    let make  = java().new_string("Toyota").unwrap();
    let model = java().new_string("Camry").unwrap();
    let car_type = com_example_Car::com_example_Car_CarType_from_str("SEDAN");

    let car = com_example_Car::new(&make, &model, 2024, &car_type).unwrap();
    car.displayInfo().unwrap();
}
```

## What gets generated

For each class you get:

- A struct named after the fully-qualified class (dots replaced with underscores), e.g. `com_example_Car`
- `fn new(...)` for each constructor
- `fn method_name(&self, ...)` for instance methods
- `fn method_name(...)` for static methods
- A `fn TypeName_from_str(s: &str)` helper for each enum/inner-class argument type
- `fn inner(&self) -> &GlobalRef` to access the raw JNI reference

Method IDs and class references are cached in `OnceCell` statics, so the JNI lookup only happens once per method across all calls.

## Requirements

- A JDK on `PATH` (for `javap` at build time and the JVM at runtime)
- Compiled `.class` files for the Java classes you want to bind

