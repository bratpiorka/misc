
#include <stdio.h>
#include <hwloc.h>
#include <numa.h>
#include <hwloc.h>

static void print_children(hwloc_topology_t topology, hwloc_obj_t obj, int depth)
{
    char string[128];
    unsigned i;

    hwloc_obj_type_snprintf(string, sizeof(string), obj, 1);
    printf("%*s%s\n", 2*depth, "", string);

    for (i = 0; i < obj->arity; i++) {
        print_children(topology, obj->children[i], depth + 1);
    }
}

int main (void)
{
    hwloc_topology_t topology;
    int err;
    int levels;
    char string[128];
    
    int num_numa_nodes = numa_num_configured_nodes();
    for (int i = 0; i < num_numa_nodes; i++)
    {
        long long size = numa_node_size64(i, NULL);
        printf("numactl id: %d, size: %lld\n", i, size);
    }

    err = hwloc_topology_init(&topology);
    assert(!err);
    err = hwloc_topology_load(topology);
    assert(!err);

    int topodepth = hwloc_topology_get_depth(topology);
        
    //printf("*** Printing overall tree\n");
    //print_children(topology, hwloc_get_root_obj(topology), 0);

    int nr = 100;
    hwloc_obj_t nodes[100] = {0};

    struct hwloc_location loc;
    loc.type = HWLOC_LOCATION_TYPE_OBJECT;
    loc.location.object = hwloc_get_root_obj(topology);

    hwloc_get_local_numanode_objs(topology, &loc, &nr, nodes, HWLOC_LOCAL_NUMANODE_FLAG_ALL);

    hwloc_cpuset_t cpuset = hwloc_bitmap_alloc();
    err = hwloc_get_cpubind (topology, cpuset, HWLOC_CPUBIND_THREAD);
    int depth = hwloc_get_nbobjs_by_type(topology, HWLOC_OBJ_NUMANODE);
    uint64_t val = -1;

    for (int i = 0; i < depth; i++)
    {
        hwloc_obj_t obj = hwloc_get_obj_by_type(topology, HWLOC_OBJ_NUMANODE, i);
        hwloc_memattr_get_value(topology, HWLOC_MEMATTR_ID_CAPACITY, obj, NULL, 0, &val);
        printf("hwloc id: %d, size: %lld\n", obj->os_index, val);
    }

    for (int i = 0; i < depth; i++)
    {
        hwloc_obj_t obj = hwloc_get_obj_by_type(topology, HWLOC_OBJ_NUMANODE, i);
        hwloc_memattr_get_value(topology, HWLOC_MEMATTR_ID_BANDWIDTH, obj, &loc, HWLOC_MEMATTR_FLAG_NEED_INITIATOR, &val);
        printf("hwloc id: %d, bw: %lld\n", obj->os_index, val);
    }

    for (int i = 0; i < depth; i++)
    {
        hwloc_obj_t obj = hwloc_get_obj_by_type(topology, HWLOC_OBJ_NUMANODE, i);
        hwloc_memattr_get_value(topology, HWLOC_MEMATTR_ID_LATENCY, obj, &loc, HWLOC_MEMATTR_FLAG_NEED_INITIATOR, &val);
        printf("hwloc id: %d, lat: %lld\n", obj->os_index, val);
    }

    hwloc_bitmap_free(cpuset);
    hwloc_topology_destroy(topology);

    return 0;
}
