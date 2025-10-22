#[macro_export]
macro_rules! info {
    (%$field:ident, $msg:literal $(,)?) => {{
        println!("[info] {} {}", $msg, $field);
    }};
    ($msg:literal, %$field:ident $(,)?) => {{
        println!("[info] {} {}", $msg, $field);
    }};
    ($msg:literal $(,)?) => {{
        println!("[info] {}", $msg);
    }};
}
