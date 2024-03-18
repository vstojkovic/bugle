use std::fmt::Display;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    cause: Option<Box<dyn std::error::Error>>,
}

#[derive(Debug)]
pub enum ErrorKind {
    InvalidType,
    InvalidValue,
    Custom,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ErrorKind::InvalidType => write!(f, "invalid type: {}", &self.message),
            ErrorKind::InvalidValue => write!(f, "invalid value: {}", &self.message),
            ErrorKind::Custom => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_deref()
    }
}

impl Error {
    pub fn invalid_type<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorKind::InvalidType, message.into())
    }

    pub fn invalid_value<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorKind::InvalidValue, message.into())
    }

    pub fn custom<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorKind::Custom, message.into())
    }

    pub fn with_cause<C: std::error::Error + 'static>(mut self, cause: C) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    fn new(kind: ErrorKind, message: String) -> Self {
        Self {
            kind,
            message,
            cause: None,
        }
    }
}
