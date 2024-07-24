#ifndef MIZU_HAL
#define MIZU_HAL

void _start() __attribute__((section(".start")));

#define CPUID_BASE    0x10000
#define CPUID_INFO    (char*)CPUID_BASE

inline void* memcpy(void* dst, const void* src, unsigned long n) {
  unsigned char* d = dst;
  const unsigned char* s = src;
  while(n--) {
    *d++ = *s++;
  }
  return dst;
}

inline int strcmp(const char* s1, const char* s2) {
  const unsigned char* p1 = (const unsigned char*) s1;
  const unsigned char* p2 = (const unsigned char*) s2;
  while(*p1 && *p1 == *p2) {
    ++p1;
    ++p2;
  }
  return (*p1 > *p2) - (*p2 > *p1);
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

#endif
