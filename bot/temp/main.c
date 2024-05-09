#include <hal.c>

void _start() {
  discord_create_message_t message = {
    .content = (char*)&"у гитлера встал",
    .reply = 1237909978950664252,
    .stickers = { 1171892398767489146 }
  };
  syscall_discord(DISCORD_CREATE_MESSAGE, &message);
}