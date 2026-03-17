__kernel void sin_kernel(__global float *out, __global const float *inp, int numel) {
    int i = get_global_id(0);
    if (i < numel) {
        out[i] = sin(inp[i]);
    }
}