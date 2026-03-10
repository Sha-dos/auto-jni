# Auto JNI
## Automatically create bindings to Java through JNI

This was created to simplify created bindings for [frcrs](https://github.com/Team-2502/frcrs) and to make it easier to create bindings for other projects.

### Auto JNI is a heavy work in progress and their are many features still being implemented.
- [x] Initialize Classes
- [x] Call Methods (static and instance)
- [x] Create enums
- [ ] Improve API
- [ ] Add more examples
- [ ] Add more documentation
- [ ] Add more tests
- [ ] Add more error handling
- [ ] Add more logging

### Example
Example.java
```java
package com.example;
class Example {
    public static int add(int a, int b) {
        return a + b;
    }
}
```
build.rs
```rust
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out = env::var("OUT_DIR").unwrap();
    let file = Path::new(&out).join("bindings.rs");
    let class_name = vec![
        "com.example.Example"
    ];
    let class_path = Some("build".to_string());

    let options = vec![
        "-Djava.class.path=build".to_string(),
    ];

    generate_bindings_file(class_name, class_path, &*file, Some(options)).expect("Failed to generate bindings");
}
```