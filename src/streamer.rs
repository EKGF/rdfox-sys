use std::{
    ffi::{c_void, CString},
    fmt::Debug,
    io::Write,
    ptr,
};

use mime::Mime;

use crate::{
    cursor::ptr_to_cstr,
    database_call,
    root::{
        size_t,
        CDataStoreConnection,
        CDataStoreConnection_evaluateStatement,
        COutputStream,
        CPrefixes,
    },
    DataStoreConnection,
    Error,
    Parameters,
    Prefix,
    Statement,
};

#[derive(PartialEq, Debug)]
struct RefToSelf<'a, W: 'a + Write + Debug> {
    streamer: *mut Streamer<'a, W>,
}

impl<'a, W: 'a + Write + Debug> Drop for RefToSelf<'a, W> {
    fn drop(&mut self) {
        log::trace!("{:p}: Dropping RefToSelf ({self:p})", self.streamer);
    }
}

/// A `Streamer` is a helper-object that's created by `evaluate_to_stream`
/// to handle the various callbacks from the underlying C-API to RDFox.
#[derive(PartialEq, Debug)]
pub struct Streamer<'a, W: 'a + Write + Debug> {
    pub connection: &'a DataStoreConnection<'a>,
    pub writer:     W,
    pub statement:  &'a Statement<'a>,
    pub mime_type:  &'static Mime,
    pub base_iri:   Prefix,
    pub instant:    std::time::Instant,
    self_p:         String,
}

impl<'a, W: 'a + Write + Debug> Drop for Streamer<'a, W> {
    fn drop(&mut self) {
        log::trace!("{}: Dropped streamer", self.self_p);
    }
}

impl<'a, W: 'a + Write + Debug> Streamer<'a, W> {
    pub fn run(
        connection: &'a DataStoreConnection,
        writer: W,
        statement: &'a Statement<'a>,
        mime_type: &'static Mime,
        base_iri: Prefix,
    ) -> Result<Self, Error> {
        let streamer = Self {
            connection,
            writer,
            statement,
            mime_type,
            base_iri,
            instant: std::time::Instant::now(),
            self_p: "".to_string(),
        };
        streamer.evaluate()
    }

    /// Evaluate/execute the statement and stream all content to the given
    /// writer, then return the streamer (i.e. self).
    fn evaluate(mut self) -> Result<Self, Error> {
        let base_iri = ptr::null_mut();
        let statement_text = self.statement.as_c_string()?;
        let statement_text_len: u64 = statement_text.as_bytes().len() as u64;
        let parameters = Parameters::empty()?;
        let query_answer_format_name = CString::new(self.mime_type.as_ref())?;
        let mut number_of_solutions = 0u64;
        let prefixes_ptr = self.prefixes_ptr();
        let connection_ptr = self.connection_ptr();

        let self_p = format!("{:p}", &self);
        self.self_p = self_p.clone();

        log::debug!("{self_p}: evaluate statement with mime={query_answer_format_name:?}");

        let ref_to_self = Box::new(RefToSelf {
            streamer: &mut self as *mut Self,
        });
        let ref_to_self_raw_ptr = Box::into_raw(ref_to_self);

        let stream = Box::new(COutputStream {
            context: ref_to_self_raw_ptr as *mut _,
            flushFn: Some(Self::flush_function),
            writeFn: Some(Self::write_function),
        });
        let stream_raw_ptr = Box::into_raw(stream);

        let result = database_call! {
            "evaluating a statement",
            CDataStoreConnection_evaluateStatement(
                connection_ptr,
                base_iri,
                prefixes_ptr,
                statement_text.as_ptr(),
                statement_text_len,
                parameters.inner,
                stream_raw_ptr as *const COutputStream,
                query_answer_format_name.as_ptr(),
                &mut number_of_solutions,
            )
        };
        // Explicitly clean up the two boxes that we allocated
        unsafe {
            ptr::drop_in_place(ref_to_self_raw_ptr);
            ptr::drop_in_place(stream_raw_ptr);
        }

        result?; // we're doing this after the drop_in_place calls to avoid memory leak

        log::debug!("{self_p}: number_of_solutions={number_of_solutions}");
        Ok(self)
    }

    unsafe fn context_as_ref_to_self(context: *mut c_void) -> &'a mut RefToSelf<'a, W> {
        let ref_to_self = context as *mut RefToSelf<'a, W>;
        let ref_to_self = &mut *ref_to_self;
        ref_to_self
    }

    extern "C" fn flush_function(context: *mut c_void) -> bool {
        let ref_to_self = unsafe { Self::context_as_ref_to_self(context) };
        let streamer = unsafe { &mut *ref_to_self.streamer };
        streamer.flush()
    }

    extern "C" fn write_function(
        context: *mut c_void,
        data: *const c_void,
        number_of_bytes_to_write: size_t,
    ) -> bool {
        let ref_to_self = unsafe { Self::context_as_ref_to_self(context) };
        let streamer = unsafe { &mut *ref_to_self.streamer };

        log::trace!("{streamer:p}: write_function");

        match ptr_to_cstr(data as *const u8, number_of_bytes_to_write as usize) {
            Ok(data_c_str) => {
                log::trace!("{streamer:p}: writing {number_of_bytes_to_write} bytes (a)");
                streamer.write(data_c_str.to_bytes_with_nul())
            },
            Err(error) => {
                log::error!("{streamer:p}: could not write: {error:?}");
                false
            },
        }
    }

    fn prefixes_ptr(&self) -> *mut CPrefixes { self.statement.prefixes.inner }

    fn connection_ptr(&self) -> *mut CDataStoreConnection { self.connection.inner }
}

trait StreamerWithCallbacks {
    fn flush(&mut self) -> bool;
    fn write(&mut self, data: &[u8]) -> bool;
}

impl<'a, W: 'a + Write + Debug> StreamerWithCallbacks for Streamer<'a, W> {
    fn flush(&mut self) -> bool {
        log::trace!("{self:p}: flush");
        let y = if let Err(err) = self.writer.flush() {
            panic!("{self:p}: Could not flush: {err:?}")
        } else {
            true
        };
        log::trace!("{self:p}: flush returns {y:?}");
        y
    }

    fn write(&mut self, data: &[u8]) -> bool {
        log::trace!("{self:p}: writing {} bytes (b)", data.len());
        match self.writer.write(data) {
            Ok(len) => {
                log::trace!("{self:p}: wrote {len} bytes");
                true
            },
            Err(err) => {
                panic!("{self:p}: could not write: {err:?}")
            },
        }
    }
}