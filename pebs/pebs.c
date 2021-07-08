/* sample_data_src.c  */
/* by Vince Weaver   vincent.weaver _at_ maine.edu */

/* An attempt to figure out the PERF_SAMPLE_DATA_SRC code */

/* In general this is only set on recent Intel CPUs with PEBS (?) */
/* It also has to be an event that supports it.                   */

#define _GNU_SOURCE 1

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <signal.h>

#include <asm/unistd.h>

#include <linux/perf_event.h>    /* Definition of PERF_* constants */
#include <linux/hw_breakpoint.h> /* Definition of HW_* constants */

#include <sys/syscall.h>         /* Definition of SYS_* constants */
#include <sys/mman.h>
#include <sys/ioctl.h>
#include <sys/prctl.h>

#define SAMPLE_FREQUENCY 100000

#define MMAP_DATA_SIZE 8

#define RAW_NONE	0
#define RAW_IBS_FETCH	1
#define RAW_IBS_OP	2

static int count_total=0;
static char *our_mmap;
static long long prev_head;
static int quiet;
static long long global_sample_type;
static long long global_sample_regs_user;

static int debug=0;
struct validate_values {
	int pid;
	int tid;
	int events;
	unsigned long branch_low;
	unsigned long branch_high;
};

// TODO: rmb?

#if defined(__KNC__)
#define rmb() __sync_synchronize()
#else
#define rmb() asm volatile("lfence":::"memory")
#endif

//#if defined (__x86_64)
//#define mb() 	asm volatile("mfence":::"memory")
//#endif

#define ARCH_UNKNOWN    -1
#define ARCH_X86        1
#define ARCH_X86_64     2
#define ARCH_POWER      3
#define ARCH_ARM        4
#define ARCH_ARM64      5

#define VENDOR_UNKNOWN -1
#define VENDOR_INTEL    1
#define VENDOR_AMD      2
#define VENDOR_IBM	3
#define VENDOR_ARM	4

#define PROCESSOR_UNKNOWN		-1
#define PROCESSOR_SANDYBRIDGE		12
#define PROCESSOR_IVYBRIDGE		20
#define PROCESSOR_KNIGHTSCORNER		21
#define PROCESSOR_SANDYBRIDGE_EP	22
#define PROCESSOR_IVYBRIDGE_EP		24
#define PROCESSOR_HASWELL		25
#define PROCESSOR_BROADWELL		28
#define PROCESSOR_HASWELL_EP		29
#define PROCESSOR_SKYLAKE		30
#define PROCESSOR_KABYLAKE		32

#define MAX_TEST_EVENTS 16

int perf_event_open(struct perf_event_attr *hw_event_uptr,
	pid_t pid, int cpu, int group_fd, unsigned long flags) {

	return syscall(__NR_perf_event_open,hw_event_uptr, pid, cpu,
		group_fd, flags);
}


