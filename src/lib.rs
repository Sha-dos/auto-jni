pub mod call;
pub mod errors;

pub use jni;
pub use once_cell;
pub use lazy_static;

use std::process::Command;
use regex::Regex;

#[derive(Debug, PartialEq)]
struct MethodBinding {
    path: String,
    name: String,
    signature: String,
    args: Vec<String>,
    return_type: String,
    is_static: bool
}

fn parse_javap_output(class_name: &str, class_path: Option<String>) -> Vec<MethodBinding> {
    let mut command = Command::new("javap");
    command.args(["-s", "-p"]);

    if let Some(cp) = class_path {
        command.arg("-classpath").arg(cp);
    }

    command.arg(class_name);

    let output = command.output().expect("Failed to execute javap");
    let output_str = String::from_utf8_lossy(&output.stdout);

    let method_regex = Regex::new(r"(?m)^\s*(?:public|private|protected)?\s*(static\s+native|native\s+static|static|native)?\s*([\w<>.\[\]]+)\s+([\w$]+)\s*\(([^)]*)\)\s*(?:throws\s+[\w.]+)?\s*;").unwrap();
    let descriptor_regex = Regex::new(r"^\s*descriptor:\s*(.+)$").unwrap();

    let mut bindings = Vec::new();
    let mut lines = output_str.lines().peekable();

    while let Some(line) = lines.next() {
        if let Some(captures) = method_regex.captures(line) {
            let is_static = captures.get(1).is_some();
            let return_type = captures.get(2).map_or("", |m| m.as_str()).to_string();
            let name = captures.get(3).map_or("", |m| m.as_str()).to_string();
            let args_str = captures.get(4).map_or("", |m| m.as_str());

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
                        is_static,
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

                args.push(format!("L{}", class_name));
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
    descriptor.split(')').nth(1).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_javap() {
        let class_name = "com.example.EnumTest";
        let class_path = Some("examples/create_enum/classes".to_string());
        let bindings = parse_javap_output(class_name, class_path);

        assert!(!bindings.is_empty(), "No bindings were parsed");

        let add_method = bindings.iter().find(|b| b.name == "check")
            .expect("Could not find check method");

        assert_eq!(add_method.path, "com/example/EnumTest");
        assert_eq!(add_method.name, "check");
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
        assert_eq!(
            parse_descriptor_args("(Lcom/example/EnumTest$CountEnum;)I"),
            vec!["Lcom/example/EnumTest$CountEnum;"]
        );
        assert_eq!(
            parse_descriptor_return("(Lcom/example/EnumTest$CountEnum;)I"),
            "I"
        )
    }
}