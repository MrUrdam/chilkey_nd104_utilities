use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolError {
    /// Payload does not fit into a 32-byte frame.
    PayloadTooLong(usize),
    /// Temperature is NaN or outside the encodable range.
    TemperatureOutOfRange(f32),
    /// Icon index outside 0–9.
    InvalidIcon(u8),
    /// Date/time field outside its valid range.
    InvalidTime(&'static str),
    /// Theme id outside 1–3.
    InvalidTheme(u8),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::PayloadTooLong(len) => {
                write!(
                    f,
                    "payload of {len} bytes does not fit into a frame (max 19)"
                )
            }
            ProtocolError::TemperatureOutOfRange(t) => {
                write!(
                    f,
                    "temperature {t} °C cannot be encoded (allowed: -99.9 to 99.9)"
                )
            }
            ProtocolError::InvalidIcon(i) => {
                write!(f, "weather icon {i} is invalid (allowed: 0–9)")
            }
            ProtocolError::InvalidTime(field) => write!(f, "invalid time field: {field}"),
            ProtocolError::InvalidTheme(id) => write!(f, "theme {id} is invalid (allowed: 1–3)"),
        }
    }
}

impl std::error::Error for ProtocolError {}
