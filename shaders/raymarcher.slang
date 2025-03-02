#include <lighting.slang>

[[vk::binding(0, 0)]]
RWTexture2D<float4> output;

[[vk::binding(1, 0)]]
RWTexture3D<uint8_t> voxel;

static const int size = 256;

[Differentiable]
float sdf(float3 pos) {
    return min(pos.y, length(pos) - 15 + sin(pos.x * 3.0) * 0.6f);
}

float3 normal(int face, float3 sign) {
    return -(float3)(face == int3(0,1,2)) * sign;
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
    float3 dir_sign = sign(ray_dir);
    float3 side_dist = (floored_pos - ray_pos + 0.5 + 0.5 * dir_sign);


    // TODO: for some reason creates artefacts?? am I missing a barrier??
    // output[id.xy] = float4(0);
    float3 color = 0.0;
    bool hit = false;
    int reflections = 0;
    int face = 0;
    for (int i = 0; i < 64; i++) {
        uint8_t raw = 0;

        if (all(floored_pos >= 0) && any(floored_pos < size)) {
            raw = voxel[(uint3)floored_pos];
        }

        bool active = (raw & 1) == 1;
        bool reflective = ((raw >> 1) & 1) == 1;
        bool refractive = ((raw >> 2) & 1) == 1;

        if (active) {
            float3 test = (floored_pos - ray_pos + 0.5 - 0.5 * dir_sign) * inv_dir;
            float max = max3(test.x, test.y, test.z);
            float3 world = ray_pos + ray_dir * max;
            float3 uv = world - floored_pos;
            float3 normal = normal(face, dir_sign);

            if (refractive || reflective) {
                normal += float3(sin(world.x * 10 + 0.2565), cos(world.y * 10 + 0.89684), sin(world.z * 10 - 0.211256)) * 0.04;

                if (refractive) {
                    ray_dir = refract(ray_dir, normalize(normal), 0.9);
                } else {
                    ray_dir = reflect(ray_dir, normalize(normal));
                }
                
                inv_dir = 1 / ray_dir;
                dir_sign = sign(ray_dir);
                side_dist = (floored_pos - world + 0.5 + 0.5 * dir_sign);
                ray_pos = world;
                reflections++;
            } else {
                hit = true;

                ao_solver solver;
                solver.voxel = voxel;
                solver.pos = (uint3)floored_pos;
                solver.uv = uv;
                solver.face = face;
                solver.sign = dir_sign;
                color = light(world, uv, normal) * solver.ao();
                //color = solver.ao();
                break;
            }
        }

        float3 reconst = side_dist * inv_dir;
        int3 eqs = select(min3(reconst.x, reconst.y, reconst.z) == reconst, 1, 0);
        face = firstbithigh(eqs.x | eqs.y << 1 | eqs.z << 2);
        floored_pos += dir_sign * eqs;
        side_dist += dir_sign * eqs;
    }

    if (!hit) {
        color = sky(ray_dir);
    }

    color /= max(reflections, 1);

    int depth = 8;
    color = clamp(color, 0, 1);
    uint3 compressed = (uint3)(color * (float)(1 << depth));
    float3 converted = (float3)compressed / (float)(1 << depth);

    output[id.xy] = float4(converted, 0);
}