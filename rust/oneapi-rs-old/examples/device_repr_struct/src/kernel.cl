typedef struct ExampleData {
    uchar a[8];
    float b[8];
} ExampleData;

__kernel void sum_struct_fields(const ExampleData data, __global float *out) {
    float sum = 0.0f;

    for (int i = 0; i < 8; i++) {
        sum += (float)data.a[i];
        sum += data.b[i];
    }

    out[0] = sum;
}