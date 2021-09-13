
#define _GNU_SOURCE

#include <pthread.h>

// TODO remove unused headers
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <dirent.h>

#include <asm/unistd.h>
#include <unistd.h>

#include <linux/hw_breakpoint.h> /* Definition of HW_* constants */
#include <linux/perf_event.h>    /* Definition of PERF_* constants */

#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/syscall.h>         /* Definition of SYS_* constants */
#include <sys/types.h>

#include <perfmon/pfmlib.h>     // pfm_get_os_event_encoding
#include <perfmon/pfmlib_perf_event.h>


#define SAMPLE_FREQUENCY 100000000 // smaller value -> more frequent sampling
#define MMAP_DATA_SIZE   8
#define rmb() asm volatile("lfence":::"memory")

pthread_t pebs_thread;
int pebs_fd;
static char *pebs_mmap;

#define PROCESS_NAME "matmul"
#define BUF_SIZE 10000

extern char *program_invocation_name;

/*
// defined in perfmon/perf_event.h
int perf_event_open(struct perf_event_attr *hw_event_uptr, pid_t pid, int cpu,
                    int group_fd, unsigned long flags)
{
    return syscall(__NR_perf_event_open, hw_event_uptr, pid, cpu, group_fd,
                   flags);
}
*/

// used for tests
///*
#define MATRIX_SIZE 512
double *a = 0, *b = 0, *c = 0;
void naive_matrix_multiply(int quiet) {

    if (a == 0) a = malloc(sizeof(double)*MATRIX_SIZE*MATRIX_SIZE);
    if (b == 0) b = malloc(sizeof(double)*MATRIX_SIZE*MATRIX_SIZE);
    if (c == 0) c = malloc(sizeof(double)*MATRIX_SIZE*MATRIX_SIZE);

    printf("_ADDR: a %p b %p c %p\n", (void*)a, (void*)b, (void*)c);

    double s;
    int i,j,k;

    printf("_ADDR: s %p i %p j %p k %p\n", 
    (void*)&s, (void*)&i, (void*)&j, (void*)&k);

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

    s=0.0;
    for(i=0;i<MATRIX_SIZE;i++) {
        for(j=0;j<MATRIX_SIZE;j++) {
            s+=c[i * MATRIX_SIZE + j];
        }
    }

    if (!quiet) printf("Matrix multiply sum: s=%lf\n",s);

    return;
}
//*/

pid_t get_process_pid()
{
    char pidline[1024];
    char *pid;
    int i = 0;
    int pidno[64];
    pid_t ret_pid;

    FILE *fp = popen("pidof " PROCESS_NAME,"r");
    fgets(pidline, 1024, fp);

    printf("%s", pidline);
    pid = strtok(pidline," ");
    while (pid != NULL)
    {
        pidno[i] = atoi(pid);
        printf("pid of " PROCESS_NAME "= %d\n", pidno[i]);
        ret_pid = pidno[i];
        pid = strtok (NULL , " ");
        i++;
    }

    pclose(fp);

    return ret_pid;
}

void getPidByName(pid_t *pid, char *task_name)
{
    DIR *dir;
    struct dirent *ptr;
    FILE *fp;
    char filepath[5000];
    char cur_task_name[50];
    char buf[BUF_SIZE];

    dir = opendir("/proc");
    if (NULL != dir)
    {
        while ((ptr = readdir(dir)) != NULL) //Cycle read every file/folder under /proc
        {
            // Skip if it reads "." or "..", and skip if it reads not the folder name
            if ((strcmp(ptr->d_name, ".") == 0) || (strcmp(ptr->d_name, "..") == 0))
                continue;
            if (DT_DIR != ptr->d_type)
                continue;

            sprintf(filepath, "/proc/%s/status", ptr->d_name);//Generate the path of the file to be read
            //printf("filepath: %s\n", filepath);
            fp = fopen(filepath, "r");
            if (NULL != fp)
            {
                if( fgets(buf, BUF_SIZE-1, fp)== NULL ){
                    fclose(fp);
                    continue;
                }
                sscanf(buf, "%*s %s", cur_task_name);

                // If the file content meets the requirements, print the name of the path (that is, the PID of the process)
                if (!strcmp(task_name, cur_task_name)){
                    sscanf(ptr->d_name, "%d", pid);
                    printf("found pid: %d\n", *pid);
                }
                fclose(fp);
            }
        }
        closedir(dir);
    }
}

