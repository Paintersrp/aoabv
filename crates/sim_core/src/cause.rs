use serde::Serialize;

/// Structured cause entry used for diagnostics and auditing.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Cause {
    pub target: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Cause {
    pub fn new<T: Into<String>, C: Into<String>>(target: T, code: C, note: Option<String>) -> Self {
        Self {
            target: target.into(),
            code: code.into(),
            note,
        }
    }
}
