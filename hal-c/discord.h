#ifndef MIZU_DISCORD
#define MIZU_DISCORD

#include "syscalls.h"

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

inline discord_message_t* discord_poll() {
  syscall_discord(DISCORD_POLL_EVENT, 0);
  discord_message_t* message;
  asm volatile("mv %0, a0" : "=r"(message) :: "a0");
  return message;
}

inline void discord_create_message(discord_create_message_t* message) {
  syscall_discord(DISCORD_CREATE_MESSAGE, message);
}

#endif
