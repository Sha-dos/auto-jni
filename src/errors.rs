#[derive(Debug)]
pub enum JNIError {
    NullPtr,
    InvalidArg,
    Unknown,
    NoClass,
    NoMethod,
    NoField,
    NoMemory,
    ClassType,
    ThreadDetached,
    AttachFailed,
    UnknownError,
}