static int dump_raw_ibs_fetch(unsigned char *data, int size) {

	unsigned long long *msrs;
	unsigned int *leftover;

	msrs=(unsigned long long *)(data+4);
	leftover=(unsigned int *)(data);

	printf("\t\tHeader: %x\n",leftover[0]);
	printf("\t\tMSR IBS_FETCH_CONTROL %llx\n",msrs[0]);
	printf("\t\t\tIBS_RAND_EN: %d\n",!!(msrs[0]&1ULL<<57));
	printf("\t\t\tL2 iTLB Miss: %d\n",!!(msrs[0]&1ULL<<56));
	printf("\t\t\tL1 iTLB Miss: %d\n",!!(msrs[0]&1ULL<<55));
	printf("\t\t\tL1TLB page size: ");
	switch( (msrs[0]>>53)&0x3) {
		case 0:	printf("4kB\n"); break;
		case 1:	printf("2MB\n"); break;
		case 2: printf("1GB\n"); break;
		default:	printf("Resreved\n"); break;
	}

	printf("\t\t\tFetch Physical Address Valid: %d\n",!!(msrs[0]&1ULL<<52));
	printf("\t\t\ticache miss: %d\n",!!(msrs[0]&1ULL<<51));
	printf("\t\t\tInstruction Fetch Complete: %d\n",!!(msrs[0]&1ULL<<50));
	printf("\t\t\tInstruction Fetch Valid: %d\n",!!(msrs[0]&1ULL<<49));
	printf("\t\t\tInstruction Fetch Enabled: %d\n",!!(msrs[0]&1ULL<<48));
	printf("\t\t\tInstruction Fetch Latency: %lld\n",((msrs[0]>>32)&0xffff));
	printf("\t\t\tInstruction Fetch Count: %lld\n",((msrs[0]>>16)&0xffff)<<4);
	printf("\t\t\tInstruction Fetch Max Count: %lld\n",(msrs[0]&0xffff)<<4);

	printf("\t\tMSR IBS_FETCH_LINEAR_ADDRESS %llx\n",msrs[1]);
	printf("\t\tMSR IBS_FETCH_PHYSICAL_ADDRESS %llx\n",msrs[2]);
	if (size>24) {
		printf("\t\tMSR IBS_BRTARGET %llx\n",msrs[3]);
	}
	return 0;
}

