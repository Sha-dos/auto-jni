use std::env;
use std::path::Path;
use auto_jni::call::generate_bindings_file;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out = env::var("OUT_DIR").unwrap();
    let file = Path::new(&out).join("bindings.rs");
    let class_name = vec!["com.example.Calculator", "com.example.DataHolder"];
    let class_path = Some("E:\\auto-jni\\examples\\calculator\\classes".to_string());

    let options = vec!["-Xcheck:jni".to_string(), "-Djava.class.path=E:\\auto-jni\\examples\\calculator\\classes".to_string()];

    generate_bindings_file(class_name, class_path, &*file, Some(options)).expect("TODO: panic message");
    // generate_bindings_file(class_name, class_path, Path::new("E:\\auto-jni\\examples\\calculator\\bindings.rs"), Some(options)).expect("TODO: panic message");
}