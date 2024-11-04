#![feature(never_type)]
#![allow(unused)]

mod gossip_model;
mod round_model;

use proptest_derive::Arbitrary;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
};

pub fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::LocalRuntime::new()
        .unwrap()
        .block_on(future)
}
