#[macro_export]
macro_rules! call_static {
    ($path:tt, $method:tt, $sig:tt, $args:expr, $ret:expr) => {
        {
        use auto_jni::once_cell::sync::OnceCell;
        use auto_jni::jni::objects::{JClass, JStaticMethodID};
        use crate::java;
        static FNPTR: OnceCell<JStaticMethodID> = OnceCell::new();
        static CLASS: OnceCell<JClass> = OnceCell::new();
        let mut java = java();
        let fnptr = FNPTR.get_or_init(|| {
            java.get_static_method_id($path, $method, $sig).unwrap()
        });
        let class = CLASS.get_or_init(|| {
            java.find_class($path).unwrap()
        });

        unsafe {
            java.call_static_method_unchecked(class, fnptr, $ret, $args).unwrap()
        }
        }
    };
}


#[macro_export]
macro_rules! call {
    ($obj:expr, $path:tt, $method:tt, $sig:tt, $args:expr, $ret:expr) => {
        {
        use once_cell::sync::OnceCell;
        use jni::objects::{JClass, JMethodID};
        use crate::java;
        static FNPTR: OnceCell<JMethodID> = OnceCell::new();
        let mut java = java();
        let fnptr = FNPTR.get_or_init(|| {
            let class = java.find_class($path).unwrap();
            java.get_method_id(class, $method, $sig).unwrap()
        });

        unsafe {
            java.call_method_unchecked($obj, fnptr, $ret, $args).unwrap()
        }
        }
    };
}

// this one only offers a performance benefit if you construct in a loop,
// the intent is just to homogenize the api
#[macro_export]
macro_rules! create {
    ($path:tt, $sig:tt, $args:expr) => {
        {
        use once_cell::sync::OnceCell;
        use jni::objects::{JClass, JMethodID};
        use crate::java;
        static FNPTR: OnceCell<JMethodID> = OnceCell::new();
        static CLASS: OnceCell<JClass> = OnceCell::new();
        let mut java = java();
        let class = CLASS.get_or_init(|| {
            java.find_class($path).unwrap()
        });
        let fnptr = FNPTR.get_or_init(|| {
            java.get_method_id(class, "<init>", $sig).unwrap()
        });

        let obj = unsafe {
            java.new_object_unchecked(class, *fnptr, $args).unwrap()
        };
        java.new_global_ref(obj).unwrap()
        }
    };
}

#[macro_export]
macro_rules! once {
    ($code:expr) => {
        {
            static ONCE: OnceCell<JObject> = OnceCell::new();

            ONCE.get_or_init(|| {$code})
        }

    };
}

use std::collections::HashMap;
use std::io::Write;
use std::fs::File;
use std::path::Path;
use jni::signature::{Primitive, ReturnType};
use crate::parse_javap_output;

