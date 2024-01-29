// Copyright (c) 2024, Object Management Group, all rights reserved.
//------------------------------------------------------------------
// #![feature(rustc_private)]
// #![feature(ptr_metadata)]
#![doc = include_str!("../README.md")]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![deny(unused_crate_dependencies)]

extern crate core;

pub use error::Error;

mod error;
mod exception;
#[macro_use]
mod r#macro;

// pub mod rdfox_api {
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
// }
