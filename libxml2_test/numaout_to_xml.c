
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <assert.h>

#include <libxml/xmlwriter.h>

enum interconnect_type
{
    INTERCONNECTS_TYPE_INVALID = -1,
    INTERCONNECTS_TYPE_ACCESS = 0,
};

enum unit_type
{
    UNIT_TYPE_INVALID = -1,
    UNIT_TYPE_KIB = 0,
    UNIT_TYPE_MIB,
};

static char* unit_type_str[] = { "KiB", "MiB" };
static char* unit_type_to_str(enum unit_type type)
{
    assert(type > UNIT_TYPE_INVALID);
    return unit_type_str[(int)type];
}

enum domain_type
{
    DOMAIN_TYPE_INVALID = -1,
    DOMAIN_TYPE_QEMU = 0,
};

struct sibling_element
{
    int id;
    int value;
};

void xml_write_sibling_element(xmlTextWriterPtr writer, struct sibling_element* sibling)
{
    char buf[10];

    xmlTextWriterStartElement(writer, "sibling");

    sprintf(buf, "%d", sibling->id);
    xmlTextWriterWriteAttribute(writer, "id", buf);

    sprintf(buf, "%d", sibling->value);
    xmlTextWriterWriteAttribute(writer, "value", buf);

    xmlTextWriterEndElement(writer);
}

struct distances_element
{
    struct sibling_element* siblings;
    int siblings_num;
};

void xml_write_distances_element(xmlTextWriterPtr writer, struct distances_element* distances)
{
    xmlTextWriterStartElement(writer, "distances");
    
    for (int i = 0; i < distances->siblings_num; i++)
    {
        xml_write_sibling_element(writer, &distances->siblings[i]);
    }

    xmlTextWriterEndElement(writer);
}

struct cell_element
{
    int id;
    int* cpus;
    int cpus_num;
    int memory;
    enum unit_type unit;
    struct distances_element distances;
};

void xml_write_cell_element(xmlTextWriterPtr writer, struct cell_element* cell)
{
    xmlTextWriterStartElement(writer, "cell");

    char buf[10];
    sprintf(buf, "%d", cell->id);
    xmlTextWriterWriteAttribute(writer, "id", buf);    
    
    if (cell->cpus_num)
    {
        // write cpus string
        char cpus_string[1024] = {0};
        char* cpus_string_pos = cpus_string;
        int i = 0;
        while (i < cell->cpus_num)
        {
            int start_cpu = cell->cpus[i];
            int curr = i + 1;

            while ((curr < cell->cpus_num) && ((start_cpu + (curr - i))) == cell->cpus[curr])
                curr ++;
            
            if (start_cpu < cell->cpus[curr - 1])
                cpus_string_pos += sprintf(cpus_string_pos, "%d-%d", start_cpu, cell->cpus[curr - 1]);
            else
                cpus_string_pos += sprintf(cpus_string_pos, "%d", start_cpu, cell->cpus[curr - 1]);

            i = curr;

            if (i < cell->cpus_num)
                cpus_string_pos += sprintf(cpus_string_pos, ",");
        }
    
        xmlTextWriterWriteAttribute(writer, "cpus", cpus_string); 
    }

    sprintf(buf, "%d", cell->memory); 
    xmlTextWriterWriteAttribute(writer, "memory", buf);
    xmlTextWriterWriteAttribute(writer, "unit", unit_type_to_str(cell->unit));
    
    xml_write_distances_element(writer, &cell->distances);

    xmlTextWriterEndElement(writer);
}

struct latency_element
{
    int initiator;
    int target;
    enum interconnect_type type;
    int value;
};

struct bandwidth_element
{
    int initiator;
    int target;
    enum interconnect_type type;
    int value;
    enum unit_type unit;
};

struct interconnects_element
{
    struct latency_element* latencies;
    int latencies_num;
    struct bandwidth_element* bandwidths;
    int bandwidths_num;
};

struct numa_element
{
    struct cell_element* cells;
    int cells_num;
    struct interconnect_element* interconnects;
    int interconnects_num;
};

void xml_write_numa_element(xmlTextWriterPtr writer, struct numa_element* numa)
{
    xmlTextWriterStartElement(writer, "numa");

    for (int i = 0; i < numa->cells_num; i++)
        xml_write_cell_element(writer, &numa->cells[i]);
        
  //  for (int i = 0; i < numa->cells_num; i++)
  //      xml_write_interconnect_element(writer, &numa->interconnects[i]);

    xmlTextWriterEndElement(writer);
}

struct cpu_element
{
    struct numa_element numa; // only one
};

void xml_write_cpu_element(xmlTextWriterPtr writer, struct cpu_element* cpu)
{
    xmlTextWriterStartElement(writer, "cpu");
    xml_write_numa_element(writer, &cpu->numa);
    xmlTextWriterEndElement(writer);
}

struct domain_element
{
    char* name;
    enum domain_type type;
    struct cpu_element cpu; // only one
};

void xml_write_domain_element(xmlTextWriterPtr writer, struct domain_element* domain)
{    
    xmlTextWriterStartElement(writer, "domain");
    
    switch(domain->type)
    {
        case DOMAIN_TYPE_QEMU:
            xmlTextWriterWriteAttribute(writer, "type", "qemu");
            break;
        default:
            assert(0);
    }
    
    xmlTextWriterWriteElement(writer, "name", domain->name);
    xml_write_cpu_element(writer, &domain->cpu);
    xmlTextWriterEndElement(writer);
}

