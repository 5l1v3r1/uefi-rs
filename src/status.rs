use super::Result;
use core::{fmt, ops};

const HIGHEST_BIT_SET: usize = !((!0_usize) >> 1);

/// Status codes are returned by UEFI interfaces
/// to indicate whether an operation completed successfully.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(usize)]
pub enum Status {
    /// The operation completed successfully.
    Success,
    /// The string contained characters that the device could not render and were skipped.
    WarnUnknownGlyph,
    /// The handle was closed, but the file was not deleted.
    WarnDeleteFailure,
    /// The handle was closed, but the data to the file was not flushed properly.
    WarnWriteFailure,
    /// The resulting buffer was too small,
    /// and the data was truncated to the buffer size.
    WarnBufferTooSmall,
    /// The data has not been updated within the timeframe
    /// set by local policy for this type of data.
    WarnStaleData,
    /// The resulting buffer contains UEFI-compliant file system.
    WarnFileSystem,
    /// The operation will be processed across a system reset.
    WarnResetRequired,
    /// The image failed to load.
    LoadError = 1 | HIGHEST_BIT_SET,
    /// A parameter was incorrect.
    InvalidParameter,
    /// The operation is not supported.
    Unsupported,
    /// The buffer was not the proper size for the request.
    BadBufferSize,
    /// The buffer is not large enough to hold the requested data.
    /// The required buffer size is returned in the appropriate parameter.
    BufferTooSmall,
    /// There is no data pending upon return.
    NotReady,
    /// The physical device reported an error while attempting the operation.
    DeviceError,
    /// The device cannot be written to.
    WriteProtected,
    /// A resource has run out.
    OutOfResources,
    /// An inconstancy was detected on the file system causing the operating to fail.
    VolumeCorrupted,
    /// There is no more space on the file system.
    VolumeFull,
    /// The device does not contain any medium to perform the operation.
    NoMedia,
    /// The medium in the device has changed since the last access.
    MediaChanged,
    /// The item was not found.
    NotFound,
    /// Access was denied.
    AccessDenied,
    /// The server was not found or did not respond to the request.
    NoResponse,
    /// A mapping to a device does not exist.
    NoMapping,
    /// The timeout time expired.
    Timeout,
    /// The protocol has not been started.
    NotStarted,
    /// The protocol has already been started.
    AlreadyStarted,
    /// The operation was aborted.
    Aborted,
    /// An ICMP error occurred during the network operation.
    IcmpError,
    /// A TFTP error occurred during the network operation.
    TftpError,
    /// A protocol error occurred during the network operation.
    ProtocolError,
    /// The function encountered an internal version that was
    /// incompatible with a version requested by the caller.
    IncompatibleVersion,
    /// The function was not performed due to a security violation.
    SecurityViolation,
    /// A CRC error was detected.
    CrcError,
    /// Beginning or end of media was reached
    EndOfMedia,
    /// The end of the file was reached.
    EndOfFile = 31 | HIGHEST_BIT_SET,
    /// The language specified was invalid.
    InvalidLanguage,
    /// The security status of the data is unknown or compromised and
    /// the data must be updated or replaced to restore a valid security status.
    CompromisedData,
    /// There is an address conflict address allocation
    IpAddressConflict,
    /// A HTTP error occurred during the network operation.
    HttpError,
}

impl Status {
    /// Returns true if status code indicates success.
    #[inline]
    pub fn is_success(self) -> bool {
        self == Status::Success
    }

    /// Returns true if status code indicates a warning.
    #[inline]
    pub fn is_warning(self) -> bool {
        (self as usize) & HIGHEST_BIT_SET == 0
    }

    /// Returns true if the status code indicates an error.
    #[inline]
    pub fn is_error(self) -> bool {
        (self as usize) & HIGHEST_BIT_SET == 1
    }
}

impl Into<Result<()>> for Status {
    /// Converts this status code to `Ok` if it indicates success,
    /// or to `Err` if it indicates an error or warning.
    #[inline]
    fn into(self) -> Result<()> {
        if self.is_success() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

impl From<fmt::Error> for Status {
    fn from(_: fmt::Error) -> Self {
        // This is pretty much the only cause why console I/O would fail.
        Status::WarnUnknownGlyph
    }
}

impl ops::Try for Status {
    type Ok = ();
    type Error = Status;

    fn into_result(self) -> Result<()> {
        self.into()
    }

    fn from_error(error: Self::Error) -> Self {
        error
    }

    fn from_ok(_: Self::Ok) -> Self {
        Status::Success
    }
}
