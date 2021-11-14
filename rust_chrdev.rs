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

/// État partagé dans notre module
struct Shared {
    open_count: AtomicU64,
}

/// Compteur d'octets lus sur notre fichier
struct RustFile {
    read_count: AtomicUsize,
}

/// Ce trait permet d'indiquer ce qui est réalisé lors de l'ouverture du
/// device.
/// Il est utilisé pour initialiser la structure qui correspond à l'état du
/// fichier ouvert, et peut également être utilisé pour y associer l'état
/// partagé (ce qui n'est pas fait dans cet exemple).
impl FileOpener<Ref<Shared>> for RustFile {
    fn open(shared: &Ref<Shared>) -> Result<Box<Self>> {
        // On met à jour le compteur d'ouverture du fichier
        shared.open_count.fetch_add(1, Ordering::SeqCst);

        // On affiche dans le `dmesg` le nombre de fois que le device a été
        // ouvert
        pr_info!(
            "Opened the file {} times\n",
            shared.open_count.load(Ordering::SeqCst)
        );

        // On initialise et on retourne la structure correspondant à l'ouverture
        // courante de notre fichier.
        Ok(Box::try_new(Self {
            read_count: AtomicUsize::new(0),
        })?)
    }
}

/// Constante correspondant à la chaîne que nous souhaitons renvoyer
const HELLO: &'static str = "🦀 Hello from rust\n\0";

/// Ce trait comporte l'ensemble des opérations possibles pour un fichier.
/// Voir la documentation [ici](https://rust-for-linux.github.io/docs/kernel/file_operations/trait.FileOperations.html)
impl FileOperations for RustFile {
    /// L'utilisation de cette macro permet de spécifier les opérations réellement
    /// implémentée pour notre device
    kernel::declare_file_operations!(read);

    /// Cette méthode est appelé lorsqu'une opération de lecture est réalisée
    /// sur le fichier device
    fn read(
        this: &Self,
        _file: &File,
        data: &mut impl IoBufferWriter,
        _offset: u64,
    ) -> Result<usize> {
        let hello_bytes = HELLO.as_bytes();
        // Si le fichier n'a pas déjà été lu
        if hello_bytes.len() > this.read_count.load(Ordering::SeqCst) {
            // Et si le buffer fournit est assez grand pour y écrire le message
            if data.len() >= hello_bytes.len() {
                // Alors on écrit notre message dans ce buffer
                data.write_slice(&hello_bytes)?;
                // On met à jour le compteur d'octets lu pour cette ouverture
                // de fichier
                this.read_count.store(hello_bytes.len(), Ordering::Relaxed);
                // On retourne le nombre d'octets lus et réellement écrits
                // dans le buffer
                return Ok(hello_bytes.len());
            }
        }
        // Dans les autres cas, on indique qu'aucun octet n'a été lu
        Ok(0)
    }
}

/// Structure correspondant à notre module, qui porte l'état partagé de
/// l'enregistrement du device
struct Rustdev {
    _dev: Pin<Box<Registration<Ref<Shared>>>>,
}

impl KernelModule for Rustdev {
    /// Cette méthode est appelé au chargement de notre module et permet
    /// d'effectuer les étapes d'initialisation
    fn init(name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        // Cette macro permet d'afficher un message d'information dans `dmesg`
        pr_info!("Rust device sample (init)\n");

        // Ici, on initialise l'état partagé qui comptera le nombre d'accès à
        // notre device
        let shared = Ref::try_new(Shared {
            open_count: AtomicU64::new(0),
        })?;

        // Enfin, on crée la structure correspondant à notre module, on crée
        // l'enregistrement qui portera notre état partagé.
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
