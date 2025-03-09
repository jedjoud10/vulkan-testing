#include <other.slang>
#include <lighting.slang>

[[vk::binding(0, 0)]]
RWTexture3D<uint8_t> voxels;

[[vk::binding(1, 0)]]
RWStructuredBuffer<SurfaceData> surface_data_buffer;

[[vk::binding(2, 0)]]
RWTexture3D<uint> voxel_indices;

[[vk::binding(3, 0)]]
RWStructuredBuffer<Atomic<uint>> counter;

[shader("compute")]
[numthreads(8, 8, 8)]
void main(uint3 id: SV_DispatchThreadID) {
    int noisy = (int)(noise((float2)id.xz * 0.1) * 10);
    int base = noisy + id.y - 15;
    //base += hash13(id) * 4;
    bool reflective = false;
    bool refractive = false;

    if (hash13(id * 3.432) > 0.95 && id.y == 40) {
        //base = -2;
    }


    if (hash12(floor(id.xz / 4)) > 0.99) {
        base -= 10;
        if (hash13(id) > 0.8) {
            refractive = true;
        }

        // TODO: reflections don't work with the current shadow surface optimization1!!!
        if (hash12(id.xz) > 0.2) {
            //base -= 30;
            //reflective = true;
        }
    }

    Voxel voxel;
    voxel.active = base < 0;
    voxel.reflective = reflective;
    voxel.refractive = refractive;
    voxels[id] = voxel.into_raw();
}

static const int3[] offsets = {
    int3(1, 0, 0),
    int3(-1, 0, 0),
    int3(0, 1, 0),
    int3(0, -1, 0),
    int3(0, 0, 1),
    int3(0, 0, -1),
};

uint calculate_enabled_faces(uint3 id, float3 camera, float3 forward) {
    float3 diff = normalize((float3)id - camera + 0.5);

    uint enabled_faces = 0;

    for (int i = 0; i < 6; i++) {
        Voxel neighbour = Voxel.from_raw(voxels[id + offsets[i]]);
        bool face_visible_neighbour = !neighbour.active || neighbour.refractive;
        if (face_visible_neighbour) {
            enabled_faces |= 1 << i;
        }
    }

    return enabled_faces;
}

static const uint SHADOW_ITER_COUNT = 48;
static const uint SHADOW_SAMPLES_PER_TICK = 2;
static const uint SURFACE_TEXELS_MODIFIED_PER_TICK = 8;
static const float SHADOW_TEMPORAL_ACCUMULATOR_FACTOR = 8.0;
static const float SHADOW_ANGLE_SPREAD_FACTOR = 0.05;

float3 dda_shadownate(
    RWTexture3D<uint8_t> voxels,
    float3 ray_dir,
    float3 ray_pos,
) {
    float3 floored_pos = floor(ray_pos);
    float3 inv_dir = 1 / ray_dir;
    float3 dir_sign = sign(ray_dir);
    float3 side_dist = (floored_pos - ray_pos + 0.5 + 0.5 * dir_sign);
    Fetcher fetcher = Fetcher(voxels, size);
    float3 color = 1.0;

    for (int i = 0; i < SHADOW_ITER_COUNT; i++) {
        Voxel voxel = fetcher.fetch((int3)(floored_pos));

        if (voxel.active && voxel.refractive) {
            color *= normalize(hash33(floor(floored_pos) * float3(23.231, -435.4354, 9412.1)));
        }

        if (voxel.active && !voxel.refractive) {
            return 0.0;
        }

        float3 reconst = side_dist * inv_dir;
        int3 eqs = select(min3(reconst.x, reconst.y, reconst.z) == reconst, 1, 0);
        floored_pos += dir_sign * eqs;
        side_dist += dir_sign * eqs;
    }

    return color;
}

