#![feature(decl_macro)]

use std::env;
use wgpu::{Backends, Instance, InstanceDescriptor};

pub fn set_up_logger() {
    unsafe {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
}

pub macro default() {
    Default::default()
}

pub fn wgpu_instance_with_env_backend() -> Instance {
    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::from_env().unwrap_or(default!()),
        ..default!()
    });
    instance
}