# Écrire un module pour Linux en Rust

Le 14 avril 2021 [une série de patch](https://lkml.org/lkml/2021/4/14/1023)
a été soumise sur la mailing list du
kernel Linux pour initier la discussion et proposer une première RFC en vue
d'ajouter le langage Rust comme second langage intégré au projet.

Le 4 juillet, [une nouvelle série de patch](https://lkml.org/lkml/2021/7/4/171)
a été envoyée, cette fois-ci pour activer le support et le rendre accessible
aux développeurs du kernel.

Dans cet article, je vais vous présenter l'intérêt de cette démarche et un
exemple "simple" de module Rust pour le noyau Linux.

## L'intérêt de Rust

L'objectif du langage Rust est de fournir un langage bas niveau, mais
fournissant davantages de garanties qu'un langage comme le C.
Les invariants tels que :
- Une référence vers une valeur ne peut pas exister au delà de l'existence de la valeur
- Une seule référence mutable vers une valeur peut exister à un instant donné

Offrent des garanties sur la manipulation de la mémoire. Elles permettent
d'éviter des comportements indéfinis liés notamment à l'utilisation de pointeurs
non-initialisés (Null pointers, Use after-free, ...).

Le système de type de Rust nous permet également d'obtenir des garanties sur
les données manipulées vérifiées par le compilateur alors que dans le cadre d'une
implémentation en C, c'est au développeur de prendre les précautions nécessaires.

Si vous voulez en savoir plus à ce sujet, je vous invite à regarder
[cette présentation](https://www.youtube.com/watch?v=46Ky__Gid7M) qui détaille
ce qu'on entend par comportement indéfini et comment Rust permet de les limiter.

## Compilation du kernel avec support de Rust  

Pour commencer, nous allons devoir compiler notre propre kernel Linux avec le
support de Rust activé.
De mon côté, j'ai décidé de faire ça sur une machine virtuelle Ubuntu 21.04.

La plupart des étapes de ce tutoriel sont basés sur la documentation officiel
du projet [Rust for Linux](https://github.com/Rust-for-Linux/linux/blob/rust/Documentation/rust/quick-start.rst).

On va commencer par l'installation des outils nécessaires à la compilation :

```sh
# Distro packages
sudo apt update
sudo apt install -y flex bison clang lld build-essential llvm git libelf-dev libclang-11-dev libssl-dev tmux
```

Ensuite on installe la toolchain Rust qui sera utilisée pour la compilation du
kernel et les dépendances nécessaires.

`rust-src` : le code source de la standard librairie Rust est nécessaire car on
va recompiler `core` et `alloc`
`bindgen` : sera utilisé pour générer les bindings avec le C du kernel lors du build.

```sh
# Rust dependencies
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup component add rust-src
cargo install --locked --version 0.56.0 bindgen
```

On clone les sources du kernel intégrant les patchs nécessaires pour Rust :

```sh
# Clone kernel src
git clone --depth=1 https://github.com/Rust-for-Linux/linux.git
```

Il faut ensuite configurer le kernel pour activer le support de Rust et intégrer
nos exemples.

On commence par recopier la configuration actuelle du kernel afin de minimiser
les changements à effectuer.

```sh
cp /boot/config-$(uname -r) linux/.config
cd linux
make oldconfig
```

On va ensuite devoir configurer les options spécifiques :
```sh
make menuconfig
```

Vous pouvez vous baser sur cette liste pour savoir quoi activer/désactiver:

Il est nécessaire de désactiver le versioning des modules :
```
Enable loadable module support => [ ] Module versioning support
```

On peut alors activer le support de Rust :
```
General Setup => [*] Rust support
```

Et activer la compilation d'un exemple de driver :
```
Kernel Hacking => Sample kernel code => [*] Rust samples => <M> Character device
```

De mon côté, j'ai également dû désactiver certaines options pour faire passer
la compilation :
```
Kernel Hacking => Compile-time checks and compiler options => [ ] Compile the kernel with debug info
Cryptographic API => Certificates for signature checking => () Additional X.509 keys for default system keyring 
Cryptographic API => Certificates for signature checking => () X.509 certificates to be preloaded into the system blacklist keyring
```

On peut alors lancer la compilation et aller se chercher 2~3 cafés...
Il faudra adapter le `-j5` en fonction du nombre de core disponibles sur la
machine utilisée pour la compilation (en général, on choisit `nombre de core + 1`,
ce qui permet de lancer 5 tâches de compilation en parallèle, et d'occuper tous
les core, même en prenant en compte le fait que certaines tâches attendant sur
des IO).
```sh
make LLVM=1 -j5
```

Une fois la compilation terminée, on installe les modules dans l'arborescence
du système et on installe le kernel.

```sh
sudo make modules_install
sudo make install
```

## Test 

Pour tester vous pouvez alors redémarrer la machine et vérifier la version du
kernel utilisé suite à ce redémarrage :

```sh
sudo reboot
uname -a 
```

Il est alors possible de charger le module écrit en Rust.
```shell-session
$ sudo insmod /lib/modules/$(uname -r)/kernel/samples/rust/rust_chrdev.ko
$ lsmod | grep rust
rust_chrdev            16384  0
$ sudo rmmod rust_chrdev
$ sudo dmesg | grep rust_chrdev
[27357.104859] rust_chrdev: Rust character device sample (init)
[27425.248428] rust_chrdev: Rust character device sample (exit)
```

Nous avons donc pu charger notre module exemple écrit en Rust !

## Exemple de module écrit en Rust avec explications

Je vous propose maintenant de passer en revue les étapes nécessaires à
l'écriture d'un module en Rust. Petite mise en garde néanmoins, ce n'est pas ma
spécialité et il est possible que ma compréhension de certains aspects ne soit
que partielle.

Le premier module que nous avions chargé était directement intégré dans
l'arborescence du noyau, mais ce n'est pas indispensable.
Vous pouvez retrouver cet exemple sur [ce repo](https://github.com/Rust-for-Linux/linux/tree/rust/samples/rust).

Nous allons commencer par le `Makefile` qui sera utilisé pour compiler notre
module. 
```Makefile
# On déclare le module à compiler en indiquant le fichier objet résultant
obj-m += rust_chrdev.o

# On déclare notre cible par défaut en précisant :
# - LLVM=1 : Qu'on souhaite utiliser LLVM
# - -C /lib/modules/$(shell uname -r)/build : On utilise le système de build du kernel
# - M=$(PWD) : On indique le chemin du module
# - modules : On indique qu'on souhaite compiler notre module
all:
	make LLVM=1 -j5 -C /lib/modules/$(shell uname -r)/build M=$(PWD) modules

# On ajoute une cible pour indiquer comment faire le ménage
clean:
	make -C /lib/modules/$(shell uname -r)/build M=$(PWD) clean
```

Passons maintenant au code de notre module, vous pouvez retrouver l'exemple
complet [sur github](https://github.com/pyaillet/rust-lkm).
Nous allons commencer par un exemple simple, notre module créera un
`character device` qui transmettra la chaîne `🦀 Hello from rust\n` lorsqu'on
lira dedans, une fois le fichier ouvert 
On stockera également un état partagé qui nous permettra de comptabiliser
combien de fois le fichier a été ouvert.

Commençons par déclarer les structures qui stockeront l'état de lecture du
fichier device et l'état partagé de notre module :

```rs
/// État partagé dans notre module
struct Shared {
    open_count: AtomicU64,
}

/// Compteur d'octets lus sur notre fichier
struct RustFile {
    read_count: AtomicUsize,
}
```

Ensuite, nous déclarons la structure qui contient l'enregistrement de notre
module :

```rs
/// Structure correspondant à notre module, qui porte l'état partagé de
/// l'enregistrement du device
struct Rustdev {
    _dev: Pin<Box<Registration<Ref<Shared>>>>,
}
```

Cette structure ne possède qu'un seul membre : l'enregistrement `Registration`
du device et qui porte l'état partagé.

Pour initialiser, le module et enregistrer notre device, il faut implémenter
le trait `KernelModule` pour notre device :

```rs
/// 
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
```

On implémente également le trait `Drop` qui sera utilisé si on décharge le
module.
```rs
impl Drop for Rustdev {
    fn drop(&mut self) {
        pr_info!("Rust device sample (exit)\n");
    }
}
```

Il nous manque encore l'appel à une macro pour finaliser les déclarations
nécessaires à la prise en compte de notre module :
```rs
module! {
    type: Rustdev,
    name: b"rust_mydev",
    author: b"Pierre-Yves Aillet",
    description: b"Rust character device sample",
    license: b"GPL v2",
}
```

Il nous manque encore 2 traits à implémenter :
- `FileOpener` pour traiter l'ouverture du fichier de notre `character device`
- `FileOperations` pour implémenter le comportement lors de la lecture de ce fichier

```rs
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
```

```rs
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
                this.read_count.store(hello_bytes.len(), Ordering::SeqCst);
                // On retourne le nombre d'octets lus et réellement écrits
                // dans le buffer
                return Ok(hello_bytes.len());
            }
        }
        // Dans les autres cas, on indique qu'aucun octet n'a été lu
        Ok(0)
    }
}
```

Vous pouvez retrouver l'exemple complet [ici]().

Voici un exemple de session avec utilisation de ce module :

```shell-session
$ make
make LLVM=1 -j5 -C /lib/modules/5.15.0+/build M=/home/pyaillet.linux/rust-lkm modules
make[1]: Entering directory '/home/pyaillet.linux/linux'
  RUSTC [M] /home/pyaillet.linux/rust-lkm/rust_chrdev.o
  MODPOST /home/pyaillet.linux/rust-lkm/Module.symvers
  CC [M]  /home/pyaillet.linux/rust-lkm/rust_chrdev.mod.o
  LD [M]  /home/pyaillet.linux/rust-lkm/rust_chrdev.ko
make[1]: Leaving directory '/home/pyaillet.linux/linux'
$ sudo insmod rust_chrdev.ko
$ sudo dmesg | grep rust_mydev
[   55.920542] rust_mydev: Rust device sample (init)
$ sudo cat /dev/rust_mydev
🦀 Hello from rust
$ sudo cat /dev/rust_mydev
🦀 Hello from rust
$ sudo dmesg | grep rust_mydev
[   55.920542] rust_mydev: Rust device sample (init)
[   75.415790] rust_mydev: Opened the file 1 times
[   76.808057] rust_mydev: Opened the file 2 times
$ sudo cat /dev/rust_mydev
🦀 Hello from rust
$ sudo dmesg | grep rust_mydev
[   55.920542] rust_mydev: Rust device sample (init)
[   75.415790] rust_mydev: Opened the file 1 times
[   76.808057] rust_mydev: Opened the file 2 times
[   82.857408] rust_mydev: Opened the file 3 times
$ sudo rmmod rust_chrdev
$ sudo dmesg | grep rust_mydev
[   55.920542] rust_mydev: Rust device sample (init)
[   75.415790] rust_mydev: Opened the file 1 times
[   76.808057] rust_mydev: Opened the file 2 times
[   82.857408] rust_mydev: Opened the file 3 times
[   95.155032] rust_mydev: Rust device sample (exit)
$
```

## Conclusion

Il reste encore du chemin à parcourir pour voir de nombreux drivers Linux
implémenter en Rust.
Comme décrit [ici](https://github.com/Rust-for-Linux/linux/blob/rust/Documentation/rust/coding.rst#abstractions-vs-bindings),
une grosse partie du travail restant consiste à disposer des abstractions
permettant d'interagir avec les APIs internes du kernel tout en conservant les
garanties fournies par Rust.
Si le sujet vous intéresse je vous invite à regarder les présentations données
en référence.

## Références

- [Rust in the Linux kernel](https://security.googleblog.com/2021/04/rust-in-linux-kernel.html)
- [Rust modules samples](https://github.com/Rust-for-Linux/linux/tree/rust/samples/rust)
- [Rust for Linux](https://www.youtube.com/watch?v=46Ky__Gid7M)
- [Rust in the Linux ecosystem](https://www.youtube.com/watch?v=jTWdk0jYy54)
