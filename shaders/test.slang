[[vk::binding(0, 0)]]
RWTexture2D<float4> output;

[Differentiable]
float sdf(float3 pos) {
    return min(pos.y, length(pos) - 15 + sin(pos.x * 3.0) * 0.2f);
}

float3 normal(float3 pos) {
    DifferentialPair<float3> x = diffPair(pos, float3(1, 0, 0));
    DifferentialPair<float> x_res = fwd_diff(sdf)(x);
    DifferentialPair<float3> y = diffPair(pos, float3(0, 1, 0));
    DifferentialPair<float> y_res = fwd_diff(sdf)(y);
    DifferentialPair<float3> z = diffPair(pos, float3(0, 0, 1));
    DifferentialPair<float> z_res = fwd_diff(sdf)(z);
    return normalize(float3(x_res.d, y_res.d, z_res.d));
}

float3 sky(float3 dir) {
    return float3(1, 1, 1) * dir.y;
}

[shader("compute")]
[numthreads(32, 32, 1)]
void main(uint3 id: SV_DispatchThreadID, uniform float2 screen, uniform matrix<float,4,4> mat, uniform float3 position) {
    float2 uvs = (float2)id.xy / screen;
    uvs *= 2.0;
    uvs -= 1.0;
    uvs.y = -uvs.y;
    uvs.x = -uvs.x;

    float3 ray_dir = normalize(float3(uvs, -1.0));
    ray_dir = (mul(mat, float4(ray_dir, 0))).xyz;

    float3 ray_pos = position;

    output[id.xy] = float4(0);
    for (int i = 0; i < 64; i++) {
        float dist = sdf(ray_pos);
        ray_pos += ray_dir * dist;

        if (ray_pos.y < 0.1) {
            ray_dir = reflect(ray_dir, float3(0, 1, 0));
            ray_dir += float3(sin(ray_pos.x + 0.2565), cos(ray_pos.y + 0.89684), sin(ray_pos.z - 0.211256)) * 0.1;
            ray_dir = normalize(ray_dir);
            ray_pos += ray_dir * 0.5f;
        }

        if (dist < 0.1) {
            float3 color = float3(ray_pos.xz, 0);
            float3 temp;
            color = ray_pos / 10.0;
            //color = modf(color, float3(1, 1, 1)); 
            output[id.xy] = float4(normal(ray_pos), 0);
            return;
        }
    }

    output[id.xy] = float4(sky(ray_dir), 0);

    
}