static int dump_raw_ibs_op(unsigned char *data, int size) {

	unsigned long long *msrs;
	unsigned int *leftover;

	msrs=(unsigned long long *)(data+4);
	leftover=(unsigned int *)(data);

	printf("\t\tHeader: %x\n",leftover[0]);
	printf("\t\tMSR IBS_EXECUTION_CONTROL %llx\n",msrs[0]);
	printf("\t\t\tIbsOpCurCnt: %lld\n",((msrs[0]>>32)&0x3ffffff));
	printf("\t\t\tIBS OpCntCtl: %d\n",!!(msrs[0]&1ULL<<19));
	printf("\t\t\tIBS OpVal: %d\n",!!(msrs[0]&1ULL<<18));
	printf("\t\t\tIBS OpEn: %d\n",!!(msrs[0]&1ULL<<17));
	printf("\t\t\tIbsOpMaxCnt: %lld\n",((msrs[0]&0xffff)<<4) |
				(msrs[0]&0x3f00000));

	printf("\t\tMSR IBS_OP_LOGICAL_ADDRESS %llx\n",msrs[1]);

	printf("\t\tMSR IBS_OP_DATA %llx\n",msrs[2]);
	printf("\t\t\tRIP Invalid: %d\n",!!(msrs[2]&1ULL<<38));
	printf("\t\t\tBranch Retired: %d\n",!!(msrs[2]&1ULL<<37));
	printf("\t\t\tBranch Mispredicted: %d\n",!!(msrs[2]&1ULL<<36));
	printf("\t\t\tBranch Taken: %d\n",!!(msrs[2]&1ULL<<35));
	printf("\t\t\tReturn uop: %d\n",!!(msrs[2]&1ULL<<34));
	printf("\t\t\tMispredicted Return uop: %d\n",!!(msrs[2]&1ULL<<33));
	printf("\t\t\tTag to Retire Cycles: %lld\n",(msrs[2]>>16)&0xffff);
	printf("\t\t\tCompletion to Retire Cycles: %lld\n",msrs[2]&0xffff);

	printf("\t\tMSR IBS_OP_DATA2 (Northbridge) %llx\n",msrs[3]);
	printf("\t\t\tCache Hit State: %c\n",(msrs[3]&1ULL<<5)?'O':'M');
	printf("\t\t\tRequest destination node: %s\n",
		(msrs[3]&1ULL<<4)?"Same":"Different");
	printf("\t\t\tNorthbridge data source: ");
	switch(msrs[3]&0x7) {
		case 0:	printf("No valid status\n"); break;
		case 1: printf("L3\n"); break;
		case 2: printf("Cache from another compute unit\n"); break;
		case 3: printf("DRAM\n"); break;
		case 4: printf("Reserved remote cache\n"); break;
		case 5: printf("Reserved\n"); break;
		case 6: printf("Reserved\n"); break;
		case 7: printf("Other: MMIO/config/PCI/APIC\n"); break;
	}

	printf("\t\tMSR IBS_OP_DATA3 (cache) %llx\n",msrs[4]);
	printf("\t\t\tData Cache Miss Latency: %lld\n",
		(msrs[4]>>32)&0xffff);
	printf("\t\t\tL2TLB data hit in 1GB page: %d\n",
		!!(msrs[4]&1ULL<<19));
	printf("\t\t\tData cache physical addr valid: %d\n",
		!!(msrs[4]&1ULL<<18));
	printf("\t\t\tData cache linear addr valid: %d\n",
		!!(msrs[4]&1ULL<<17));
	printf("\t\t\tMAB hit: %d\n",
		!!(msrs[4]&1ULL<<16));
	printf("\t\t\tData cache locked operation: %d\n",
		!!(msrs[4]&1ULL<<15));
	printf("\t\t\tUncachable memory operation: %d\n",
		!!(msrs[4]&1ULL<<14));
	printf("\t\t\tWrite-combining memory operation: %d\n",
		!!(msrs[4]&1ULL<<13));
	printf("\t\t\tData forwarding store to load canceled: %d\n",
		!!(msrs[4]&1ULL<<12));
	printf("\t\t\tData forwarding store to load operation: %d\n",
		!!(msrs[4]&1ULL<<11));
	printf("\t\t\tBank conflict on load operation: %d\n",
		!!(msrs[4]&1ULL<<9));
	printf("\t\t\tMisaligned access: %d\n",
		!!(msrs[4]&1ULL<<8));
	printf("\t\t\tData cache miss: %d\n",
		!!(msrs[4]&1ULL<<7));
	printf("\t\t\tData cache L2TLB hit in 2M: %d\n",
		!!(msrs[4]&1ULL<<6));
	printf("\t\t\tData cache L2TLB hit in 1G: %d\n",
		!!(msrs[4]&1ULL<<5));
	printf("\t\t\tData cache L1TLB hit in 2M: %d\n",
		!!(msrs[4]&1ULL<<4));
	printf("\t\t\tData cache L2TLB miss: %d\n",
		!!(msrs[4]&1ULL<<3));
	printf("\t\t\tData cache L1TLB miss: %d\n",
		!!(msrs[4]&1ULL<<2));
	printf("\t\t\tOperation is a store: %d\n",
		!!(msrs[4]&1ULL<<1));
	printf("\t\t\tOperation is a load: %d\n",
		!!(msrs[4]&1ULL<<0));

	if (msrs[4]&1ULL<<17) {
		printf("\t\tMSR IBS_DC_LINEAR_ADDRESS %llx\n",msrs[5]);
	}
	if (msrs[4]&1ULL<<18) {
		printf("\t\tMSR IBS_DC_PHYSICAL_ADDRESS %llx\n",msrs[6]);
	}

	if (size>64) {
		printf("\t\tMSR IBS_OP_DATA4 %llx\n",msrs[7]);
	}
	return 0;
}