pub fn generate_bindings_file(class_name: Vec<&str>, class_path: Option<String>, output_path: &Path, jvm_options: Option<Vec<String>>) -> std::io::Result<()> {
    let mut file = File::create(output_path)?;

    // Write header imports
    writeln!(file, "use auto_jni::jni::objects::{{JObject, GlobalRef}};")?;
    writeln!(file, "use auto_jni::errors::JNIError;")?;
    writeln!(file, "use auto_jni::{{call, call_static, create}};")?;
    writeln!(file, "use auto_jni::jni::objects::JValue;")?;
    writeln!(file, "use auto_jni::jni::signature::{{Primitive, ReturnType}};")?;
    writeln!(file, "use auto_jni::jni;")?;
    writeln!(file, "use auto_jni::once_cell;")?;
    writeln!(file, "use auto_jni::lazy_static::lazy_static;")?;
    writeln!(file, "use auto_jni::jni::{{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM}};")?;
    writeln!(file, "use auto_jni::jni::objects::JObjectArray;")?;
    writeln!(file)?;

    // Create java functions
    writeln!(file, "lazy_static! {{ static ref JAVA: JavaVM = create_jvm(); }}")?;
    writeln!(file)?;
    writeln!(file, "fn create_jvm() -> JavaVM {{")?;
    writeln!(file, "    let jvm_args = InitArgsBuilder::new()")?;
    writeln!(file, "        .version(JNIVersion::V8)")?;
    if let Some(jvm_options) = jvm_options {
        for option in jvm_options {
            writeln!(file, "        .option(\"{}\")", option.replace("\\", "\\\\"))?;
        }
    }

    writeln!(file, "        .build().unwrap();")?;
    writeln!(file, "    JavaVM::new(jvm_args).unwrap()")?;
    writeln!(file, "}}")?;
    writeln!(file)?;

    writeln!(file, "pub fn java() -> JNIEnv<'static> {{")?;
    writeln!(file, "    JAVA.attach_current_thread_permanently().unwrap()")?;
    writeln!(file, "}}")?;
    writeln!(file)?;

    // Extract struct name from class_name (last part after dot)
    // let struct_name = class_name.split('.').last().unwrap_or(class_name);
    for class in class_name {
        let bindings = parse_javap_output(class, class_path.clone());
        let struct_name = class.replace('.', "_");

        // Write struct definition
        writeln!(file, "#[allow(non_snake_case)]")?;
        writeln!(file, "#[allow(non_camel_case_types)]")?;
        writeln!(file, "pub struct {} {{", struct_name)?;
        writeln!(file, "    inner: GlobalRef,")?;
        writeln!(file, "}}")?;
        writeln!(file)?;

        // Write implementation
        writeln!(file, "#[allow(non_snake_case)]")?;
        writeln!(file, "#[allow(non_camel_case_types)]")?;
        writeln!(file, "impl<'a> {} {{", struct_name)?;

        println!("Length: {}", bindings.len());

        let mut methods: HashMap<String, i32> = HashMap::new();
        let mut enums: Vec<String> = Vec::new();

        // Generate methods for each binding
        for mut binding in bindings {
            println!("Creating binding for: {}", binding.name);

            let mut enums_to_add: Vec<String> = binding.args.iter()
                .filter(|arg| arg.contains('$'))
                .map(|arg| arg.to_string())
                .collect::<Vec<String>>();

            for mut enum_name in enums_to_add {
                enum_name.remove(0);
                if !enums.iter().any(|e| e == &enum_name) {
                    enums.push(enum_name.clone());
                    writeln!(file, "    #[allow(non_snake_case)]")?;
                    writeln!(file, "    pub fn {}_from_str(s: &str) -> JObject {{", enum_name.replace("/", "_").replace("$", "_"))?;
                    writeln!(file, "        call_static!(")?;
                    writeln!(file, "            \"{}\",", enum_name)?;
                    writeln!(file, "            \"valueOf\",")?;
                    writeln!(file, "            \"(Ljava/lang/String;)L{};\",", enum_name)?;
                    writeln!(file, "            &[JValue::Object(&java().new_string(s).unwrap()).as_jni()],")?;
                    writeln!(file, "            ReturnType::Object")?;
                    writeln!(file, "        ).l().unwrap()")?;
                    writeln!(file, "    }}")?;
                }
            }

            // Filter names to remove $ from lambda (lambda$takeSnapshot$1)
            if binding.name.contains('$') {
                let mut split = binding.name.split('$');
                split.next();
                binding.name = split.next().unwrap().to_string();
            }

            // Convert Java types to Rust types for arguments
            let args: Vec<(String, String)> = binding.args.iter().enumerate()
                .map(|(i, arg_type)| {
                    (format!("arg_{}", i), arg_type.to_string())
                })
                .collect();

            // Convert return type
            let return_type = match binding.return_type.as_str() {
                "I" => "i32",
                "J" => "i64",
                "D" => "f64",
                "F" => "f32",
                "Z" => "bool",
                "B" => "i8",
                "C" => "u16",
                "S" => "i16",
                "V" => "()",
                _ => "JObject<'static>"
            };

            let mut method_name = if binding.name.to_ascii_lowercase() == "x" {
                "new".to_string()
            } else {
                binding.name.clone()
            };

            if methods.contains_key(&method_name) {
                methods.insert(method_name.clone(), methods.get(&method_name.clone()).unwrap() + 1);
                method_name.push_str(format!("_{}", &methods.get(&method_name.clone()).unwrap().to_string()).as_str());
            } else {
                methods.insert(method_name.clone(), 1);
            }

            // Write method signature
            writeln!(file, "    #[allow(non_snake_case)]")?;
            write!(file, "    pub fn {}(", method_name)?;

            // Write method body
            if binding.is_constructor {
                // Write arguments
                for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                    write!(file, "{}: {}", arg_name, java_type_to_rust(arg_type))?;
                    if i < args.len() - 1 {
                        write!(file, ", ")?;
                    }
                }
                writeln!(file, ") -> Result<Self, JNIError> {{")?;

                writeln!(file, "        Ok(Self {{")?;
                write!(file, "            inner: create!(\"{}\", \"{}\", &[",
                       binding.path,
                       binding.signature)?;
                for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                    write!(file, "{}", get_input_type(arg_name, arg_type))?;
                    if i < args.len() - 1 {
                        write!(file, ", ")?;
                    }
                }
                writeln!(file, "])")?;
                writeln!(file, "        }})")?;
            } else if binding.is_static {
                for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                    write!(file, "{}: {}", arg_name, java_type_to_rust(arg_type))?;
                    if i < args.len() - 1 {
                        write!(file, ", ")?;
                    }
                }
                writeln!(file, ") -> Result<{}, JNIError> {{", return_type)?;
                let return_type = get_return_type(&*binding.return_type);
                writeln!(file, "        {} call_static!(", if return_type == ReturnType::Primitive(Primitive::Void) {
                    "".to_string()
                } else {
                    "let result =".to_string()
                })?;
                writeln!(file, "            \"{}\",", binding.path)?;
                writeln!(file, "            \"{}\",", binding.name)?;
                writeln!(file, "            \"{}\",", binding.signature)?;
                write!(file, "            &[")?;
                for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                    write!(file, "{}", get_input_type(arg_name, arg_type))?;
                    if i < args.len() - 1 {
                        write!(file, ", ")?;
                    }
                }
                writeln!(file, "],")?;
                writeln!(file, "            {}", convert_return_type_to_string(return_type.clone()))?;
                writeln!(file, "        );")?;
                writeln!(file, "        Ok({})", return_type_to_function(return_type.clone()))?;
            } else {
                write!(file, "instance: &'a GlobalRef, ")?;
                for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                    write!(file, "{}: {}", arg_name, java_type_to_rust(arg_type))?;
                    if i < args.len() - 1 {
                        write!(file, ", ")?;
                    }
                }
                writeln!(file, ") -> Result<{}, JNIError> {{", return_type)?;
                let return_type = get_return_type(&*binding.return_type);
                writeln!(file, "        {} call!(", if return_type == ReturnType::Primitive(Primitive::Void) {
                    "".to_string()
                } else {
                    "let result =".to_string()
                })?;
                writeln!(file, "            instance.as_obj(),")?;
                writeln!(file, "            \"{}\",", binding.path)?;
                writeln!(file, "            \"{}\",", binding.name)?;
                writeln!(file, "            \"{}\",", binding.signature)?;
                write!(file, "            &[")?;
                for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                    write!(file, "{}", get_input_type(arg_name, arg_type))?;
                    if i < args.len() - 1 {
                        write!(file, ", ")?;
                    }
                }
                let return_type = get_return_type(&*binding.return_type);
                writeln!(file, "],")?;
                writeln!(file, "            {}", convert_return_type_to_string(return_type.clone()))?;
                writeln!(file, "        );")?;
                writeln!(file, "        Ok({})", return_type_to_function(return_type.clone()))?;
            }
            writeln!(file, "    }}")?;
        }

        // Expose inner
        writeln!(file, "    pub fn inner(&self) -> GlobalRef {{")?;
        writeln!(file, "        self.inner.clone()")?;
        writeln!(file, "    }}")?;

        writeln!(file, "}}")?;
        writeln!(file)?;
    }

    Ok(())
}

