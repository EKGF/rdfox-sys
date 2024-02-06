use {
    crate::CException,
    std::{
        ffi::CStr,
        fmt::{Display, Formatter},
        panic::catch_unwind,
        str::Utf8Error,
    },
};

impl CException {
    pub fn handle<F>(action: &str, f: F) -> Result<(), crate::Error>
    where F: FnOnce() -> *const CException + std::panic::UnwindSafe {
        unsafe {
            let result = catch_unwind(|| {
                let c_exception = f();
                if c_exception.is_null() {
                    Ok(())
                } else {
                    Err(crate::Error::Exception {
                        action:  action.to_string(),
                        message: format!("{:}", *c_exception).replace("RDFoxException: ", ""),
                    })
                }
            });
            match result {
                Ok(res) => {
                    match res {
                        Ok(..) => Ok(()),
                        Err(err) => {
                            // panic!("{err:}")
                            Err(err)
                        },
                    }
                },
                Err(err) => {
                    panic!("RDFox panicked while {action}: {:?}", err)
                },
            }
        }
    }

    pub fn name(&self) -> Result<&'static str, Utf8Error> {
        let name = unsafe { CStr::from_ptr(crate::CException_getExceptionName(self)) };
        name.to_str()
    }

    pub fn what(&self) -> Result<&'static str, Utf8Error> {
        let what = unsafe { CStr::from_ptr(crate::CException_what(self)) };
        what.to_str()
    }
}

impl Display for CException {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Ok(name) = self.name() {
            if let Ok(what) = self.what() {
                return writeln!(f, "{:}: {:}", name, what);
            };
        };
        f.write_str("Could not show exception, unicode error")
    }
}
