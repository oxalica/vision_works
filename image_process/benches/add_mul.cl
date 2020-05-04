kernel void arr_add (global int *a, global int const *b) {
    size_t gid = get_global_id(0);
    a[gid] += b[gid];
}

kernel void mat_mul (
    int n,
    global int const *a,
    global int const *b,
    global int *out
) {
    int i = get_global_id(0);
    int j = get_global_id(1);
    int sum = 0;
    for (int k = 0; k < n; ++k)
        sum += a[i * n + k] * b[k * n + j];
    out[i * n + j] = sum;
}
