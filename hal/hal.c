inline void syscall(int num) {
  asm volatile(
    "li a7, %0\n"
    "ecall" :: "i"(num)
  );
}

inline void syscall_discord(int id, void* data) {
  asm volatile(
    "li a0, %0\n"
    "mv a1, %1" :: "i"(id), "r"(data) : "a0"
  );
  syscall(10);
}

#define DISCORD_CREATE_MESSAGE 1

typedef struct discord_create_message {
  unsigned long flags;
  unsigned long reply;
  unsigned long stickers[1];
  char* content;
} discord_create_message_t;

//void _start() {
//  int a;
//  int b = 1;
//  syscall(2112);
//  asm(
//    "li x1, 0x80000000"
//  );
//  asm volatile("addi a1, %0, 4" : "=r"(a) : "r"(b) :);
//  asm volatile("csrrw zero, mstatus, t0");
//}
