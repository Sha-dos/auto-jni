use std::env;
use std::path::Path;
use auto_jni::call::generate_bindings_file;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out = env::var("OUT_DIR").unwrap();
    let file = Path::new(&out).join("bindings.rs");
    let class_name = "com.example.Calculator";
    let class_path = Some("E:\\auto-jni\\test\\target\\classes");

    generate_bindings_file(class_name, class_path, &*file).expect("TODO: panic message");
    // generate_bindings_file(class_name, class_path, Path::new("E:\\auto-jni\\examples\\calculator\\bindings.rs")).expect("TODO: panic message");
}