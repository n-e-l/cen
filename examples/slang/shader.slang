[[vk::binding(0)]]
[[vk::image_format("rgba8")]]
RWTexture2D<float4> image;

float3 palette(float t, float3 a, float3 b, float3 c, float3 d)
{
    return a + b * cos(6.28318 * (c * t + d));
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    uint width, height;
    image.GetDimensions(width, height);

    int2 p = int2(id.xy);
    if (p.x >= int(width) || p.y >= int(height))
        return;

    float2 pos = float2(p) / float2(width, height) - 0.5;
    float t = 0.3 + 0.5 * length(pos);
    float3 color = palette(t,
        float3(0.5, 0.5, 0.2), float3(0.5, 0.5, 0.5),
        float3(2.0, 1.0, 0.0), float3(0.5, 0.20, 0.25));
    image[p] = float4(color, 1.0);
}