[shader("compute")]
[numthreads(8, 8, 8)]
void update(uint3 id: SV_DispatchThreadID, uniform float4 forward, uniform float4 position, uniform float4 sun, uniform uint tick, uniform float delta) {
    /*
    if ((voxels[id - uint3(0, 1, 0)] & 1) == 0 && (voxels[id] & 1) == 1 && id.y > 0 && (voxels[id] & 4) == 4) {
        voxels[id - uint3(0, 1, 0)] = voxels[id];
        voxels[id] = 0;
    }
    */

    uint old = voxel_indices[id];
    
    if ((voxels[id] & 1) == 0) {
        return;
    }

    uint original = 0;

    if (tick == 0) {
        voxel_indices[id] = -1;

        uint enabled_faces = calculate_enabled_faces(id, position.xyz, forward.xyz);
        if (enabled_faces == 0) {
            return;
        }
        
        if (enabled_faces > 0) {
            original = counter[0].add(countbits(enabled_faces), MemoryOrder.Relaxed);
            uint some_magic = original;
            some_magic |= enabled_faces << (32 - 6);
            voxel_indices[id] = some_magic;

            for (int i = 0; i < countbits(enabled_faces); i++) {
                SurfaceData data = SurfaceData(null_data);
                surface_data_buffer[original + i] = data;
            }
        }
    } else {
        original = voxel_indices[id];
    }

    float3 diff = normalize((float3)id - position.xyz + 0.5);
    bool block_visible = dot(forward.xyz, diff) > 0.0;

    float2[4] distance_ranges = {
        float2(0, 50),
        float2(0, 50),
        float2(0, 50),
        float2(50, 100),
    };

    float d = distance(id, position.xyz);
    bool block_in_range = distance_ranges[tick % 4].x < d && d < distance_ranges[tick % 4].y;
    //bool block_in_range = distance_ranges[0].x < d && d < distance_ranges[0].y;
    bool blockinate = block_in_range && block_visible;

    uint funny_index_magic = voxel_indices[id];
    uint funny_index = funny_index_magic & ~(0b111111 << (32 - 6));
    uint enabled_faces = (funny_index_magic >> (32 - 6)) & 0b111111;
    if (enabled_faces > 0 && blockinate && original != -1) {
        uint indexu = 0;

        for (int i = 0; i < 6; i++) {
            bool face_visible_camera = dot(offsets[i], diff) < 0.0;
            if (((enabled_faces >> i) & 1) == 1) {
                if (face_visible_camera) {
                    // Per surface texel shadows!!!! Cached!!!
                    // This works!!!! Finally!!!
                    SurfaceData data = surface_data_buffer[original + indexu];
                    if (dot(offsets[i], sun.xyz) > 0) {
                        for (int k = 0; k < SURFACE_TEXELS_MODIFIED_PER_TICK; k++) {
                            // jarvis... randominate this shit....
                            int p = hash(k * 423 + tick * 32 + id.x + id.y * 2 + id.z * 4) % 16;
                            uint2 uv = uint2(p % 4, p / 4);
                            uint3 unflattened = unflatten_uvs(i / 2, i % 2 == 0, uv);
                            float3 shadow_color = 0.0;

                            // ts so slow twin...
                            for (int s = 0; s < SHADOW_SAMPLES_PER_TICK; s++) {
                                // jarvis... randominate this shit...
                                float3 sun_sample = normalize(sun.xyz + (hash33(s * 2432.43243 - (id + unflattened / 4.0) * 232.342 + tick * 43.23) - 0.5) * SHADOW_ANGLE_SPREAD_FACTOR);
                                float3 world_pos = id + 0.125 + offsets[i] * 0.15 + unflattened / 4.0 + sun_sample * 0.15;
                                shadow_color += dda_shadownate(voxels, sun_sample, world_pos);
                            }

                            uint8_t4 old = data.colors[p];
                            float3 a = old.xyz / 255.0;
                            float3 b = shadow_color / SHADOW_SAMPLES_PER_TICK;

                            // TODO: find a better accumulator function...
                            float3 output = lerp(a, b, clamp(SHADOW_TEMPORAL_ACCUMULATOR_FACTOR * delta, 0.01, 1.0));

                            data.colors[p] = uint8_t4(clamp(output, 0, 1) * 255, 0);
                        }
                    } else {
                        data.colors = null_data;
                    }

                    //data.colors[0] += 10;
                    surface_data_buffer[original + indexu] = data;

                    //float3 amogsun = -sun.xyz;

                    // data.other = clamp(reflected, 0, 1) * 255;
                    // data.other = id;
                    //surface_data_buffer[original + indexu] = data;
                }
                

                indexu++;
            }
        }
    }
}

/*
shadow_color = dda_shadownate(voxels, reflected, world_pos);
color = shadow_color;
data.colors[k] = uint8_t4(clamp(color, 0, 1) * 255, 0);
*/

/*
// FIXME: this is *slightly* slow... kek (ts so ass ts so slow ts pmo ngl no cap)
for (int killme = 0; killme < 16; killme++) {
    shadow_color += dda_shadownate(voxels, normalize(sun.xyz + (hash33(id + i * 123.12 + k * -432.32 + killme * 5456.23) - 0.5) * 0.05), world_pos);
}
color = shadow_color / 16.0;
*/

/*
                for (int k = 0; k < 16; k++) {
                    float3 world_pos = id + 0.5 + offsets[i] * 1.0 - amogsun * 0.1;
                    float3 reflected = reflect(-amogsun, normalize(offsets[i] + (hash33(k * 2.43243 + id * 232.342) - 0.5) * 0.0));
                    uint face = 0;
                    float3 dir_sign = sign(reflected);
                    uint3 target = dda_shadownate2(voxels, reflected, world_pos, face);

                    if (all(target > 0)) {
                        uint funny_index_magic = voxel_indices[target];

                        uint funny_index = funny_index_magic & ~(0b111111 << (32 - 6));
                        uint enabled_faces2 = (funny_index_magic >> (32 - 6)) & 0b111111;
                        uint face2 = global_face(face, dir_sign);
                        float3 normal = normal(face, dir_sign);

                        uint enabled_faces2_copy = (enabled_faces2 << (6 - face2)) & 0b111111;
                        int packed_face_index = countbits(enabled_faces2_copy);

                        if (((enabled_faces2 >> face2) & 1) == 1 && dot((float3)offsets[i], amogsun) > 0) {
                            // surface_data_buffer[funny_index + packed_face_index].other = clamp(offsets[i], 0, 1) * 255;
                            uint3 adder = amogus((float3)offsets[i]);
                            InterlockedAdd(surface_data_buffer[funny_index + packed_face_index].other.x, adder.x);
                            InterlockedAdd(surface_data_buffer[funny_index + packed_face_index].other.y, adder.y);
                            InterlockedAdd(surface_data_buffer[funny_index + packed_face_index].other.z, adder.z);
                        }
                    }
                }
                */

/*
SurfaceData data;

for (int k = 0; k < 64; k++) {
    float3 world_pos = id + 0.5 + offsets[i] * 1.0 - amogsun * 0.1;
    float3 reflected = reflect(-amogsun, normalize(offsets[i] + (hash33(k * 2.43243 + id * 232.342) - 0.5) * 0.3));
    uint face = 0;
    float3 dir_sign = sign(reflected);
    uint3 target = dda_shadownate2(voxels, reflected, world_pos, face);


    if (all(target > 0) && dot((float3)offsets[i], sun.xyz) > 0) {
        data.other += 4;
    } else {
        data.other = 0;
    }
}
*/