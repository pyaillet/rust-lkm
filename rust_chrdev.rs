//! Rust device sample

#![no_std]
#![feature(allocator_api, global_asm)]

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use kernel::{
    file::File,
    file_operations::{FileOpener, FileOperations},
    io_buffer::IoBufferWriter,
    miscdev::Registration,
    prelude::*,
    str::CStr,
    sync::Ref,
    ThisModule,
};

module! {
    type: Rustdev,
    name: b"rust_mydev",
    author: b"Rust for Linux Contributors",
    description: b"Rust character device sample",
    license: b"GPL v2",
}

struct Shared {
    open_count: AtomicU64,
}

struct RustFile {
    read_count: AtomicUsize,
}

impl FileOpener<Ref<Shared>> for RustFile {
    fn open(shared: &Ref<Shared>) -> Result<Box<Self>> {
        shared.open_count.fetch_add(1, Ordering::SeqCst);
        pr_info!(
            "Opened the file {} times\n",
            shared.open_count.load(Ordering::SeqCst)
        );
        Ok(Box::try_new(Self {
            read_count: AtomicUsize::new(0),
        })?)
    }
}

const HELLO: &'static str = "🦀 Hello from rust\n\0";

impl FileOperations for RustFile {
    kernel::declare_file_operations!(read);

    fn read(
        this: &Self,
        _file: &File,
        data: &mut impl IoBufferWriter,
        _offset: u64,
    ) -> Result<usize> {
        let hello_bytes = HELLO.as_bytes();
        if hello_bytes.len() > this.read_count.load(Ordering::SeqCst) {
            if data.len() >= hello_bytes.len() {
                data.write_slice(&hello_bytes)?;
                this.read_count.store(hello_bytes.len(), Ordering::Relaxed);
                return Ok(hello_bytes.len());
            }
        }
        Ok(0)
    }
}

struct Rustdev {
    _dev: Pin<Box<Registration<Ref<Shared>>>>,
}

impl KernelModule for Rustdev {
    fn init(name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust device sample (init)\n");

        let shared = Ref::try_new(Shared {
            open_count: AtomicU64::new(0),
        })?;

        Ok(Rustdev {
            _dev: Registration::new_pinned::<RustFile>(name, None, shared)?,
        })
    }
}

impl Drop for Rustdev {
    fn drop(&mut self) {
        pr_info!("Rust device sample (exit)\n");
    }
}
