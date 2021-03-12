
#define _GNU_SOURCE

#include <stdio.h>
#include <dlfcn.h>
#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <pthread.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <execinfo.h>
#include <signal.h>
#include <stdlib.h>
#include <unistd.h>
#include <link.h>
#include <pthread.h>
#include <sys/types.h>
#include <unistd.h>

#define UTILS_EXPORT __attribute__((visibility("default")))
#define UTILS_INIT   __attribute__((constructor))
#define UTILS_FINI   __attribute__((destructor))

uint32_t adler32(const void *buf, size_t buflength) {
     const uint8_t *buffer = (const uint8_t*)buf;

     uint32_t s1 = 1;
     uint32_t s2 = 0;

     for (size_t n = 0; n < buflength; n++) {
        s1 = (s1 + buffer[n]) % 65521;
        s2 = (s2 + s1) % 65521;
     }
     return (s2 << 16) | s1;
}

static void* (*real_malloc)(size_t) = NULL;

static void mtrace_init(void)
{
    real_malloc = dlsym(RTLD_NEXT, "malloc");
    if (NULL == real_malloc) {
        fprintf(stderr, "Error in `dlsym`: %s\n", dlerror());
        abort();
    }
}

#define MAX_BT_DEPTH 12
#define MAX_HASH_COUNT 1024

void* pool_start[255];
void* pool_end[255];
int pool_num;
uint32_t hashes[MAX_HASH_COUNT];
int initialized = 0;
void* stack_end;

void UTILS_INIT bt_init()
{
    char proc_maps_path[255] = {0};

    pid_t PID = getpid();
    sprintf(proc_maps_path, "/proc/%d/maps", PID);

    FILE* fh = fopen(proc_maps_path, "r");

    int num = 0;
    int bufferLength = 1024;
    char buffer[bufferLength];

    void *start, *end;
    char r, w, x, p;
    void *off;
    char sth[6] = {0};
    unsigned int sth2;
    char path[255] = {0};


    while(fgets(buffer, bufferLength, fh)) {

        num = sscanf(buffer, "%p-%p %c%c%c%c %p %s %d %s",
                          &start, &end, &r, &w, &x, &p,
                          &off, sth, &sth2, path);

        if (strcmp(path, "[stack]") == 0) {
            stack_end = end;
            fprintf(stderr, "%s: %s", proc_maps_path, buffer);
        }

        if ((num < 10) || (x != 'x')) {
            continue;
        }

        fprintf(stderr, "%s: %s", proc_maps_path, buffer);
        pool_start[pool_num] = start;
        pool_end[pool_num] = end;
        pool_num++;
    }

    initialized = 1;
}

void UTILS_FINI bt_fini()
{
    int i;
    int sum_all = 0;
    int sum_dif = 0;
    for (i = 0; i < MAX_HASH_COUNT; i++) {
        if (hashes[i] != 0) {
            fprintf(stderr, "bucket: %d count: %d\n", i, hashes[i]);
            sum_all += hashes[i];
            sum_dif ++;
        }
    }

    fprintf(stderr, "all: %d dif: %d\n", sum_all, sum_dif);
}

UTILS_EXPORT void *malloc(size_t size)
{
    if (real_malloc == 0)
        mtrace_init();

    if (initialized) {
        unsigned char* stack = __builtin_frame_address(0);

        void* calls[MAX_BT_DEPTH];
        int num_calls = 0;
        int depth = 0;
        while ((num_calls < MAX_BT_DEPTH) && (stack <= stack_end - sizeof(size_t))) {
            int i;
            //fprintf(stderr, "looking for: %p\n", *((size_t*)stack));
            //fprintf(stderr, "%p-%p (%d) ", ((size_t*)stack), *((size_t*)stack), depth);
            for (i = 0; i < pool_num; i++) {
                if ((*((size_t*)stack) > pool_start[i]) &&
                    (*((size_t*)stack) < pool_end[i])) {
                    //fprintf(stderr, "call %p in %p-%p\n", stack, pool_start[i], pool_end[i]);
                    //fprintf(stderr, "found: %p ", *((size_t*)stack));
                    fprintf(stderr, "%p ", *((size_t*)stack));
                    calls[num_calls] = *((size_t*)stack);
                    num_calls ++;
                }
            }
            stack += 4; // 8? are there any alignment constraints that we could use?
            depth ++;
        }
        unsigned int hash = adler32(calls, num_calls);
        fprintf(stderr, "hash %x\n", hash);
        hashes[hash % MAX_HASH_COUNT] ++;
    }

    void *p = real_malloc(size);
    return p;
}

#ifndef __MALLOC_HOOK_VOLATILE
#define __MALLOC_HOOK_VOLATILE
#endif

void *(*__MALLOC_HOOK_VOLATILE __test_hook)(size_t size,
                                            const void *caller) = (void *)malloc;
