pub fn progress(message: impl AsRef<str>) {
    println!("==> {}", message.as_ref());
}

#[allow(dead_code)]
pub fn warn(message: impl AsRef<str>) {
    eprintln!("warn: {}", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    eprintln!("error: {}", message.as_ref());
}
