// Copyright (c) 2018-2022, agnos.ai UK Ltd, all rights reserved.
//---------------------------------------------------------------

use std::{
    ffi::{CStr, CString},
    ptr,
    sync::Arc,
};

use crate::{
    database_call,
    error::Error,
    root::{
        CServerConnection,
        CServerConnection_createDataStore,
        CServerConnection_deleteDataStore,
        CServerConnection_destroy,
        CServerConnection_getNumberOfThreads,
        CServerConnection_getVersion,
        CServerConnection_newDataStoreConnection,
        CServerConnection_setNumberOfThreads,
    },
    DataStore,
    DataStoreConnection,
    RoleCreds,
    Server,
};

#[derive(Debug)]
pub struct ServerConnection {
    #[allow(dead_code)]
    role_creds: RoleCreds,
    server:     Arc<Server>,
    inner:      *mut CServerConnection,
}

impl Drop for ServerConnection {
    fn drop(&mut self) { self.destroy() }
}

impl std::fmt::Display for ServerConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "connection to {:})", self.server)
    }
}

impl ServerConnection {
    pub(crate) fn new(
        role_creds: RoleCreds,
        server: Arc<Server>,
        server_connection_ptr: *mut CServerConnection,
    ) -> Self {
        assert!(!server_connection_ptr.is_null());
        assert!(
            server.is_running(),
            "cannot connect to an RDFox server that is not running"
        );
        Self {
            role_creds,
            server,
            inner: server_connection_ptr,
        }
    }

    /// Return the version number of the underlying database engine
    ///
    /// CRDFOX const CException*
    /// CServerConnection_getVersion(
    ///     CServerConnection* serverConnection,
    ///     const char** version
    /// );
    pub fn get_version(&self) -> Result<String, Error> {
        let mut c_buf: *const std::os::raw::c_char = ptr::null();
        database_call!(
            "Getting the version",
            CServerConnection_getVersion(self.inner, &mut c_buf)
        )?;
        let c_version = unsafe { CStr::from_ptr(c_buf) };
        Ok(c_version.to_str().unwrap().to_owned())
    }

    pub fn get_number_of_threads(&self) -> Result<u32, Error> {
        let mut number_of_threads = 0_usize;
        database_call!(
            "Getting the number of threads",
            CServerConnection_getNumberOfThreads(self.inner, &mut number_of_threads)
        )?;
        log::debug!("Number of threads is {}", number_of_threads);
        Ok(number_of_threads as u32)
    }

    pub fn set_number_of_threads(&self, number_of_threads: u32) -> Result<(), Error> {
        assert!(!self.inner.is_null());
        let msg = format!("Setting the number of threads to {}", number_of_threads);
        database_call!(
            msg.as_str(),
            CServerConnection_setNumberOfThreads(self.inner, number_of_threads as usize)
        )
    }

    pub fn delete_data_store(&self, data_store: &DataStore) -> Result<(), Error> {
        assert!(!self.inner.is_null());
        let msg = format!("Deleting {data_store}");
        let c_name = CString::new(data_store.name.as_str()).unwrap();
        database_call!(
            msg.as_str(),
            CServerConnection_deleteDataStore(self.inner, c_name.as_ptr())
        )
    }

    pub fn create_data_store(&self, data_store: &DataStore) -> Result<(), Error> {
        log::trace!("Creating {data_store}");
        assert!(!self.inner.is_null());
        let c_name = CString::new(data_store.name.as_str()).unwrap();
        database_call!(
            "creating a datastore",
            CServerConnection_createDataStore(
                self.inner,
                c_name.as_ptr(),
                data_store.parameters.inner,
            )
        )?;
        log::debug!("Created {data_store}");
        Ok(())
    }

    pub fn connect_to_data_store<'b>(
        self: &'b Arc<Self>,
        data_store: &Arc<DataStore>,
    ) -> Result<Arc<DataStoreConnection>, Error> {
        log::debug!("Connecting to {}", data_store);
        assert!(!self.inner.is_null());
        let mut ds_connection = DataStoreConnection::new(self, data_store, ptr::null_mut());
        let c_name = CString::new(data_store.name.as_str()).unwrap();
        log::error!(
            target: crate::LOG_TARGET_DATABASE,
            "Creating datastore connection {}",
            ds_connection.number
        );
        database_call!(
            "creating a datastore connection",
            CServerConnection_newDataStoreConnection(
                self.inner,
                c_name.as_ptr(),
                &mut ds_connection.inner,
            )
        )?;
        log::error!(
            target: crate::LOG_TARGET_DATABASE,
            "Connected to {}",
            data_store
        );
        Ok(Arc::new(ds_connection))
    }

    fn destroy(&mut self) {
        assert!(!self.inner.is_null());
        unsafe {
            CServerConnection_destroy(self.inner);
        }
        self.inner = ptr::null_mut();
        log::debug!("Destroyed server connection");
    }
}
