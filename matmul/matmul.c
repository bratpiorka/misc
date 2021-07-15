
#include <pthread.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <dirent.h>

#define MATRIX_SIZE 512
double *a = 0, *b = 0, *c = 0;
double s;
int i,j,k;

void naive_matrix_multiply() {
    for(i=0;i<MATRIX_SIZE;i++) {
        for(j=0;j<MATRIX_SIZE;j++) {
            a[i * MATRIX_SIZE + j]=(double)i*(double)j;
            b[i * MATRIX_SIZE + j]=(double)i/(double)(j+5);
        }
    }

    for(j=0;j<MATRIX_SIZE;j++) {
        for(i=0;i<MATRIX_SIZE;i++) {
            s=0;
            for(k=0;k<MATRIX_SIZE;k++) {
                s+=a[i * MATRIX_SIZE + k]*b[k * MATRIX_SIZE + j];
            }
            c[i * MATRIX_SIZE + j] = s;
        }
    }

    s = 0.0;
    for(i=0;i<MATRIX_SIZE;i++) {
        for(j=0;j<MATRIX_SIZE;j++) {
            s+=c[i * MATRIX_SIZE + j];
        }
    }
    return;
}


int main(int argc, char **argv) {

    int mat_size = sizeof(double)*MATRIX_SIZE*MATRIX_SIZE;
    a = malloc(mat_size);
    b = malloc(mat_size);
    c = malloc(mat_size);

    printf("ADDR a: %p - %p\n", (void*)a, (void*)(a + mat_size));
    printf("ADDR b: %p - %p\n", (void*)b, (void*)(b + mat_size));
    printf("ADDR c: %p - %p\n", (void*)c, (void*)(c + mat_size));
    printf("ADDR s: %p i: %p j: %p k: %p\n",
        (void*)&s, (void*)&i, (void*)&j, (void*)&k);

	for (int it = 0; it < 10000; it++) {
	    naive_matrix_multiply();
        printf(".");
        fflush(stdout);
	}
}
