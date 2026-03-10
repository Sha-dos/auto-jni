use std::env;
use std::path::Path;
use auto_jni::generate_bindings_file;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out = env::var("OUT_DIR").unwrap();
    let file = Path::new(&out).join("bindings.rs");

    generate_bindings_file(
        vec!["com.example.Car"],
        Some("../java/src".to_string()),
        &file,
        Some(vec!["-Djava.class.path=../java/src".to_string()]),
    ).expect("Failed to generate bindings");
}