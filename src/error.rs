use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    NoDevice,
    UnsupportedDevice,
    AccessDenied,
    DeviceBusy,
    InvalidCommand,
    InvalidReportLength { expected: usize, actual: usize },
    InvalidChecksum,
    UnexpectedResponse,
    HidTransport,
}

impl AppError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::NoDevice => "No supported Razer mouse found.",
            Self::UnsupportedDevice => "Unsupported Razer device.",
            Self::AccessDenied => "Device access denied.",
            Self::DeviceBusy => "Device busy. Will retry.",
            Self::InvalidCommand => "Unknown command.",
            Self::InvalidReportLength { .. } | Self::InvalidChecksum | Self::UnexpectedResponse => {
                "Battery query failed."
            }
            Self::HidTransport => "HID transport failed.",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReportLength { expected, actual } => {
                write!(
                    formatter,
                    "invalid report length: expected {expected}, got {actual}"
                )
            }
            _ => formatter.write_str(self.user_message()),
        }
    }
}

impl std::error::Error for AppError {}