/// Convert java type to rust type
fn java_type_to_rust(java_type: &str) -> &str {
    match java_type {
        "I" => "i32",
        "J" => "i64",
        "D" => "f64",
        "F" => "f32",
        "Z" => "bool",
        "B" => "i8",
        "C" => "u16",
        "S" => "i16",
        "V" => "()",
        t if t.starts_with("L") => "&JObject",
        t if t.starts_with("[") => "&JObjectArray",
        _ => "JObject"
    }
}

/// Convert return type to function to get type
/// ex. ReturnType::Primitive(Primitive::Int) => ".i().unwrap()"
fn return_type_to_function(return_type: ReturnType) -> String {
    match return_type {
        ReturnType::Primitive(Primitive::Int) => "result.i().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Long) => "result.j().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Double) => "result.d().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Float) => "result.f().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Boolean) => "result.z().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Byte) => "result.b().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Char) => "result.c().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Short) => "result.s().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Void) => "()".to_string(),
        ReturnType::Object => "result.l().unwrap()".to_string(),
        _ => "".to_string()
    }
}

/// Get the input types from a string
fn get_input_type(arg_name: &str, arg_type: &str) -> String {
    match arg_type {
        "I" => format!("JValue::Int({}).as_jni()", arg_name),
        "J" => format!("JValue::Long({}).as_jni()", arg_name),
        "D" => format!("JValue::Double({}).as_jni()", arg_name),
        "F" => format!("JValue::Float({}).as_jni()", arg_name),
        "Z" => format!("JValue::Bool({} as u8).as_jni()", arg_name),
        "B" => format!("JValue::Byte({}).as_jni()", arg_name),
        "C" => format!("JValue::Char({}).as_jni()", arg_name),
        "S" => format!("JValue::Short({}).as_jni()", arg_name),
        t => format!("JValue::Object({}).as_jni()", arg_name),
        _ => arg_type.to_string()
    }
}