long long perf_mmap_read( void *our_mmap, int mmap_size,
			long long prev_head,
			int sample_type, int read_format, long long reg_mask,
			struct validate_values *validate,
			int quiet, int *events_read,
			int raw_type ) {

	struct perf_event_mmap_page *control_page = our_mmap;
	long long head,offset;
	int i,size;
	long long bytesize,prev_head_wrap;

	unsigned char *data;

	void *data_mmap=our_mmap+getpagesize();

	if (mmap_size==0) return 0;

	if (control_page==NULL) {
		fprintf(stderr,"ERROR mmap page NULL\n");
		return -1;
	}

	head=control_page->data_head;
	rmb(); /* Must always follow read of data_head */

	size=head-prev_head;

	if (debug) {
		printf("Head: %lld Prev_head=%lld\n",head,prev_head);
		printf("%d new bytes\n",size);
	}

	bytesize=mmap_size*getpagesize();

	if (size>bytesize) {
		printf("error!  we overflowed the mmap buffer %d>%lld bytes\n",
			size,bytesize);
	}

	data=malloc(bytesize);
	if (data==NULL) {
		return -1;
	}

	if (debug) {
		printf("Allocated %lld bytes at %p\n",bytesize,data);
	}

	prev_head_wrap = prev_head % bytesize;

	if (debug) {
		printf("Copying %lld bytes from (%p)+%lld to (%p)+%d\n",
			  bytesize-prev_head_wrap,data_mmap,prev_head_wrap,data,0);
	}

	memcpy(data,(unsigned char*)data_mmap + prev_head_wrap,
		bytesize-prev_head_wrap);

	if (debug) {
		printf("Copying %lld bytes from %d to %lld\n",
			prev_head_wrap,0,bytesize-prev_head_wrap);
	}

	memcpy(data+(bytesize-prev_head_wrap),(unsigned char *)data_mmap,
		prev_head_wrap);

	struct perf_event_header *event;

	offset=0;
	if (events_read) *events_read=0;

	while(offset<size) {

		if (debug) printf("Offset %lld Size %d\n",offset,size);
		event = ( struct perf_event_header * ) & data[offset];

		/********************/
		/* Print event Type */
		/********************/

		if (!quiet) {
			switch(event->type) {
				case PERF_RECORD_SAMPLE:
					printf("PERF_RECORD_SAMPLE [%x]",sample_type);
					break;
				default: printf("UNKNOWN %d",event->type);
					break;
			}

			printf(", MISC=%d (",event->misc);
			switch(event->misc & PERF_RECORD_MISC_CPUMODE_MASK) {
				case PERF_RECORD_MISC_CPUMODE_UNKNOWN:
					printf("PERF_RECORD_MISC_CPUMODE_UNKNOWN"); break; 
				case PERF_RECORD_MISC_KERNEL:
					printf("PERF_RECORD_MISC_KERNEL"); break;
				case PERF_RECORD_MISC_USER:
					printf("PERF_RECORD_MISC_USER"); break;
				case PERF_RECORD_MISC_HYPERVISOR:
					printf("PERF_RECORD_MISC_HYPERVISOR"); break;
				case PERF_RECORD_MISC_GUEST_KERNEL:
					printf("PERF_RECORD_MISC_GUEST_KERNEL"); break;
				case PERF_RECORD_MISC_GUEST_USER:
					printf("PERF_RECORD_MISC_GUEST_USER"); break;
				default:
					printf("Unknown %d!\n",event->misc); break;
			}

			/* All three have the same value */
			if (event->misc & PERF_RECORD_MISC_MMAP_DATA) {
				if (event->type==PERF_RECORD_MMAP) {
					printf(",PERF_RECORD_MISC_MMAP_DATA ");
				}
				else if (event->type==PERF_RECORD_COMM) {
					printf(",PERF_RECORD_MISC_COMM_EXEC ");
				}
				else if ((event->type==PERF_RECORD_SWITCH) ||
					(event->type==PERF_RECORD_SWITCH_CPU_WIDE)) {
					printf(",PERF_RECORD_MISC_SWITCH_OUT ");
				}
				else {
					printf("UNKNOWN ALIAS!!! ");
				}
			}


			if (event->misc & PERF_RECORD_MISC_EXACT_IP) {
				printf(",PERF_RECORD_MISC_EXACT_IP ");
			}

			if (event->misc & PERF_RECORD_MISC_EXT_RESERVED) {
				printf(",PERF_RECORD_MISC_EXT_RESERVED ");
			}

			printf("), Size=%d\n",event->size);
		}

		offset+=8; /* skip header */

		/***********************/
		/* Print event Details */
		/***********************/

		switch(event->type) {

		/* Lost */
		case PERF_RECORD_LOST: {
			long long id,lost;
			memcpy(&id,&data[offset],sizeof(long long));
			if (!quiet) printf("\tID: %lld\n",id);
			offset+=8;
			memcpy(&lost,&data[offset],sizeof(long long));
			if (!quiet) printf("\tLOST: %lld\n",lost);
			offset+=8;
			}
			break;

		/* COMM */
		case PERF_RECORD_COMM: {
			int pid,tid,string_size;
			char *string;

			memcpy(&pid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPID: %d\n",pid);
			offset+=4;
			memcpy(&tid,&data[offset],sizeof(int));
			if (!quiet) printf("\tTID: %d\n",tid);
			offset+=4;

			/* FIXME: sample_id handling? */

			/* two ints plus the 64-bit header */
			string_size=event->size-16;
			string=calloc(string_size,sizeof(char));
			memcpy(string,&data[offset],string_size);
			if (!quiet) printf("\tcomm: %s\n",string);
			offset+=string_size;
			if (string) free(string);
			}
			break;

		/* Fork */
		case PERF_RECORD_FORK: {
			int pid,ppid,tid,ptid;
			long long fork_time;

			memcpy(&pid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPID: %d\n",pid);
			offset+=4;
			memcpy(&ppid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPPID: %d\n",ppid);
			offset+=4;
			memcpy(&tid,&data[offset],sizeof(int));
			if (!quiet) printf("\tTID: %d\n",tid);
			offset+=4;
			memcpy(&ptid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPTID: %d\n",ptid);
			offset+=4;
			memcpy(&fork_time,&data[offset],sizeof(long long));
			if (!quiet) printf("\tTime: %lld\n",fork_time);
			offset+=8;
			}
			break;

		/* mmap */
		case PERF_RECORD_MMAP: {
			int pid,tid,string_size;
			long long address,len,pgoff;
			char *filename;

			memcpy(&pid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPID: %d\n",pid);
			offset+=4;
			memcpy(&tid,&data[offset],sizeof(int));
			if (!quiet) printf("\tTID: %d\n",tid);
			offset+=4;
			memcpy(&address,&data[offset],sizeof(long long));
			if (!quiet) printf("\tAddress: %llx\n",address);
			offset+=8;
			memcpy(&len,&data[offset],sizeof(long long));
			if (!quiet) printf("\tLength: %llx\n",len);
			offset+=8;
			memcpy(&pgoff,&data[offset],sizeof(long long));
			if (!quiet) printf("\tPage Offset: %llx\n",pgoff);
			offset+=8;

			string_size=event->size-40;
			filename=calloc(string_size,sizeof(char));
			memcpy(filename,&data[offset],string_size);
			if (!quiet) printf("\tFilename: %s\n",filename);
			offset+=string_size;
			if (filename) free(filename);

			}
			break;

		/* mmap2 */
		case PERF_RECORD_MMAP2: {
			int pid,tid,string_size;
			long long address,len,pgoff;
			int major,minor;
			long long ino,ino_generation;
			int prot,flags;
			char *filename;

			memcpy(&pid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPID: %d\n",pid);
			offset+=4;
			memcpy(&tid,&data[offset],sizeof(int));
			if (!quiet) printf("\tTID: %d\n",tid);
			offset+=4;
			memcpy(&address,&data[offset],sizeof(long long));
			if (!quiet) printf("\tAddress: %llx\n",address);
			offset+=8;
			memcpy(&len,&data[offset],sizeof(long long));
			if (!quiet) printf("\tLength: %llx\n",len);
			offset+=8;
			memcpy(&pgoff,&data[offset],sizeof(long long));
			if (!quiet) printf("\tPage Offset: %llx\n",pgoff);
			offset+=8;
			memcpy(&major,&data[offset],sizeof(int));
			if (!quiet) printf("\tMajor: %d\n",major);
			offset+=4;
			memcpy(&minor,&data[offset],sizeof(int));
			if (!quiet) printf("\tMinor: %d\n",minor);
			offset+=4;
			memcpy(&ino,&data[offset],sizeof(long long));
			if (!quiet) printf("\tIno: %llx\n",ino);
			offset+=8;
			memcpy(&ino_generation,&data[offset],sizeof(long long));
			if (!quiet) printf("\tIno generation: %llx\n",ino_generation);
			offset+=8;
			memcpy(&prot,&data[offset],sizeof(int));
			if (!quiet) printf("\tProt: %d\n",prot);
			offset+=4;
			memcpy(&flags,&data[offset],sizeof(int));
			if (!quiet) printf("\tFlags: %d\n",flags);
			offset+=4;

			string_size=event->size-72;
			filename=calloc(string_size,sizeof(char));
			memcpy(filename,&data[offset],string_size);
			if (!quiet) printf("\tFilename: %s\n",filename);
			offset+=string_size;
			if (filename) free(filename);

			}
			break;

		/* Exit */
		case PERF_RECORD_EXIT: {
			int pid,ppid,tid,ptid;
			long long fork_time;

			memcpy(&pid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPID: %d\n",pid);
			offset+=4;
			memcpy(&ppid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPPID: %d\n",ppid);
			offset+=4;
			memcpy(&tid,&data[offset],sizeof(int));
			if (!quiet) printf("\tTID: %d\n",tid);
			offset+=4;
			memcpy(&ptid,&data[offset],sizeof(int));
			if (!quiet) printf("\tPTID: %d\n",ptid);
			offset+=4;
			memcpy(&fork_time,&data[offset],sizeof(long long));
			if (!quiet) printf("\tTime: %lld\n",fork_time);
			offset+=8;
			}
			break;

		/* Throttle/Unthrottle */
		case PERF_RECORD_THROTTLE:
		case PERF_RECORD_UNTHROTTLE: {
			long long throttle_time,id,stream_id;

			memcpy(&throttle_time,&data[offset],sizeof(long long));
			if (!quiet) printf("\tTime: %lld\n",throttle_time);
			offset+=8;
			memcpy(&id,&data[offset],sizeof(long long));
			if (!quiet) printf("\tID: %lld\n",id);
			offset+=8;
			memcpy(&stream_id,&data[offset],sizeof(long long));
			if (!quiet) printf("\tStream ID: %lld\n",stream_id);
			offset+=8;

			}
			break;

		/* Sample */
		case PERF_RECORD_SAMPLE:
			if (sample_type & PERF_SAMPLE_ADDR) {
				long long addr;
				memcpy(&addr,&data[offset],sizeof(long long));
				if (!quiet) printf("\tPERF_SAMPLE_ADDR, addr: %llx\n",addr);
				offset+=8;
			}
			if (sample_type & PERF_SAMPLE_RAW) {
				int size;

				memcpy(&size,&data[offset],sizeof(int));
				if (!quiet) printf("\tPERF_SAMPLE_RAW, Raw length: %d\n",size);
				offset+=4;

				if (!quiet) {
					if (raw_type==RAW_IBS_FETCH) {
						dump_raw_ibs_fetch(&data[offset],size);
					}
					else if (raw_type==RAW_IBS_OP) {
						dump_raw_ibs_op(&data[offset],size);
					}
					else {
						printf("\t\t");
						for(i=0;i<size;i++) {
							printf("%d ",data[offset+i]);
						}
						printf("\n");
					}
				}
				offset+=size;
			}

			if (sample_type & PERF_SAMPLE_DATA_SRC) {
				long long src;

				memcpy(&src,&data[offset],sizeof(long long));
				if (!quiet) printf("\tPERF_SAMPLE_DATA_SRC, Raw: %llx\n",src);
				offset+=8;

				if (!quiet) {
					if (src!=0) printf("\t\t");
					if (src & (PERF_MEM_OP_NA<<PERF_MEM_OP_SHIFT))
						printf("Op Not available ");
					if (src & (PERF_MEM_OP_LOAD<<PERF_MEM_OP_SHIFT))
						printf("Load ");
					if (src & (PERF_MEM_OP_STORE<<PERF_MEM_OP_SHIFT))
						printf("Store ");
					if (src & (PERF_MEM_OP_PFETCH<<PERF_MEM_OP_SHIFT))
						printf("Prefetch ");
					if (src & (PERF_MEM_OP_EXEC<<PERF_MEM_OP_SHIFT))
						printf("Executable code ");

					if (src & (PERF_MEM_LVL_NA<<PERF_MEM_LVL_SHIFT))
						printf("Level Not available ");
					if (src & (PERF_MEM_LVL_HIT<<PERF_MEM_LVL_SHIFT))
						printf("Hit ");
					if (src & (PERF_MEM_LVL_MISS<<PERF_MEM_LVL_SHIFT))
						printf("Miss ");
					if (src & (PERF_MEM_LVL_L1<<PERF_MEM_LVL_SHIFT))
						printf("L1 cache ");
					if (src & (PERF_MEM_LVL_LFB<<PERF_MEM_LVL_SHIFT))
						printf("Line fill buffer ");
					if (src & (PERF_MEM_LVL_L2<<PERF_MEM_LVL_SHIFT))
						printf("L2 cache ");
					if (src & (PERF_MEM_LVL_L3<<PERF_MEM_LVL_SHIFT))
						printf("L3 cache ");
					if (src & (PERF_MEM_LVL_LOC_RAM<<PERF_MEM_LVL_SHIFT))
						printf("Local DRAM ");
					if (src & (PERF_MEM_LVL_REM_RAM1<<PERF_MEM_LVL_SHIFT))
						printf("Remote DRAM 1 hop ");
					if (src & (PERF_MEM_LVL_REM_RAM2<<PERF_MEM_LVL_SHIFT))
						printf("Remote DRAM 2 hops ");
					if (src & (PERF_MEM_LVL_REM_CCE1<<PERF_MEM_LVL_SHIFT))
						printf("Remote cache 1 hop ");
					if (src & (PERF_MEM_LVL_REM_CCE2<<PERF_MEM_LVL_SHIFT))
						printf("Remote cache 2 hops ");
					if (src & (PERF_MEM_LVL_IO<<PERF_MEM_LVL_SHIFT))
						printf("I/O memory ");
					if (src & (PERF_MEM_LVL_UNC<<PERF_MEM_LVL_SHIFT))
						printf("Uncached memory ");

					if (src & (PERF_MEM_SNOOP_NA<<PERF_MEM_SNOOP_SHIFT))
						printf("Not available ");
					if (src & (PERF_MEM_SNOOP_NONE<<PERF_MEM_SNOOP_SHIFT))
						printf("No snoop ");
					if (src & (PERF_MEM_SNOOP_HIT<<PERF_MEM_SNOOP_SHIFT))
						printf("Snoop hit ");
					if (src & (PERF_MEM_SNOOP_MISS<<PERF_MEM_SNOOP_SHIFT))
						printf("Snoop miss ");
					if (src & (PERF_MEM_SNOOP_HITM<<PERF_MEM_SNOOP_SHIFT))
						printf("Snoop hit modified ");

					if (src & (PERF_MEM_LOCK_NA<<PERF_MEM_LOCK_SHIFT))
						printf("Not available ");
					if (src & (PERF_MEM_LOCK_LOCKED<<PERF_MEM_LOCK_SHIFT))
						printf("Locked transaction ");

					if (src & (PERF_MEM_TLB_NA<<PERF_MEM_TLB_SHIFT))
						printf("Not available ");
					if (src & (PERF_MEM_TLB_HIT<<PERF_MEM_TLB_SHIFT))
						printf("Hit ");
					if (src & (PERF_MEM_TLB_MISS<<PERF_MEM_TLB_SHIFT))
						printf("Miss ");
					if (src & (PERF_MEM_TLB_L1<<PERF_MEM_TLB_SHIFT))
						printf("Level 1 TLB ");
					if (src & (PERF_MEM_TLB_L2<<PERF_MEM_TLB_SHIFT))
						printf("Level 2 TLB ");
					if (src & (PERF_MEM_TLB_WK<<PERF_MEM_TLB_SHIFT))
						printf("Hardware walker ");
					if (src & ((long long)PERF_MEM_TLB_OS<<PERF_MEM_TLB_SHIFT))
						printf("OS fault handler ");
				}

				if (!quiet) printf("\n");
			}

			break;


		default:
			if (!quiet) printf("\tUnknown type %d\n",event->type);
			/* Probably best to just skip it all */
			offset=size;

		}
		if (events_read) (*events_read)++;
	}

//	mb();
	control_page->data_tail=head;

	free(data);

	return head;

}

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

static void our_handler(int signum, siginfo_t *info, void *uc) {

	int ret;

	int fd = info->si_fd;

	ret=ioctl(fd, PERF_EVENT_IOC_DISABLE, 0);

	prev_head=perf_mmap_read(our_mmap,MMAP_DATA_SIZE,prev_head,
		global_sample_type,0,global_sample_regs_user,
		NULL,quiet,NULL,RAW_NONE);

	count_total++;

	ret=ioctl(fd, PERF_EVENT_IOC_REFRESH, 1);

	(void) ret;

}

int main(int argc, char **argv) {

	int ret;
	int fd;
	int mmap_pages = 1 + MMAP_DATA_SIZE;
	char event_name[BUFSIZ];

	struct perf_event_attr pe;

	quiet=0;

	if (!quiet)
    printf("This tests PERF_SAMPLE_DATA_SRC samples\n");

	struct sigaction sa;
	memset(&sa, 0, sizeof(struct sigaction));
        sa.sa_sigaction = our_handler;
        sa.sa_flags = SA_SIGINFO;

  if (sigaction(SIGIO, &sa, NULL) < 0) {
          fprintf(stderr,"Error setting up signal handler\n");
          exit(1);
  }

	/* Set up Appropriate Event */
	memset(&pe,0,sizeof(struct perf_event_attr));

  // TODO get event coding from pfm_get_perf_event_encoding
  // https://man7.org/linux/man-pages/man3/pfm_get_perf_event_encoding.3.html
	pe.type=PERF_TYPE_RAW;
  pe.config=0x1cd;
  pe.config1=0x3;
  pe.precise_ip=2;
  strcpy(event_name, "MEM_UOPS_RETIRED:ALL_LOADS");
	if (!quiet) printf("Using event %s\n",event_name);

  pe.size=sizeof(struct perf_event_attr);
  pe.sample_period = SAMPLE_FREQUENCY;
  pe.sample_type = PERF_SAMPLE_ADDR;

  pe.read_format=0;
  pe.disabled=1;
  pe.pinned=1;
  pe.exclude_kernel=1;
  pe.exclude_hv=1;
  pe.wakeup_events=1;

	global_sample_type=pe.sample_type;

	fd = perf_event_open(&pe,0,-1,-1,0);
	if (fd < 0) {
		if (!quiet) {
			fprintf(stderr,"Problem opening leader %s\n", strerror(errno));
			//test_fail(test_string);
		}
	}

  int map_size = mmap_pages * getpagesize();
	our_mmap = mmap(NULL, map_size,
		PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);

	fcntl(fd, F_SETFL, O_RDWR | O_NONBLOCK | O_ASYNC);
	fcntl(fd, F_SETSIG, SIGIO);
	fcntl(fd, F_SETOWN, getpid());

	ioctl(fd, PERF_EVENT_IOC_RESET, 0);

	ret=ioctl(fd, PERF_EVENT_IOC_ENABLE,0);

	if (ret<0) {
		if (!quiet) {
			fprintf(stderr,"Error with PERF_EVENT_IOC_ENABLE "
				"of group leader: %d %s\n",
				errno,strerror(errno));
			exit(1);
		}
	}

  // code to test
	naive_matrix_multiply(quiet);

	ret = ioctl(fd, PERF_EVENT_IOC_REFRESH,0);

	if (!quiet) {
    printf("Counts %d, using mmap buffer %p\n",count_total,our_mmap);
  }

	if (count_total==0) {
		if (!quiet) printf("No overflow events generated.\n");
		//test_fail(test_string);
	}

	munmap(our_mmap, mmap_pages * getpagesize());

	close(fd);
  printf("PASS\n");

	return 0;
}
