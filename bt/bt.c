
#define _GNU_SOURCE

#include <stdio.h>
#include <dlfcn.h>
#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <pthread.h>
#include <stdarg.h>
#include <stdio.h>
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


UTILS_EXPORT void *a(size_t size);
UTILS_EXPORT void *b(size_t size);
UTILS_EXPORT void *c(size_t size);
UTILS_EXPORT void *d(size_t size);
UTILS_EXPORT void *e(size_t size);


UTILS_EXPORT void *a(size_t size)
{
    void *x = b(size);

    fprintf(stderr, "aaaaaaaaaaa");
    void *addr_re = __builtin_return_address (0);
    fprintf(stderr, "a re add 0: %p \n", addr_re);
    void *addr_reex = __builtin_extract_return_addr(__builtin_return_address(0));
    fprintf(stderr, "a re add ex 0: %p \n", addr_re);

    void *addr_re1 = __builtin_return_address (1);
    fprintf(stderr, "a re add 1: %p\n", addr_re1);
    void *addr_re2 = __builtin_return_address (2);
    fprintf(stderr, "a re add 2: %p\n", addr_re2);
    void *addr_re3 = __builtin_return_address (3);
    fprintf(stderr, "a re add 3: %p\n", addr_re3);
    void *addr_re4 = __builtin_return_address (4);
    fprintf(stderr, "a re add 4: %p\n", addr_re4);
    void *addr_re5 = __builtin_return_address (5);
    fprintf(stderr, "a re add 5: %p\n", addr_re5);
    void *addr_re6 = __builtin_return_address (6);
    fprintf(stderr, "a re add 6: %p\n", addr_re6);

    void *addr0 = __builtin_frame_address(0);
    fprintf(stderr, "a frame 0: %p \n", addr0);
    void *addr1 = __builtin_frame_address(1);
    fprintf(stderr, "a frame 1: %p \n", addr1);
    void *addr2 = __builtin_frame_address(2);
    fprintf(stderr, "a frame 2: %p \n", addr2);
    void *addr3 = __builtin_frame_address(3);
    fprintf(stderr, "a frame 3: %p \n", addr3);
    void *addr4 = __builtin_frame_address(4);
    fprintf(stderr, "a frame 4: %p \n", addr4);
    void *addr5 = __builtin_frame_address(5);
    fprintf(stderr, "a frame 5: %p \n", addr5);
    void *addr6 = __builtin_frame_address(6);
    fprintf(stderr, "a frame 6: %p \n", addr6);

    return x;
}

UTILS_EXPORT void *b(size_t size)
{
    void *x = c(size);

    fprintf(stderr, "bbbbbbbbbbbb");
    void *addr_re = __builtin_return_address (0);
    fprintf(stderr, "b re add 0: %p \n", addr_re);
    void *addr_reex = __builtin_extract_return_addr(__builtin_return_address(0));
    fprintf(stderr, "b re add ex 0: %p \n", addr_re);

    void *addr_re1 = __builtin_return_address (1);
    fprintf(stderr, "b re add 1: %p\n", addr_re1);
    void *addr_re2 = __builtin_return_address (2);
    fprintf(stderr, "b re add 2: %p\n", addr_re2);
    void *addr_re3 = __builtin_return_address (3);
    fprintf(stderr, "b re add 3: %p\n", addr_re3);
    void *addr_re4 = __builtin_return_address (4);
    fprintf(stderr, "b re add 4: %p\n", addr_re4);
    void *addr_re5 = __builtin_return_address (5);
    fprintf(stderr, "b re add 5: %p\n", addr_re5);
    void *addr_re6 = __builtin_return_address (6);
    fprintf(stderr, "b re add 6: %p\n", addr_re6);

    void *addr0 = __builtin_frame_address(0);
    fprintf(stderr, "b frame 0: %p \n", addr0);
    void *addr1 = __builtin_frame_address(1);
    fprintf(stderr, "b frame 1: %p \n", addr1);
    void *addr2 = __builtin_frame_address(2);
    fprintf(stderr, "b frame 2: %p \n", addr2);
    void *addr3 = __builtin_frame_address(3);
    fprintf(stderr, "b frame 3: %p \n", addr3);
    void *addr4 = __builtin_frame_address(4);
    fprintf(stderr, "b frame 4: %p \n", addr4);
    void *addr5 = __builtin_frame_address(5);
    fprintf(stderr, "b frame 5: %p \n", addr5);
    void *addr6 = __builtin_frame_address(6);
    fprintf(stderr, "b frame 6: %p \n", addr6);

    return x;
}

