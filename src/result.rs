use std::error::Error as StdError;
use std::fmt;

pub struct Error {
    inner: InnerError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Error").field(&self.inner).finish()
    }
}

impl fmt::Display for InnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InnerError::ByMessage(m) => f.write_str(m),
        }
    }
}

impl StdError for Error {}

pub struct AlreadyExists<T>(pub T);
pub struct NotFound<T>(pub T);

#[non_exhaustive]
#[derive(Debug)]
pub(crate) enum InnerError {
    ByMessage(String),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        use InnerError::*;
        match (&self.inner, &other.inner) {
            (ByMessage(x), ByMessage(y)) if x == y => true,
            _ => false,
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.inner {
            InnerError::ByMessage(msg) => f.write_str(&msg),
        }
    }
}

impl Error {
    fn new(inner: InnerError) -> Self {
        Error { inner }
    }

    pub fn from_text(text: String) -> Self {
        Self::new(InnerError::ByMessage(text))
    }
}

impl<T: fmt::Debug> From<AlreadyExists<T>> for Error {
    fn from(value: AlreadyExists<T>) -> Self {
        Self::from_text(format!("{0:#?}", value))
    }
}

impl<T: fmt::Debug> From<NotFound<T>> for Error {
    fn from(value: NotFound<T>) -> Self {
        Self::from_text(format!("{0:#?}", value))
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::from_text(value)
    }
}

impl<T: fmt::Debug> fmt::Debug for AlreadyExists<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Already exists: ")?;
        self.0.fmt(f)
    }
}

impl<T: fmt::Debug> fmt::Debug for NotFound<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Not found: ")?;
        self.0.fmt(f)
    }
}

pub type Result<S> = std::result::Result<S, Error>;

pub type CreationResult<T, S = ()> = std::result::Result<S, AlreadyExists<T>>;
pub type UpdateResult<T, S = ()> = std::result::Result<S, NotFound<T>>;
