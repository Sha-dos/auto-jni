#[macro_export]
macro_rules! call_static {
    ($path:expr, $method:expr, $sig:expr, $args:expr, $ret:ty) => {{
        static FNPTR: OnceCell<jni::objects::JStaticMethodID> = OnceCell::new();
        static CLASS: OnceCell<jni::objects::JClass<'static>> = OnceCell::new();

        let env = get_env()?;
        let class = CLASS.get_or_init(|| {
            env.find_class($path)
                .map_err(|e| JniError::Jni(e))
                .expect("Failed to find class")
        });

        let method_id = FNPTR.get_or_init(|| {
            env.get_static_method_id(class, $method, $sig)
                .map_err(|e| JniError::MethodNotFound(format!("{}::{} - {}", $path, $method, e)))
                .expect("Failed to get method ID")
        });

        let result = unsafe {
            env.call_static_method_unchecked(
                class,
                method_id,
                ReturnType::Primitive(<$ret as IntoJavaType>::into_type()),
                $args,
            )?
        };

        Ok(<$ret as FromJValue>::from_jvalue(result))
    }};
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

#[macro_export]
macro_rules! generate_bindings {
    ($class_name:expr, $class_path:expr) => {
        use jni::objects::{JObject, GlobalRef};
        use jni::sys::*;
        use once_cell::sync::OnceCell;
        use crate::errors::JniError;
        use crate::{call, call_static, create, once};
        use jni::objects::JValue;
        use jni::signature::{Primitive, ReturnType};

        pub struct $class_name {
            inner: GlobalRef,
        }

        impl $class_name {
            fn parse_bindings() -> Vec<MethodBinding> {
                parse_javap_output($class_name, Some($class_path))
            }

            pub fn new() -> Result<Self, JniError> {
                Ok(Self {
                    inner: create!($class_name, "()V", &[])
                })
            }

            $(
                // Generate method for each binding
                pub fn $binding.name(&self $(, $arg_name: $arg_type)*) -> Result<$return_type, JniError> {
                    if binding.is_static {
                        call_static!(
                            &binding.path,
                            &binding.name,
                            &binding.signature,
                            &[$(JValue::from($arg_name)),*],
                            $return_type
                        )
                    } else {
                        let result = call!(
                            &self.inner,
                            &binding.path,
                            &binding.name,
                            &binding.signature,
                            &[$(JValue::from($arg_name)),*],
                            ReturnType::from(binding.return_type.clone())
                        );
                        Ok(<$return_type>::from_jvalue(result)?)
                    }
                }
            )*
        }

        // Generate constructor variants based on binding patterns
        impl $class_name {
            $(
                pub fn $ctor_name($($arg_name: $arg_type),*) -> Result<Self, JniError> {
                    Ok(Self {
                        inner: create!(
                            &binding.path,
                            &binding.signature,
                            &[$(JValue::from($arg_name)),*]
                        )
                    })
                }
            )*
        }

        // Implement conversion traits
        impl From<JObject<'_>> for $class_name {
            fn from(obj: JObject) -> Self {
                let java = crate::java();
                Self {
                    inner: java.new_global_ref(obj).unwrap()
                }
            }
        }

        impl AsRef<JObject<'_>> for $class_name {
            fn as_ref(&self) -> &JObject {
                self.inner.as_obj()
            }
        }
    };
}

use std::io::Write;

