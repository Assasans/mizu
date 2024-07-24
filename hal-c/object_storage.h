#ifndef MIZU_OBJECT_STORAGE
#define MIZU_OBJECT_STORAGE

typedef struct object_storage_item {
  const unsigned long length;
  const char* data;
} object_storage_item_t;

typedef struct object_storage_get {
  const char* key;
} object_storage_get_t;

typedef struct object_storage_put {
  const char* key;
  const object_storage_item_t item;
} object_storage_put_t;

#define OBJECT_STORAGE_GET  1
#define OBJECT_STORAGE_PUT  2

inline void syscall_object_storage(int action, void* request, void** response) {
  asm volatile(
    "li a0, %0\n"
    "mv a1, %1" :: "i"(action), "r"(request) : "a0", "a1"
  );
  syscall(SYSCALL_OBJECT_STORAGE);

  asm volatile("mv %0, a0" : "=r"(*response) :: "a0");
}

inline void object_storage_get(object_storage_get_t* request, object_storage_item_t** item) {
  syscall_object_storage(OBJECT_STORAGE_GET, request, (void**) item);
}

inline void object_storage_put(object_storage_put_t* request) {
  object_storage_item_t* item;
  syscall_object_storage(OBJECT_STORAGE_PUT, request, (void**) &item);
}

#endif
