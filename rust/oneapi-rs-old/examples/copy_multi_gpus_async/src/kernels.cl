__kernel void init_values(__global uint* output, uint n) {
    size_t index = get_global_id(0);
    if (index < (size_t)n) {
        output[index] = (uint)index;
    }
}

__kernel void add_one(__global uint* values, uint n) {
    size_t index = get_global_id(0);
    if (index < (size_t)n) {
        values[index] += 1u;
    }
}