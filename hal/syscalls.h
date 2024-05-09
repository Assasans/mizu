#ifndef MIZU_SYSCALL
#define MIZU_SYSCALL

#define SYSCALL_DISCORD     10
#define SYSCALL_PERF_DUMP   11
#define SYSCALL_HTTP        12

inline void syscall(int num) {
  asm volatile(
    "li a7, %0\n"
    "ecall" :: "i"(num) : "a7"
  );
}

inline unsigned long syscall_discord(int id, void* data) {
  asm volatile(
    "li a0, %0\n"
    "mv a1, %1" :: "i"(id), "r"(data) : "a0", "a1"
  );
  syscall(SYSCALL_DISCORD);

  unsigned long message_id;
  asm volatile("mv %0, a0" : "=r"(message_id) :: "a0");
  return message_id;
}

inline void syscall_perf_dump() {
  syscall(SYSCALL_HTTP);
}

#endif
