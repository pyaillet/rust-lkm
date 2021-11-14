obj-m += rust_chrdev.o
OPTIONS = "LLVM=1 -j5"


all:
	make LLVM=1 -j5 -C /lib/modules/$(shell uname -r)/build M=$(PWD) modules

clean:
	make -C /lib/modules/$(shell uname -r)/build M=$(PWD) clean

rust-analyzer:
	make LLVM=1 -C /lib/modules/$(shell uname -r)/build rust-analyzer
	mv /lib/modules/$(shell uname -r)/build/rust-project.json .

