#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <numa.h>
#include <unistd.h>

#define GB (1024.0 * 1024.0 * 1024.0)

void print_stats() 
{
    long long size, freep;
    for (int i = 0; i < 8; i++) {
        size = numa_node_size64(i, &freep);
        if (size > 0) {
            printf("numa %d size: %.2f GB, used %.2f GB free %.2f GB\n", 
                i, size / GB, (size - freep) / GB, freep / GB);
        }
    }
}

int main(int argc, char **argv) {

    long long size, size2, freep, freep2;
    print_stats();

    printf("HARDCODED: alloc only from nodes 0 & 1\n");
    struct bitmask* bm = numa_bitmask_alloc(8);
    numa_bitmask_setbit(bm, 0);
    numa_bitmask_setbit(bm, 1);
    numa_set_membind(bm);

    long long num = atoll(argv[1]);
    if (argv[1][strlen(argv[1]) - 1] == 'G') {
        num *= GB;
    }

    if (num < 0) {
        num *= -1.0;

        printf("leave only %.2f GB bytes in nodes 0 & 1...\n", num / GB);

        size = numa_node_size64(0, &freep);
        size2 = numa_node_size64(1, &freep2);
        num = freep + freep2 - num;
    } 

    printf("malloc %.2f GB bytes...\n", num / GB);
    fflush(stdout);
    void* p = malloc(num);
    memset(p, 1, num);
    printf("done\n");

    print_stats();

    fprintf(stderr, "pause...\n");
    pause();
    free(p);
}
