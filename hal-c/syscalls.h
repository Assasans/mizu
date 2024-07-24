#ifndef MIZU_SYSCALL
#define MIZU_SYSCALL

#define SYSCALL_DISCORD         10
#define SYSCALL_PERF_DUMP       11
#define SYSCALL_HTTP            12
#define SYSCALL_OBJECT_STORAGE  13

inline void syscall(int num) {
  asm volatile(
    "li a7, %0\n"
    "ecall" :: "i"(num) : "a7"
  );
}

inline void syscall_perf_dump() {
  syscall(SYSCALL_HTTP);
}

#endif