static void *pebs_monitor(void *a)
{
    // set low priority
    int policy;
    struct sched_param param;
    pthread_getschedparam(pthread_self(), &policy, &param);
    param.sched_priority = sched_get_priority_min(policy);
    pthread_setschedparam(pthread_self(), policy, &param);

    __u64 last_head = 0;

    // log to file
    static int pid;
    static int log_file;
    int cur_pid = getpid();
    int cur_tid = gettid();
    if (pid != cur_pid) {
        char name[255] = {0};
        sprintf(name, "tier_pid_%d_tid_%d.log", cur_pid, cur_tid);
        log_file = open(name, O_CREAT | O_WRONLY, S_IRWXU);
    }
    static char buf[4096];
    lseek(log_file, 0, SEEK_END);

    while (1) {
        struct perf_event_mmap_page* pebs_metadata =
            (struct perf_event_mmap_page*)pebs_mmap;

        // must call this before read from data head
		rmb();

        // DEBUG
        // printf("head: %llu size: %lld\n", pebs_metadata->data_head, pebs_metadata->data_head - last_head);
        if (last_head < pebs_metadata->data_head) {
            printf("new data!\n");

            while (last_head < pebs_metadata->data_head) {
	            char *data_mmap = pebs_mmap + getpagesize() + last_head % (MMAP_DATA_SIZE * getpagesize());

                struct perf_event_header *event =
                    (struct perf_event_header *)data_mmap;

                switch (event->type) {
                    case PERF_RECORD_SAMPLE:
                    {
                        // 'x' is the acessed address
                        char* x = data_mmap + sizeof(struct perf_event_header);
                        // DEBUG
                        sprintf(buf, "last: %llu, head: %llu x: %llx\n",
                            last_head, pebs_metadata->data_head, *(__u64*)x);
                        write(log_file, buf, strlen(buf));
                    }
                    break;
                default:
                    break;
                };

                last_head += event->size;
                data_mmap += event->size;
            }
        }

		ioctl(pebs_fd, PERF_EVENT_IOC_REFRESH, 0);
        last_head = pebs_metadata->data_head;
        pebs_metadata->data_tail = pebs_metadata->data_head;

        // sleep not working correctly?
		sleep(1);
    }

    return NULL;
}

void pebs_init()
{
    // TODO add code that writes to /proc/sys/kernel/perf_event_paranoid ?

    struct perf_event_attr pe;
    memset(&pe, 0, sizeof(struct perf_event_attr));

    // NOTE: code bellow requires link to libpfm
    ///*
    int ret = pfm_initialize();
    if (ret != PFM_SUCCESS) {
        printf("pfm_initialize() failed!\n");
        exit(-1);
    }

    pfm_perf_encode_arg_t arg;
    memset(&arg, 0, sizeof(arg));
    arg.attr = &pe;
    ret = pfm_get_os_event_encoding("MEM_UOPS_RETIRED:ALL_LOADS", PFM_PLM3,
        PFM_OS_PERF_EVENT_EXT, &arg);

    if (ret != PFM_SUCCESS) {
        printf("pfm_get_os_event_encoding() failed!\n");        
        printf("trying to use magic numbers!!!\n");
            
        pe.type=PERF_TYPE_RAW;
        pe.config=0x1cd;
        pe.config1=0x3;
        pe.precise_ip=2;
    }
    /*/

    ///*
	pe.type=PERF_TYPE_RAW;
    pe.config=0x1cd;
    pe.config1=0x3;
    pe.precise_ip=2;
    //*/

    pe.size = sizeof(struct perf_event_attr);
    pe.sample_period = SAMPLE_FREQUENCY;
    pe.sample_type = PERF_SAMPLE_ADDR;

    pe.precise_ip = 2;
    pe.read_format = 0;
    pe.disabled = 1;
    pe.pinned = 1;
    pe.exclude_kernel = 1;
    pe.exclude_hv = 1;
    pe.wakeup_events = 1;

    pid_t pid = 0;          // measure current process
    // pid_t pid = get_process_pid();
    getPidByName(&pid, PROCESS_NAME);
    //pid += 2;
    int cpu = -1;           // .. on any CPU

    // or any process on CPU #0
    //pid = -1;
    //cpu = 0;

    int group_fd = -1; // use single event group
    unsigned long flags = 0;
    pebs_fd = perf_event_open(&pe, pid, cpu, group_fd, flags);

    int mmap_pages = 1 + MMAP_DATA_SIZE;
    int map_size = mmap_pages * getpagesize();
    pebs_mmap = mmap(NULL, map_size,
                     PROT_READ | PROT_WRITE, MAP_SHARED, pebs_fd, 0);

    // DEBUG
    // printf("PEBS thread start for " PROCESS_NAME ", pid = %d\n", pid);
    printf("PEBS thread start\n");

    pthread_create(&pebs_thread, NULL, &pebs_monitor, NULL);

	ioctl(pebs_fd, PERF_EVENT_IOC_RESET, 0);
	ioctl(pebs_fd, PERF_EVENT_IOC_ENABLE, 0);

    // TEST
    /*
	for (int x = 0; x < 10000; x++) {
	    naive_matrix_multiply(0);
	}
    //*/

    for (int i = 0; i < 20; i++) {
        sleep(1);
        printf(".");
        fflush(stdout);
    }
}

void pebs_fini()
{
    ioctl(pebs_fd, PERF_EVENT_IOC_DISABLE, 0);

    int mmap_pages = 1 + MMAP_DATA_SIZE;
    munmap(pebs_mmap, mmap_pages * getpagesize());
    close(pebs_fd);

    // DEBUG
    printf("PEBS thread end\n");
}

int main(int argc, char **argv) {
	pebs_init();
	pebs_fini();
}
