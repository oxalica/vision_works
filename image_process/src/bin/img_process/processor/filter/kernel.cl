const sampler_t sampler_const =
    CLK_NORMALIZED_COORDS_FALSE |
    CLK_ADDRESS_CLAMP |
    CLK_FILTER_NEAREST;

kernel void linear_transform (
    // Covariance kernel.
    read_only image2d_t knrl,
    read_only image2d_t img,
    write_only image2d_t out
) {
    // Axis in OpenCL is flipped.
    int2 coord = (int2)(get_global_id(1), get_global_id(0));
    // Kernel is square.
    int kernel_radius = get_image_height(knrl) / 2;

    float4 sum = (float4)(0.f, 0.f, 0.f, 0.f);
    for (int i = -kernel_radius; i < kernel_radius; ++i)
        for (int j = -kernel_radius; j < kernel_radius; ++j) {
            float4 v = read_imagef(img, sampler_const, (coord + (int2)(i, j)).yx);
            float4 w = read_imagef(knrl, sampler_const, (kernel_radius + (int2)(i, j)).yx);
            sum += v * w;
        }

    write_imagef(out, coord.yx, sum);
}
