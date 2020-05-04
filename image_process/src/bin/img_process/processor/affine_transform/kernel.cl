float4 interpolate3 (float xx, float4 y0, float4 y1, float4 y2, float4 y3) {
    const float x0 = 0.f, x1 = 1.f, x2 = 2.f, x3 = 3.f;
    return
        (xx - x1) * (xx - x2) * (xx - x3) / ((x0 - x1) * (x0 - x2) * (x0 - x3)) * y0 +
        (xx - x0) * (xx - x2) * (xx - x3) / ((x1 - x0) * (x1 - x2) * (x1 - x3)) * y1 +
        (xx - x0) * (xx - x1) * (xx - x3) / ((x2 - x0) * (x2 - x1) * (x2 - x3)) * y2 +
        (xx - x0) * (xx - x1) * (xx - x2) / ((x3 - x0) * (x3 - x1) * (x3 - x2)) * y3;
}

const sampler_t sampler_const =
    CLK_NORMALIZED_COORDS_FALSE |
    CLK_ADDRESS_CLAMP |
    CLK_FILTER_NEAREST;

kernel void affine_transform (
/* mat = [ s0 s1 s2 ]
 *       [ s3 s4 s5 ]
 */
    float8 mat,
    read_only image2d_t img,
    write_only image2d_t out
) {
    // Axis in OpenCL is flipped.
    int2 coord = (int2)(get_global_id(1), get_global_id(0));

    float2 src_coord = (float2)(
        coord.x * mat.s0 + coord.y * mat.s1 + mat.s2,
        coord.x * mat.s3 + coord.y * mat.s4 + mat.s5
    );
    int2 src_top_left = convert_int2_rtn(src_coord) - (int2)(1, 1);
    float2 relative = src_coord - convert_float2(src_top_left);

#define P(i, j) read_imagef(img, sampler_const, (src_top_left + (int2)(i, j)).yx)
    float4 val = interpolate3(
        relative.x,
        interpolate3(relative.y, P(0, 0), P(0, 1), P(0, 2), P(0, 3)),
        interpolate3(relative.y, P(1, 0), P(1, 1), P(1, 2), P(1, 3)),
        interpolate3(relative.y, P(2, 0), P(2, 1), P(2, 2), P(2, 3)),
        interpolate3(relative.y, P(3, 0), P(3, 1), P(3, 2), P(3, 3))
    );
#undef P

    write_imagef(out, coord.yx, val);
}