UTILS_EXPORT void *c(size_t size)
{
    void *x = d(size);

    fprintf(stderr, "cccccccccccccc");
    void *addr_re = __builtin_return_address (0);
    fprintf(stderr, "c re add 0: %p \n", addr_re);
    void *addr_reex = __builtin_extract_return_addr(__builtin_return_address(0));
    fprintf(stderr, "c re add ex 0: %p \n", addr_re);

    void *addr_re1 = __builtin_return_address (1);
    fprintf(stderr, "c re add 1: %p\n", addr_re1);
    void *addr_re2 = __builtin_return_address (2);
    fprintf(stderr, "c re add 2: %p\n", addr_re2);
    void *addr_re3 = __builtin_return_address (3);
    fprintf(stderr, "c re add 3: %p\n", addr_re3);
    void *addr_re4 = __builtin_return_address (4);
    fprintf(stderr, "c re add 4: %p\n", addr_re4);
    void *addr_re5 = __builtin_return_address (5);
    fprintf(stderr, "c re add 5: %p\n", addr_re5);
    void *addr_re6 = __builtin_return_address (6);
    fprintf(stderr, "c re add 6: %p\n", addr_re6);

    void *addr0 = __builtin_frame_address(0);
    fprintf(stderr, "c frame 0: %p \n", addr0);
    void *addr1 = __builtin_frame_address(1);
    fprintf(stderr, "c frame 1: %p \n", addr1);
    void *addr2 = __builtin_frame_address(2);
    fprintf(stderr, "c frame 2: %p \n", addr2);
    void *addr3 = __builtin_frame_address(3);
    fprintf(stderr, "c frame 3: %p \n", addr3);
    void *addr4 = __builtin_frame_address(4);
    fprintf(stderr, "c frame 4: %p \n", addr4);
    void *addr5 = __builtin_frame_address(5);
    fprintf(stderr, "c frame 5: %p \n", addr5);
    void *addr6 = __builtin_frame_address(6);
    fprintf(stderr, "c frame 6: %p \n", addr6);

    return x;
}
UTILS_EXPORT void *d(size_t size)
{
    void *x = e(size);

    fprintf(stderr, "dddddddddd");
    void *addr_re = __builtin_return_address (0);
    fprintf(stderr, "d re add 0: %p \n", addr_re);
    void *addr_reex = __builtin_extract_return_addr(__builtin_return_address(0));
    fprintf(stderr, "d re add ex 0: %p \n", addr_re);

    void *addr_re1 = __builtin_return_address (1);
    fprintf(stderr, "d re add 1: %p\n", addr_re1);
    void *addr_re2 = __builtin_return_address (2);
    fprintf(stderr, "d re add 2: %p\n", addr_re2);
    void *addr_re3 = __builtin_return_address (3);
    fprintf(stderr, "d re add 3: %p\n", addr_re3);
    void *addr_re4 = __builtin_return_address (4);
    fprintf(stderr, "d re add 4: %p\n", addr_re4);
    void *addr_re5 = __builtin_return_address (5);
    fprintf(stderr, "d re add 5: %p\n", addr_re5);
    void *addr_re6 = __builtin_return_address (6);
    fprintf(stderr, "d re add 6: %p\n", addr_re6);

    void *addr0 = __builtin_frame_address(0);
    fprintf(stderr, "d frame 0: %p \n", addr0);
    void *addr1 = __builtin_frame_address(1);
    fprintf(stderr, "d frame 1: %p \n", addr1);
    void *addr2 = __builtin_frame_address(2);
    fprintf(stderr, "d frame 2: %p \n", addr2);
    void *addr3 = __builtin_frame_address(3);
    fprintf(stderr, "d frame 3: %p \n", addr3);
    void *addr4 = __builtin_frame_address(4);
    fprintf(stderr, "d frame 4: %p \n", addr4);
    void *addr5 = __builtin_frame_address(5);
    fprintf(stderr, "d frame 5: %p \n", addr5);
    void *addr6 = __builtin_frame_address(6);
    fprintf(stderr, "d frame 6: %p \n", addr6);

    return x;
}
UTILS_EXPORT void *e(size_t size)
{
    int a = 4;
    int b = 5;

    fprintf(stderr, "eeeeee\n");
    void *addr_re = __builtin_return_address (0);
    fprintf(stderr, "e re add 0: %p &a: %p\n", addr_re, &a);
    void *addr_reex = __builtin_extract_return_addr(__builtin_return_address(0));
    fprintf(stderr, "e re add ex 0: %p \n", addr_reex);

    void *addr_re1 = __builtin_return_address (1);
    fprintf(stderr, "e re add 1: %p\n", addr_re1);
    void *addr_re2 = __builtin_return_address (2);
    fprintf(stderr, "e re add 2: %p\n", addr_re2);
    void *addr_re3 = __builtin_return_address (3);
    fprintf(stderr, "e re add 3: %p\n", addr_re3);
    void *addr_re4 = __builtin_return_address (4);
    fprintf(stderr, "e re add 4: %p\n", addr_re4);
    void *addr_re5 = __builtin_return_address (5);
    fprintf(stderr, "e re add 5: %p\n", addr_re5);

    void *addr0 = __builtin_frame_address(0);
    fprintf(stderr, "e frame 0: %p \n", addr0);
    void *addr1 = __builtin_frame_address(1);
    fprintf(stderr, "e frame 1: %p \n", addr1);
    void *addr2 = __builtin_frame_address(2);
    fprintf(stderr, "e frame 2: %p \n", addr2);
    void *addr3 = __builtin_frame_address(3);
    fprintf(stderr, "e frame 3: %p \n", addr3);
    void *addr4 = __builtin_frame_address(4);
    fprintf(stderr, "e frame 4: %p \n", addr4);
    void *addr5 = __builtin_frame_address(5);
    fprintf(stderr, "e frame 5: %p \n", addr5);

    //void *addr6 = __builtin_frame_address(6);
    //fprintf(stderr, "e frame 6: %p \n", addr6);

    return &a + b;
}

