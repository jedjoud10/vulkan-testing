#ifndef OTHER
#define OTHER

static const int size = 256;

// https://www.shadertoy.com/view/4djSRW
float hash12(float2 p) {
    float3 p3  = fract(float3(p.xyx) * .1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float hash13(float3 p3)
{
    p3 = fract(p3 * .1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

float3 hash33(float3 p3)
{
    p3 = fract(p3 * float3(.1031, .1030, .0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

float noise(float2 p) {
    float zz = hash12(floor(p));
    float zo = hash12(floor(p) + float2(1, 0));
    float oz = hash12(floor(p) + float2(0, 1));
    float oo = hash12(floor(p) + float2(1, 1));

    float2 uv = frac(p);
    return lerp(lerp(zz, zo, uv.x), lerp(oz, oo, uv.x), uv.y);
}

struct Voxel {
    bool active;
    bool reflective;
    bool refractive;

    uint8_t into_raw() {
        uint8_t raw = 0;
        raw |= active ? 1 : 0;
        raw |= reflective ? 2 : 0;
        raw |= refractive ? 4 : 0;
        return raw;
    }

    static Voxel from_raw(uint8_t raw) {
        Voxel voxel;
        voxel.active = (raw & 1) == 1;
        voxel.reflective = ((raw >> 1) & 1) == 1;
        voxel.refractive = ((raw >> 2) & 1) == 1;
        return voxel;
    }
}

struct Fetcher {
    RWTexture3D<uint8_t> voxels;
    int max_size;

    Voxel fetch(int3 position) {
        uint8_t raw = 0;

        if (all(position >= 0) && all(position < max_size)) {
            raw = voxels[position];
        }
        
        return Voxel.from_raw(raw);
    }
}

float3 dda_shadownate(
    RWTexture3D<uint8_t> voxels,
    float3 ray_dir,
    float3 ray_pos,
    float base_shadow,
) {
    float3 floored_pos = ceil(ray_pos);
    float3 inv_dir = 1 / ray_dir;
    float3 dir_sign = sign(ray_dir);
    float3 side_dist = (floored_pos - ray_pos + 0.5 + 0.5 * dir_sign);
    Fetcher fetcher = Fetcher(voxels, size);
    float3 color = 1.0;

    for (int i = 0; i < 32; i++) {
        Voxel voxel = fetcher.fetch((int3)(floored_pos) - 1);

        if (voxel.active && voxel.refractive) {
            color *= normalize(hash33(floor(floored_pos) * float3(23.231, -435.4354, 9412.1)));
        }

        if (voxel.active && !voxel.refractive) {
            return base_shadow;
        }

        float3 reconst = side_dist * inv_dir;
        int3 eqs = select(min3(reconst.x, reconst.y, reconst.z) == reconst, 1, 0);
        floored_pos += dir_sign * eqs;
        side_dist += dir_sign * eqs;
    }

    return color;
}

struct SurfaceData {
    uint8_t4[4 * 4] colors;
}

#endif