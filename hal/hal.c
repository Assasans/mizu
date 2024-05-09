void _start() __attribute__((section(".start")));

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
  asm volatile("mv %0, a0" : "=r"(message_id));
  return message_id;
}

#define CPUID_BASE    0x10000
#define CPUID_INFO    (char*)CPUID_BASE

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

typedef struct http_request {
  const char* url;
} http_request_t;

typedef struct http_response {
  const unsigned short status_code;
  const char* body;
} http_response_t;

inline void* memcpy(void *dst, const void *src, unsigned long n) {
  unsigned char *d = dst;
  const unsigned char *s = src;
  while(n--) {
    *d++ = *s++;
  }
  return dst;
}

inline int strcmp(const char *s1, const char *s2) {
  const unsigned char *p1 = (const unsigned char*)s1;
  const unsigned char *p2 = (const unsigned char*)s2;
  while(*p1 && *p1 == *p2) {
    ++p1;
    ++p2;
  }
  return (*p1 > *p2) - (*p2  > *p1);
}

inline discord_message_t* discord_poll() {
  syscall_discord(DISCORD_POLL_EVENT, 0);
  discord_message_t* message;
  asm volatile("mv %0, a0" : "=r"(message) :: "a0");
  return message;
}

inline void discord_create_message(discord_create_message_t* message) {
  syscall_discord(DISCORD_CREATE_MESSAGE, message);
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
