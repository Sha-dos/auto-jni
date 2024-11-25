pub mod call;
pub mod errors;

pub use jni;
pub use once_cell;

use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jni::objects::JObject;
use jni::signature::ReturnType;
use lazy_static::lazy_static;
use regex::Regex;

fn create_jvm() -> JavaVM {
    /*let jvm_args = InitArgsBuilder::new()
        .version(JNIVersion::V8)
        .option("-XX:+UseSerialGC")
        .option("-Djava.lang.invoke.stringConcat=BC_SB")
        // .option("-Djava.library.path=/usr/local/frc/third-party/lib")
        .option("-Djava.library.path=C:\\Program Files\\Microsoft\\jdk-11.0.16.101-hotspot\\bin")
        // .option("-Djava.class.path=/home/lvuser/javastub.jar")
        .build().unwrap();

    let jvm = JavaVM::with_libjvm(jvm_args, || Ok("/usr/local/frc/JRE/lib/client/libjvm.so")).unwrap();
    jvm.attach_current_thread_as_daemon().unwrap();
    jvm*/

    let jvm_args = InitArgsBuilder::new()
        .version(JNIVersion::V8)
        .option("-Xcheck:jni")
        .option("-Djava.class.path=E:\\auto-jni\\test\\target\\classes")
        .build()
        .unwrap();

    JavaVM::new(jvm_args).unwrap()
}

lazy_static! {
     static ref JAVA: JavaVM = create_jvm();
}

pub fn java() -> JNIEnv<'static> {
    JAVA.attach_current_thread_permanently().unwrap()
}

#[derive(Debug, PartialEq)]
struct MethodBinding {
    path: String,
    name: String,
    signature: String,
    args: Vec<String>,
    return_type: String,
}

fn parse_javap_output(class_name: &str, class_path: Option<&str>) -> Vec<MethodBinding> {
    let mut command = Command::new("javap");
    command.args(["-s", "-p"]);

    if let Some(cp) = class_path {
        command.arg("-classpath").arg(cp);
    }

    command.arg(class_name);

    let output = command.output().expect("Failed to execute javap");
    let output_str = String::from_utf8_lossy(&output.stdout);

    // Example line: "  public int add(int, int);"
    // Descriptor line: "    descriptor: (II)I"
    let method_regex = Regex::new(r"(?m)^\s*(?:public|private|protected)?\s*(?:static)?\s*(\S+)\s+(\w+)\s*\((.*?)\);").unwrap();
    let descriptor_regex = Regex::new(r"^\s*descriptor:\s*([^;\n]+)").unwrap();

    let mut bindings = Vec::new();
    let mut lines = output_str.lines().peekable();

    while let Some(line) = lines.next() {
        if let Some(captures) = method_regex.captures(line) {
            let return_type = captures.get(1).map_or("", |m| m.as_str()).to_string();
            let name = captures.get(2).map_or("", |m| m.as_str()).to_string();
            let args_str = captures.get(3).map_or("", |m| m.as_str());

            while let Some(next_line) = lines.peek() {
                if let Some(desc_captures) = descriptor_regex.captures(next_line) {
                    let signature = desc_captures.get(1).map_or("", |m| m.as_str()).to_string();

                    let args = parse_descriptor_args(&signature);
                    let return_type = parse_descriptor_return(&signature);

                    bindings.push(MethodBinding {
                        path: class_name.replace('.', "/"),
                        name,
                        signature,
                        args,
                        return_type,
                    });
                    break;
                }
                lines.next();
            }
        }
    }

    bindings
}

fn parse_descriptor_args(descriptor: &str) -> Vec<String> {
    let args_section = descriptor
        .trim_start_matches('(')
        .split(')')
        .next()
        .unwrap_or("");

    let mut args = Vec::new();
    let mut chars = args_section.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            'L' => {
                let mut class_name = String::new();
                while let Some(nc) = chars.next() {
                    if nc == ';' { break; }
                    class_name.push(nc);
                }
                args.push(class_name);
            },
            'I' | 'J' | 'D' | 'F' | 'B' | 'C' | 'S' | 'Z' => args.push(c.to_string()),
            '[' => {
                let mut array_type = String::from("[");
                if let Some(next_char) = chars.next() {
                    array_type.push(next_char);
                    if next_char == 'L' {
                        while let Some(nc) = chars.next() {
                            array_type.push(nc);
                            if nc == ';' { break; }
                        }
                    }
                }
                args.push(array_type);
            },
            _ => continue,
        }
    }

    args
}

fn parse_descriptor_return(descriptor: &str) -> String {
    let return_type = descriptor.split(')').nth(1).unwrap_or("");
    return_type.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_javap() {
        let class_name = "com.example.Calculator";
        let class_path = Some("test/target/classes");
        let bindings = parse_javap_output(class_name, class_path);

        assert!(!bindings.is_empty(), "No bindings were parsed");

        let add_method = bindings.iter().find(|b| b.name == "add")
            .expect("Could not find add method");

        assert_eq!(add_method.path, "com/example/Calculator");
        assert_eq!(add_method.name, "add");
        assert_eq!(add_method.signature, "(II)I");
        assert_eq!(add_method.args, vec!["I", "I"]);
        assert_eq!(add_method.return_type, "I");
    }

    #[test]
    fn test_parse_descriptor() {
        assert_eq!(
            parse_descriptor_args("(II)I"),
            vec!["I", "I"]
        );
        assert_eq!(
            parse_descriptor_args("(ILjava/lang/String;[I)V"),
            vec!["I", "java/lang/String", "[I"]
        );
        assert_eq!(
            parse_descriptor_return("(II)I"),
            "I"
        );
    }
}