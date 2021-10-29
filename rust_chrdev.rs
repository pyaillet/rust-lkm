// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample

#![no_std]
#![feature(allocator_api, global_asm)]

use kernel::prelude::*;
use kernel::{
    c_str, chrdev, file::File, file_operations::FileOperations, io_buffer::IoBufferWriter,
};

module! {
    type: RustChrdev,
    name: b"rust_chrdev",
    author: b"Rust for Linux Contributors",
    description: b"Rust character device sample",
    license: b"GPL v2",
}

#[derive(Default)]
struct RustFile;

impl FileOpener for RustFile {}

impl FileOperations for RustFile {
    // kernel::declare_file_operations!();
    kernel::declare_file_operations!(open, read, read_iter);

    fn read(
        _shared: &Self,
        _file: &File,
        data: &mut impl IoBufferWriter,
        _offset: u64,
    ) -> Result<usize> {
        let hello = "Hello from the kernel in Rust".as_bytes();
        data.write_slice(&hello)?;
        Ok(hello.len())
    }
}

struct RustChrdev {
    _dev: Pin<Box<chrdev::Registration<1>>>,
}

impl KernelModule for RustChrdev {
    fn init() -> Result<Self> {
        pr_info!("Rust character device sample (init)\n");

        let mut chrdev_reg =
            chrdev::Registration::new_pinned(c_str!("rust_chrdev"), 0, &THIS_MODULE)?;

        chrdev_reg.as_mut().register::<RustFile>()?;

        Ok(RustChrdev { _dev: chrdev_reg })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust character device sample (exit)\n");
    }
}
