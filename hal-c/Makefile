hal.bin: hal.h syscalls.h http.h memmap.ld Makefile
	clang -O1 -Wl,-Tmemmap.ld -nostdlib --target=riscv64 -march=rv64g -mno-relax -o hal main.c
	llvm-objcopy -O binary hal hal.bin
	riscv64-unknown-elf-objdump -d --disassembler-color=on -S hal

all: hal.bin

clean:
	rm -v hal hal.bin