int main(int argc, char **argv) 
{
    char input_file_name[] = "/home/rrudnick/misc/libxml2_test/numactl_out.txt";
    
    // read input string 
    FILE *fp = fopen(input_file_name, "r");   
    fseek(fp, 0, SEEK_END);
    long fsize = ftell(fp);
    fseek(fp, 0, SEEK_SET); 
    
    char *input_string = malloc(fsize + 1);
    fread(input_string, sizeof(char), fsize, fp);

    fclose(fp);

    // parse input and build tree
    struct domain_element* domain = (struct domain_element*)malloc(sizeof(struct domain_element));
    domain->name = (char*)malloc(sizeof("test"));
    domain->type = DOMAIN_TYPE_QEMU;
    strcpy(domain->name, "test");

    struct numa_element* numa = &domain->cpu.numa;
  
    char* pos = input_string;

    // parse "available: 1 nodes (0)"
    int num_nodes;
    char nodes_ids[10] = {0};
    int num_chars_read;
    sscanf(pos, "available: %d nodes (%[^)])\n%n", &num_nodes, nodes_ids, &num_chars_read);
    pos += num_chars_read;    

    numa->cells_num = num_nodes;
    numa->cells = (struct cell_element*)malloc(sizeof(struct cell_element) * num_nodes);

    for (int i = 0; i < num_nodes; i++)
    {
        // parse "node 0 cpus: 0 1 2 3 4 5 6 7 32 33 34 35 36 37 38 39"

        int node_id;
        int cpus_num = 0;
        char cpu_list[1024] = {0};
        int cpu_ids[1024] = {0};
        int node_size;
        char size_unit[10] = {0};
        int node_free;
        char free_unit[10] = {0};
        num_chars_read = 0;

        int elem = sscanf(pos, "node %d cpus:%[^\n]\n%n", &node_id, cpu_list, &num_chars_read);
        if (elem != 2)
        {
            // no cpus
            sscanf(pos, "node %d cpus:\n%n", &node_id, &num_chars_read);
            pos += num_chars_read;
        }
        else
        {
            pos += num_chars_read;
            char* cpus_list_ptr = cpu_list;
            while (*cpus_list_ptr != '\n')
            {   
                int cpu_id;         
                int ok = sscanf(cpus_list_ptr, "%d%n", &cpu_id, &num_chars_read);
                if (ok > 0)
                {
                    cpu_ids[cpus_num++] = cpu_id;
                    cpus_list_ptr += num_chars_read;
                }
                else
                    break;
            }
        }
        
        numa->cells[i].id = node_id;
        numa->cells[i].cpus = (int*)malloc(sizeof(int) * cpus_num);
        memcpy(numa->cells[i].cpus, cpu_ids, sizeof(int) * cpus_num);
        numa->cells[i].cpus_num = cpus_num;

        // parse "node 0 size: 31871 MB"
        
        sscanf(pos, "node %d size: %d %[^\n]\n%n", &node_id, &node_size, size_unit, &num_chars_read);
        pos += num_chars_read;
        
        assert(numa->cells[i].id == node_id);
        numa->cells[i].memory = node_size;

        if (strncmp(size_unit, "MB", sizeof("MB")) == 0)
            numa->cells[i].unit = UNIT_TYPE_MIB;
        else
            assert(0);
            
        // parse "node 0 free: 13349 MB"

        sscanf(pos, "node %d size: %d %s%n", &node_id, &node_free, free_unit, &num_chars_read);
        pos += num_chars_read;
        // NOTE: currently free is not used
    }

    // parse "node distances:""
    char distances[1024] = {0};
    sscanf(pos, "node distances:\n%n", &num_chars_read);
    pos += num_chars_read;

    // parse distances header, e.g. "node   0   1   2   3   4   5   6"
    sscanf(pos, "node%[^\n]\n%n", distances, &num_chars_read);
    pos += num_chars_read;

    // parse distances
    for (int i = 0; i < num_nodes; i++)
    {
        // parse "  0:  10  21  31  21  17  38  28 "
        int node_id;    
        sscanf(pos, "%d: %[^\n]\n%n", &node_id, distances, &num_chars_read);
        pos += num_chars_read;
        
        numa->cells[i].distances.siblings = 
            (struct sibling_element*)malloc(sizeof(struct sibling_element) * num_nodes);
        numa->cells[i].distances.siblings_num = num_nodes;

        char* distances_pos = distances;
        for (int j = 0; j < num_nodes; j++)
        {            
            struct sibling_element* sibling = &numa->cells[i].distances.siblings[j];
            sibling->id = j;
            sscanf(distances_pos, "%d%n", &sibling->value, &num_chars_read);
            distances_pos += num_chars_read;
        }
    }

    xmlTextWriterPtr writer;
    writer = xmlNewTextWriterFilename("test.xml", 0);

    xmlTextWriterStartDocument(writer, NULL, "UTF-8", NULL);

    xml_write_domain_element(writer, domain);

    xmlTextWriterEndDocument(writer);

    xmlFreeTextWriter(writer);    
}