/// Convert string return type to ReturnType enum
fn get_return_type(return_type: &str) -> ReturnType {
    match return_type {
        "I" => ReturnType::Primitive(Primitive::Int),
        "J" => ReturnType::Primitive(Primitive::Long),
        "D" => ReturnType::Primitive(Primitive::Double),
        "F" => ReturnType::Primitive(Primitive::Float),
        "Z" => ReturnType::Primitive(Primitive::Boolean),
        "B" => ReturnType::Primitive(Primitive::Byte),
        "C" => ReturnType::Primitive(Primitive::Char),
        "S" => ReturnType::Primitive(Primitive::Short),
        "V" => ReturnType::Primitive(Primitive::Void),
        t if t.starts_with("L") => ReturnType::Object,
        t if t.starts_with("[") => ReturnType::Object,
        _ => ReturnType::Object
    }
}

/// Convert ReturnType to string to be added to file
fn convert_return_type_to_string(return_type: ReturnType) -> String {
    match return_type {
        ReturnType::Primitive(Primitive::Int) => "ReturnType::Primitive(Primitive::Int)".to_string(),
        ReturnType::Primitive(Primitive::Long) => "ReturnType::Primitive(Primitive::Long)".to_string(),
        ReturnType::Primitive(Primitive::Double) => "ReturnType::Primitive(Primitive::Double)".to_string(),
        ReturnType::Primitive(Primitive::Float) => "ReturnType::Primitive(Primitive::Float)".to_string(),
        ReturnType::Primitive(Primitive::Boolean) => "ReturnType::Primitive(Primitive::Boolean)".to_string(),
        ReturnType::Primitive(Primitive::Byte) => "ReturnType::Primitive(Primitive::Byte)".to_string(),
        ReturnType::Primitive(Primitive::Char) => "ReturnType::Primitive(Primitive::Char)".to_string(),
        ReturnType::Primitive(Primitive::Short) => "ReturnType::Primitive(Primitive::Short)".to_string(),
        ReturnType::Primitive(Primitive::Void) => "ReturnType::Primitive(Primitive::Void)".to_string(),
        ReturnType::Object => "ReturnType::Object".to_string(),
        _ => "".to_string()
    }
}

pub use {call, create, call_static, once};