

#include <stdio.h>
#include "../../memkind/include/memkind.h"


int main(int argc, char **argv) {

    int aval = memkind_check_available(MEMKIND_DEFAULT);
    if (aval != MEMKIND_SUCCESS) return -1;

    for (int dd = 0; dd < 100; dd++) {
        void* ppp = memkind_malloc(MEMKIND_DEFAULT, 10);
        printf("%p\n", ppp);
    }
}
