#ifndef HTTP_H
#define HTTP_H

#include "syscalls.h"

typedef struct http_request {
  const char* url;
} http_request_t;

typedef struct http_response {
  const unsigned short status_code;
  const char* body;
} http_response_t;

inline void syscall_http(http_request_t* request, http_response_t** response) {
  asm volatile("mv a0, %0" :: "r"(request) : "a0");
  syscall(SYSCALL_HTTP);

  asm volatile("mv %0, a0" : "=r"(*response) :: "a0");
}

#endif
