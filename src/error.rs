use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum RadxError {
    IoError(io::Error),
    BadAhxFrameHeader,
    BadAdxHeader(&'static str),
}

impl fmt::Display for RadxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RadxError::IoError(ref err) => write!(f, "IO Error: {}", err),
            RadxError::BadAhxFrameHeader => write!(f, "bad ahx frame header"),
            RadxError::BadAdxHeader(reason) => write!(f, "bad adx header: {}", reason),
        }
    }
}

impl Error for RadxError {
    fn description(&self) -> &str {
        match *self {
            RadxError::IoError(ref err) => err.description(),
            RadxError::BadAhxFrameHeader => "bad ahx frame header",
            RadxError::BadAdxHeader(reason) => reason,
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            RadxError::IoError(ref err) => Some(err),
            RadxError::BadAhxFrameHeader => None,
            RadxError::BadAdxHeader(_) => None,
        }
    }
}

impl From<io::Error> for RadxError {
    fn from(err: io::Error) -> Self {
        RadxError::IoError(err)
    }
}

pub type RadxResult<T> = Result<T, RadxError>;