int xe(int val)
{
    int a = 4;
    void *ptr = &a;

    void *addr_re = __builtin_return_address (0);
    fprintf(stderr, "xe re add 0: %p &a: %p\n", addr_re, &a);
    void *addr_re1 = __builtin_return_address (1);
    fprintf(stderr, "xe re add 1: %p\n", addr_re1);

    int cnt = 20;
    while (cnt >= 0) {
        fprintf(stderr, "xe ptr = %p, *=%p\n", ptr, *(size_t*)ptr);
        cnt --;
        ptr += 4;
    }
}

int xd(int val)
{
    int xxd = 4;
    int xxd2 = 4;
    int xxd3 = 4;
    int xxd4 = 4;
    int xxd5 = 4;
    fprintf(stderr, "xd addr: %p\n", &xd);
    xxd = xe(xxd + xxd2 + xxd3 + xxd4 + xxd5);
    return xxd;
}

int xc(int val)
{
    int xxc = 4;
    int xxc2 = 4;
    int xxc3 = 4;
    fprintf(stderr, "xc addr: %p\n", &xc);
    xxc = xd(xxc + xxc2 + xxc3);
    return xxc;
}

int xb(int val)
{
    int xxb = 4;
    int xxb2 = 4;
    fprintf(stderr, "xb addr: %p\n", &xb);
    xxb = xc(xxb + xxb2);
    return xxb;
}

int xa(int val)
{
    int xxa = val;
    fprintf(stderr, "xa addr: %p\n", &xa);
    xxa = xb(xxa);
    return xxa;
}

static void* (*real_malloc)(size_t) = NULL;

static void mtrace_init(void)
{
    real_malloc = dlsym(RTLD_NEXT, "malloc");

    if (NULL == real_malloc) {
        fprintf(stderr, "Error in `dlsym`: %s\n", dlerror());
    }
}

void parse_proc_maps(int PID)
{
    static char c;
    static long pos = 0;
    /*
    fh = fopen(proc_stat_path, "r");
    if(fh == NULL) ...


    // Find the last ")" char in stat file and parse fields thereafter.
    #define RIGHTBRACKET ')'
    while(1)
    {
            c = fgetc(fh);
            if (c == EOF) break;
            if (c == RIGHTBRACKET) pos = ftell(fh);
    }
    fseek(fh, pos, 0);

    fscanf(fh, " %c %d %d" ..., &state, &ppid, ...);*/
}

void* pool_start[255];
void* pool_end[255];
int pool_num;
int max_bt_depth = 10;
int initialized = 0;

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
    unsigned int off;
    char sth[6] = {0};
    unsigned int sth2;
    char path[255] = {0};


    while(fgets(buffer, bufferLength, fh)) {

        num = sscanf(buffer, "%p-%p %c%c%c%c %p %s %d %s",
                          &start, &end, &r, &w, &x, &p,
                          &off, sth, &sth2, path);

        if ((num < 10) || (x != 'x')) {
            //fprintf(stderr, "num: %d x: %c\n", num, x);
            continue;
        }

        fprintf(stderr, buffer);
        pool_start[pool_num] = start;
        pool_end[pool_num] = end;
        pool_num++;
    }

    initialized = 1;
}

static pthread_once_t init_once = PTHREAD_ONCE_INIT;

UTILS_EXPORT void *malloc(size_t size)
{
    if (real_malloc == 0)
        mtrace_init();

    if (initialized) {
        unsigned char* stack = __builtin_frame_address(0);
        int num_calls = 0;
        while (num_calls < max_bt_depth) {
            int i;
            //fprintf(stderr, "looking for: %p\n", *((size_t*)stack));
            for (i = 0; i < pool_num; i++) {
                if (*((size_t*)stack) > pool_start[i] && *((size_t*)stack) < pool_end[i]) {
                    //fprintf(stderr, "call %p in %p-%p\n", stack, pool_start[i], pool_end[i]);
                    if (stack != NULL)
                        //fprintf(stderr, "found: %p ", *((size_t*)stack));
                        fprintf(stderr, "%p ", *((size_t*)stack));
                    num_calls ++;
                }
            }
            stack += 4;
        }
        fprintf(stderr, "\n");
    }

    void *p = real_malloc(size);
    return p;
}

#ifndef __MALLOC_HOOK_VOLATILE
#define __MALLOC_HOOK_VOLATILE
#endif

void *(*__MALLOC_HOOK_VOLATILE __test_hook)(size_t size,
                                            const void *caller) = (void *)malloc;
