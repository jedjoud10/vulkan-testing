#include <other.slang>

static const float3 sun = normalize(float3(1, 1, 1));

float3 light(float3 world, float3 uv, float3 normal) {
    float3 pixelated = floor(world * 8 + 0.001) / 8;
    float col = hash13(pixelated * float3(123.321, 21.322, -32.321));
    float mixer = col * 0.2 + 0.8;

    float3 grass = float3(26, 51, 30) / 255;
    float3 dirt = float3(59, 38, 25) / 255;

    return (normal.y > 0.5 ? grass : dirt) * mixer * (max(dot(normal, sun), 0) + 0.6);
}

float3 sky(float3 dir) {
    float3 sky1 = float3(52, 186, 235);
    float3 sky2 = float3(20, 45, 105);
    float3 color = lerp(sky1, sky2, dir.y) / 255.0;
    return color + pow(max(dot(dir, sun), 0), 100);
}