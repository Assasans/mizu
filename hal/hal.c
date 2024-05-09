void _start() __attribute__((section(".start")));

inline void syscall(int num) {
  asm volatile(
    "li a7, %0\n"
    "ecall" :: "i"(num)
  );
}

inline unsigned long syscall_discord(int id, void* data) {
  asm volatile(
    "li a0, %0\n"
    "mv a1, %1" :: "i"(id), "r"(data) : "a0"
  );
  syscall(10);

  unsigned long message_id;
  asm volatile("mv %0, a0" : "=r"(message_id));
  return message_id;
}

#define DISCORD_CREATE_MESSAGE   1
#define DISCORD_CREATE_REACTION  2
#define DISCORD_POLL_EVENT       10

typedef struct discord_create_message {
  unsigned long channel_id;
  unsigned long flags;
  unsigned long reply;
  unsigned long stickers[3];
  const char* content;
} discord_create_message_t;

typedef struct discord_create_reaction {
  unsigned long channel_id;
  unsigned long message_id;
  const char* emoji;
} discord_create_reaction_t;

typedef struct discord_message {
  unsigned long id;
  unsigned long channel_id;
  unsigned long author_id;
  const char* content;
} discord_message_t;

void* memcpy(void *dst, const void *src, unsigned long n) {
  unsigned char *d = dst;
  const unsigned char *s = src;
  while(n--) {
    *d++ = *s++;
  }
  return dst;
}

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
