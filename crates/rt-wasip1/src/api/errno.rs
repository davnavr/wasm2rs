macro_rules! errno {
    ($(
        $case:ident$(($num:literal))? = $message:literal,
    )*) => {
        /// Error code returned by functions in the WASI [`Api`].
        ///
        /// [`Api`]: crate::api::Api
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
        #[repr(u16)]
        #[non_exhaustive]
        pub enum Errno {
            $(
                #[doc = $message]
                #[allow(non_camel_case_types)]
                $case $(= $num)?
            ),*
        }

        impl Errno {
            /// Attempts to convert the given integer value into an [`Errno`].
            ///
            /// Returns `Some(Err)` if a matching [`Errno`] was found, or `Some(Ok(()))` if the
            /// integer value was `0`, indicating a success.
            ///
            /// # Errors
            ///
            /// Returns `None` if the integer does not correspond to a known [`Errno`] value.
            pub const fn try_from_raw(raw: u16) -> Option<Result<(), Self>> {
                if raw == 0 {
                    Some(Ok(()))
                }

                $(
                    // Can't match against non-existant `$num` case.
                    else if raw == Self::$case as u16 {
                        Some(Err(Self::$case))
                    }
                )*

                else {
                    None
                }
            }
        }

        impl core::fmt::Display for Errno {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                if f.alternate() {
                    write!(f, "[{}]: ", *self as u16)?;
                }

                f.write_str(match self {
                    $(Self::$case => $message,)*
                })
            }
        }
    };
}

errno! {
    _2big(1) = "Argument list too long.",
    _acces = "Permission denied.",
    _addrinuse = "Address in use.",
    _addrnotavail = "Address not available.",
    _afnosupport = "Address family not supported.",
    _again = "Resource unavailable, or operation would block.",
    _already = "Connection already in progress.",
    _badf = "Bad file descriptor.",
    _badmsg = "Bad message.",
    _busy = "Device or resource busy.",
    _canceled = "Operation canceled.",
    _child = "No child processes.",
    _connaborted = "Connection aborted.",
    _connrefused = "Connection refused.",
    _connreset = "Connection reset.",
    _deadlk = "Resource deadlock would occur.",
    _destaddrreq = "Destination address required.",
    _dom = "Mathematics argument out of domain of function.",
    _dquot = "Reserved.",
    _exist = "File exists.",
    _fault = "Bad address.",
    _fbig = "File too large.",
    _hostunreach = "Host is unreachable.",
    _idrm = "Identifier removed.",
    _ilseq = "Illegal byte sequence.",
    _inprogress = "Operation in progress.",
    _intr = "Interrupted function.",
    _inval = "Invalid argument.",
    _io = "I/O error.",
    _isconn = "Socket is connected.",
    _isdir = "Is a directory.",
    _loop = "Too many levels of symbolic links.",
    _mfile = "File descriptor value too large.",
    _mlink = "Too many links.",
    _msgsize = "Message too large.",
    _multihop = "Reserved.",
    _nametoolong = "Filename too long.",
    _netdown = "Network is down.",
    _netreset = "Connection aborted by network.",
    _netunreach = "Network unreachable.",
    _nfile = "Too many files open in system.",
    _nobufs = "No buffer space available.",
    _nodev = "No such device.",
    _noent = "No such file or directory.",
    _noexec = "Executable file format error.",
    _nolck = "No locks available.",
    _nolink = "Reserved.",
    _nomem = "Not enough space.",
    _nomsg = "No message of the desired type.",
    _noprotoopt = "Protocol not available.",
    _nospc = "No space left on device.",
    _nosys = "Function not supported.",
    _notconn = "The socket is not connected.",
    _notdir = "Not a directory or a symbolic link to a directory.",
    _notempty = "Directory not empty.",
    _notrecoverable = "State not recoverable.",
    _notsock = "Not a socket.",
    _notsup = "Not supported, or operation not supported on socket.",
    _notty = "Inappropriate I/O control operation.",
    _nxio = "No such device or address.",
    _overflow = "Value too large to be stored in data type.",
    _ownerdead = "Previous owner died.",
    _perm = "Operation not permitted.",
    _pipe = "Broken pipe.",
    _proto = "Protocol error.",
    _protonosupport = "Protocol not supported.",
    _prototype = "Protocol wrong type for socket.",
    _range = "Result too large.",
    _rofs = "Read-only file system.",
    _spipe = "Invalid seek.",
    _srch = "No such process.",
    _stale = "Reserved.",
    _timedout = "Connection timed out.",
    _txtbsy = "Text file busy.",
    _xdev = "Cross-device link.",
    _notcapable = "Capabilities insufficient.",
}

#[cfg(feature = "std")]
impl std::error::Error for Errno {}

impl From<wasm2rs_rt_memory::BoundsCheckError> for Errno {
    fn from(error: wasm2rs_rt_memory::BoundsCheckError) -> Self {
        let _ = error;
        Self::_fault
    }
}