pub fn generate_bindings_file(class_name: &str, class_path: Option<&str>, output_path: &Path) -> std::io::Result<()> {
    let bindings = parse_javap_output(class_name, class_path);
    let mut file = File::create(output_path)?;

    // Write header imports
    writeln!(file, "use auto_jni::jni::objects::{{JObject, GlobalRef}};")?;
    writeln!(file, "use auto_jni::jni::sys::*;")?;
    writeln!(file, "use auto_jni::once_cell::sync::OnceCell;")?;
    writeln!(file, "use auto_jni::errors::JNIError;")?;
    writeln!(file, "use auto_jni::{{call, call_static, create, once}};")?;
    writeln!(file, "use auto_jni::jni::objects::JValue;")?;
    writeln!(file, "use auto_jni::jni::signature::{{Primitive, ReturnType}};")?;
    writeln!(file, "use auto_jni::jni;")?;
    writeln!(file, "use auto_jni::once_cell;")?;
    writeln!(file, "use auto_jni::java;")?;
    writeln!(file)?;

    // Extract struct name from class_name (last part after dot)
    // let struct_name = class_name.split('.').last().unwrap_or(class_name);
    let struct_name = class_name.replace('.', "_");

    // Write struct definition
    writeln!(file, "pub struct {} {{", struct_name)?;
    writeln!(file, "    inner: GlobalRef,")?;
    writeln!(file, "}}")?;
    writeln!(file)?;

    // Write implementation
    writeln!(file, "impl<'a> {} {{", struct_name)?;

    // Write default constructor
    writeln!(file, "    pub fn new() -> Result<Self, JNIError> {{")?;
    writeln!(file, "        Ok(Self {{")?;
    writeln!(file, "            inner: create!(\"{}\", \"()V\", &[])", class_name.replace('.', "/"))?;
    writeln!(file, "        }})")?;
    writeln!(file, "    }}")?;

    println!("Length: {}", bindings.len());
    // Generate methods for each binding
    for binding in bindings {
        println!("Creating binding for: {}", binding.name);
        writeln!(file)?;  // Add spacing between methods

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

        let method_name = if binding.name == "<init>" {
            "new".to_string()
        } else {
            binding.name.clone()
        };

        // Write method signature
        write!(file, "    pub fn {}(", method_name)?;

        // Add self parameter for instance methods
        // if method_name != "new" {
        //     write!(file, "&self")?;
        //     if !args.is_empty() {
        //         write!(file, ", ")?;
        //     }
        // }

        // Write method body
        if method_name == "new" {
            // Write arguments
            for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                write!(file, "{}: {}", arg_name, java_type_to_rust(arg_type))?;
                if i < args.len() - 1 {
                    write!(file, ", ")?;
                }
            }
            writeln!(file, ") -> Result<{}, JNIError> {{", return_type)?;

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
        } else {
            // Write arguments
            write!(file, "instance: &'a GlobalRef, ")?;
            for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                write!(file, "{}: {}", arg_name, java_type_to_rust(arg_type))?;
                if i < args.len() - 1 {
                    write!(file, ", ")?;
                }
            }
            writeln!(file, ") -> Result<{}, JNIError> {{", return_type)?;
            writeln!(file, "        let result = call!(")?;
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
            writeln!(file, "        Ok(result{})", return_type_to_function(return_type.clone()))?;
        }
        writeln!(file, "    }}")?;
    }

    writeln!(file, "}}")?;
    writeln!(file)?;

    // Write conversion implementations
    // writeln!(file, "impl From<JObject<'_>> for {} {{", struct_name)?;
    // writeln!(file, "    fn from(obj: JObject) -> Self {{")?;
    // writeln!(file, "        let java = auto_jni::java();")?;
    // writeln!(file, "        Self {{")?;
    // writeln!(file, "            inner: java.new_global_ref(obj).unwrap()")?;
    // writeln!(file, "        }}")?;
    // writeln!(file, "    }}")?;
    // writeln!(file, "}}")?;
    // writeln!(file)?;
    //
    // writeln!(file, "impl AsRef<JObject<'_>> for {} {{", struct_name)?;
    // writeln!(file, "    fn as_ref(&self) -> &JObject {{")?;
    // writeln!(file, "        self.inner.as_obj()")?;
    // writeln!(file, "    }}")?;
    // writeln!(file, "}}")?;

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
        t if t.starts_with("L") => "JObject",
        t if t.starts_with("[") => "JObjectArray",
        _ => "JObject"
    }
}

/// Convert return type to function to get type
/// ex. ReturnType::Primitive(Primitive::Int) => ".i().unwrap()"
fn return_type_to_function(return_type: ReturnType) -> String {
    match return_type {
        ReturnType::Primitive(Primitive::Int) => ".i().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Long) => ".j().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Double) => ".d().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Float) => ".f().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Boolean) => ".z().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Byte) => ".b().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Char) => ".c().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Short) => ".s().unwrap()".to_string(),
        ReturnType::Primitive(Primitive::Void) => "".to_string(),
        ReturnType::Object => ".l().unwrap()".to_string(),
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
        "Z" => format!("JValue::Boolean({}).as_jni()", arg_name),
        "B" => format!("JValue::Byte({}).as_jni()", arg_name),
        "C" => format!("JValue::Char({}).as_jni()", arg_name),
        "S" => format!("JValue::Short({}).as_jni()", arg_name),
        t => format!("JValue::Object(&{}).as_jni()", arg_name),
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

use std::fs::File;
use std::path::Path;
use jni::objects::JValue;
use jni::signature::{Primitive, ReturnType};
pub use {call, create, call_static, once, generate_bindings};
use crate::parse_javap_output;