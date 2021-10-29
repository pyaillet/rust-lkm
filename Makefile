obj-m += rust_chrdev.o


all:
	make LLVM=1 -j5 -C /lib/modules/$(shell uname -r)/build M=$(PWD) modules

clean:
	make LLVM=1 -j5 -C /lib/modules/$(shell uname -r)/build M=$(PWD) clean

