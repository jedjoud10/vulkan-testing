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

struct ao_solver {
    RWTexture3D<uint8_t> voxel;
    uint3 pos;
    float3 uv;
    int face;
    float3 sign;

    float test_ao(int3 offset, int target_face, float value) {
        if (sign.x < 0) {
            offset.x = -offset.x;
        }

        if (face == target_face) {
            if ((voxel[pos + offset] & 1) == 1) {
                return value;
            } else {
                return 1.0;
            }
        } else {
            return 0.0;
        }
    }

    float ao() {
        return 1.0;
        /*
        float x1 = test_ao(int3(-1, -1, 0), 0, uv.y);
        float x2 = test_ao(int3(-1, 1, 0), 0, 1-uv.y);
        float x3 = test_ao(int3(-1, 0, -1), 0, uv.z);
        float x4 = test_ao(int3(-1, 0, 1), 0, 1-uv.z);
        

        float x11 = test_ao(int3(-1, -1, -1), 0, min(uv.y + uv.z, 1));
        float x12 = test_ao(int3(-1, -1, 1), 0, min(uv.y + 1 - uv.z, 1));
        float x13 = test_ao(int3(-1, 1, -1), 0, min(1 - uv.y + uv.z, 1));
        float x14 = test_ao(int3(-1, 1, 1), 0, min(1 - uv.y + 1 - uv.z, 1));

        // return x1 * x2 * x3 * x4;
        return x11 * x12 * x13 * x14 * x1 * x2 * x3 * x4;
        */
    }
}