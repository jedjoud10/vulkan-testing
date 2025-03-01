[[vk::binding(0, 0)]]
RWTexture2D<float4> output;

[[vk::binding(0, 1)]]
Texture3D<uint> voxel;

static const float3 sun = normalize(float3(1, 1, 1));

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
    float3 sky1 = float3(52, 186, 235);
    float3 sky2 = float3(20, 45, 105);
    float3 color = lerp(sky1, sky2, dir.y) / 255.0;
    return color + pow(dot(dir, sun), 100);
}

[shader("compute")]
[numthreads(32, 32, 1)]
void main(uint3 id: SV_DispatchThreadID, uniform float2 screen, uniform matrix<float,4,4> mat, uniform float3 position) {
    float2 uvs = (float2)id.xy / screen;
    uvs *= 2.0;
    uvs -= 1.0;
    uvs.y = -uvs.y;
    uvs.x = -uvs.x;

    float3 ray_dir = normalize((mul(mat, float4(uvs, 1, 0))).xyz);
    float3 ray_pos = position;
    float3 floored_pos = floor(ray_pos);
    float3 inv_dir = 1 / ray_dir;
    float3 sign = sign(ray_dir);
    float3 side_dist = (floored_pos - ray_pos + 0.5 + 0.5 * sign);


    // TODO: for some reason creates artefacts?? am I missing a barrier??
    // output[id.xy] = float4(0);
    float3 color = 0.0;
    bool hit = false;
    int face;
    for (int i = 0; i < 128; i++) {
        if (floored_pos.y < sin(floored_pos.x + floored_pos.z) * 2) {
            /*
            if (floored_pos.x < 0) {
                hit = true;
                color = (fmod(abs(floored_pos), 5)) / 5;
                break;
            } else {
                ray_dir = reflect(ray_dir, float3(0, 1, 0));
                ray_dir = normalize(ray_dir);
            }
            */
            hit = true;
            color = (float)face / 3;
            break;
        }

        float3 reconst = side_dist * inv_dir;
        int3 eqs = select(min3(reconst.x, reconst.y, reconst.z) == reconst, 1, 0);
        face = firstbithigh(eqs.x | eqs.y << 1 | eqs.z << 2);
        floored_pos += sign * eqs;
        side_dist += sign * eqs;
        
        /*
        float dist = sdf(ray_pos);
        ray_pos += ray_dir * dist;

        if (ray_pos.y < 0.1) {
            ray_dir = reflect(ray_dir, float3(0, 1, 0));
            ray_dir += float3(sin(ray_pos.x * 10 + 0.2565), cos(ray_pos.y * 10 + 0.89684), sin(ray_pos.z * 10 - 0.211256)) * 0.04;
            ray_dir = normalize(ray_dir);
            ray_pos += ray_dir * 0.5f;
        }

        if (dist < 0.1) {
            hit = true;
            color = float3(dot(normal(ray_pos), sun));
            break;
        }
        */
    }

    if (!hit) {
        color = sky(ray_dir);
    }

    int depth = 5;
    color = clamp(color, 0, 1);
    uint3 compressed = (uint3)(color * (float)(1 << depth));
    float3 converted = (float3)compressed / (float)(1 << depth);

    output[id.xy] = float4(converted, 0);
}