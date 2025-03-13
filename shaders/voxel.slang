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
    int base = id.y - 15;
    base += noisy;
    //base += hash13(id) * 4;
    bool reflective = false;
    bool refractive = false;

    if (hash13(id * 3.432) > 0.95 && id.y == 40) {
        //base = -2;
    }


    if (hash12(floor(id.xz / 4)) > 0.99) {
        base -= 10;
        if (hash13(id) > 0.8) {
            //refractive = true;
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

// #define
static const bool RANDOMIZED_SURFACE_TEXELS = false;
static const uint SHADOW_ITER_COUNT = 32;
static const uint SHADOW_SAMPLES_PER_TICK = 4;
static const uint SURFACE_TEXELS_MODIFIED_PER_TICK = 16;
static const float SHADOW_TEMPORAL_LERP_ACCUMULATOR_FACTOR = 8.0;
static const bool SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR = true;
static const float SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR_FACTOR = 10.0;
static const float SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR_MARGIN = 0.3;
static const float SHADOW_ANGLE_SPREAD_FACTOR = 0.01;


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

struct GlassThingy {
    bool hit;
    uint3 floored;
    float3 world;
};

GlassThingy dda_gi_nate(
    RWTexture3D<uint8_t> voxels,
    float3 ray_dir,
    float3 ray_pos,
    out uint face,
) {
    float3 floored_pos = floor(ray_pos);
    float3 inv_dir = 1 / ray_dir;
    float3 dir_sign = sign(ray_dir);
    float3 side_dist = (floored_pos - ray_pos + 0.5 + 0.5 * dir_sign);
    Fetcher fetcher = Fetcher(voxels, size);
    face = 0;

    for (int i = 0; i < 32; i++) {
        Voxel voxel = fetcher.fetch((int3)(floored_pos));

        float3 test = (floored_pos - ray_pos + 0.5 - 0.5 * dir_sign) * inv_dir;
        float max = max3(test.x, test.y, test.z);
        float3 world = ray_pos + ray_dir * max;
        if (voxel.active) {
            return GlassThingy(true, (uint3)(floored_pos), world);
        }

        float3 reconst = side_dist * inv_dir;
        int3 eqs = select(min3(reconst.x, reconst.y, reconst.z) == reconst, 1, 0);
        face = firstbithigh(eqs.x | eqs.y << 1 | eqs.z << 2);
        floored_pos += dir_sign * eqs;
        side_dist += dir_sign * eqs;
    }

    return GlassThingy(false, 0, 0);
}

[shader("compute")]
[numthreads(8, 8, 8)]
void update(uint3 id: SV_DispatchThreadID, uniform float4 forward, uniform float4 position, uniform float4 sun, uniform uint tick, uniform float delta_raw) {
    /*
    if ((voxels[id - uint3(0, 1, 0)] & 1) == 0 && (voxels[id] & 1) == 1 && id.y > 0 && (voxels[id] & 4) == 4) {
        voxels[id - uint3(0, 1, 0)] = voxels[id];
        voxels[id] = 0;
    }
    */

    uint old = voxel_indices[id];
    bool empty = (voxels[id] & 1) == 0;

    if (empty) {
        return;
    }

    uint original = 0;

    if (!empty) {
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
                    SurfaceData data;
                    data.colors = null_data;
                    data.colors2 = null_data2;
                    surface_data_buffer[original + i] = data;
                }
            }
        } else {
            original = voxel_indices[id];
        }
    }

    if (!empty) {
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

        float delta = delta_raw;

        // we do some funky stuff... delta needs to be accounted for
        if (tick % 4 == 3) {
            delta /= (1.0 / 4.0);
        } else {
            delta /= (3.0 / 4.0);
        }

        // bool block_in_range = distance_ranges[0].x < d && d < distance_ranges[0].y;
        bool blockinate = block_in_range && block_visible;

        uint funny_index_magic = voxel_indices[id];
        uint funny_index = funny_index_magic & ~(0b111111 << (32 - 6));
        uint enabled_faces = (funny_index_magic >> (32 - 6)) & 0b111111;

        if (enabled_faces > 0 && blockinate && original != -1) {
            uint indexu = 0;

            for (int i = 0; i < 6; i++) {
                bool face_visible_camera = dot(offsets[i], diff) < 0.0;
                face_visible_camera = true;
                if (((enabled_faces >> i) & 1) == 1) {
                    if (face_visible_camera) {
                        // Per surface texel shadows!!!! Cached!!!
                        // This works!!!! Finally!!!
                        SurfaceData data = surface_data_buffer[original + indexu];
                        if (dot(offsets[i], sun.xyz) > 0) {
                            for (int k = 0; k < SURFACE_TEXELS_MODIFIED_PER_TICK; k++) {
                                // jarvis... randominate this shit....
                                int p = 0;
                                if (RANDOMIZED_SURFACE_TEXELS) {
                                    p = hash(k * 423 + tick * 32 + id.x + id.y * 2 + id.z * 4) % 16;
                                } else {
                                    p = k;
                                }

                                uint2 uv = uint2(p % 4, p / 4);
                                uint3 unflattened = unflatten_uvs(i / 2, i % 2 == 0, uv);
                                float3 shadow_color = 0.0;

                                // ts so slow twin...
                                for (int s = 0; s < SHADOW_SAMPLES_PER_TICK; s++) {
                                    // jarvis... randominate this shit...
                                    float3 sun_sample = normalize(sun.xyz + (hash33(s * 2432.43243 - (id + unflattened / 3.0) * 232.342 + tick * 43.23) - 0.5) * SHADOW_ANGLE_SPREAD_FACTOR);
                                    float3 world_pos = id + 0.125 + offsets[i] * 0.15 + unflattened / 4.0 + sun_sample * 0.15;
                                    shadow_color += dda_shadownate(voxels, sun_sample, world_pos);
                                }

                                uint8_t4 old = data.colors[p];
                                float3 a = old.xyz / 255.0;
                                float3 b = shadow_color / SHADOW_SAMPLES_PER_TICK;
                                float3 output = 0.0;
                                
                                // TODO: find a better accumulator function...
                                if (SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR) {
                                    float3 l = SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR_MARGIN;
                                    output = a + SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR_FACTOR * clamp(b - a, -l, l) * delta;
                                } else {
                                    output = lerp(a, b, clamp(SHADOW_TEMPORAL_LERP_ACCUMULATOR_FACTOR * delta, 0.01, 1.0));
                                }
                                data.colors[p] = uint8_t4(clamp(output, 0, 1) * 255, 0);
                            }
                        }
                        surface_data_buffer[original + indexu] = data;
                    }

                    indexu++;
                }
            }
        }

        AllMemoryBarrier();

        /*
        for (int i = 0; i < countbits(enabled_faces); i++) {
            SurfaceData data = surface_data_buffer[original + i];
            data.colors2 = null_data2;
            surface_data_buffer[original + i] = data;
        }
        */

        int indexu = 0;
        if (enabled_faces > 0 && blockinate && original != -1) {
            for (int i = 0; i < 6; i++) {
                if (((enabled_faces >> i) & 1) == 1) {
                    for (int k = 0; k < 16; k++) {
                        // jarvis... randominate this shit....
                        int p = k;
                        //int p = hash(k * 423 + tick * 32 + id.x + id.y * 2 + id.z * 4) % 16;
                        uint2 uv = uint2(p % 4, p / 4);
                        uint3 unflattened = unflatten_uvs(i / 2, i % 2 == 0, uv);
                        float3 shadow_color = 0.0;

                        for (int s = 0; s < 1; s++) {
                            // create a random sample in the hemisphere pointing in the face direction...
                            float3 random_sample = hash33((id + unflattened / 3.0) * 232.342 + tick * 43.23 + s * 432.43243) - 0.5;

                            // rudementary mapping to keep is surface normal aligned
                            if (dot(random_sample, offsets[i]) < 0) {
                                random_sample = -random_sample;
                            }

                            random_sample = normalize(random_sample);
                            float3 world_pos = id + 0.125 + offsets[i] * 0.15 + unflattened / 4.0 + random_sample * 0.55;
                            uint face = 0;
                            float3 dir_sign = sign(random_sample);

                            Fetcher fetcher;
                            fetcher.max_size = size;
                            fetcher.voxels = voxels;

                            float3 src_color = light(sun.xyz, fetcher, id, 0.0, random_sample, unflattened / 3.0, offsets[i], 1.0, 1.0);
                            uint8_t4 shadowed_FUCK_ME_I_NEED_TO_WORK_ON_MY_DISCRETE_ASSIGNMENT = surface_data_buffer[original + indexu].colors[p];
                            float3 shadowed = shadowed_FUCK_ME_I_NEED_TO_WORK_ON_MY_DISCRETE_ASSIGNMENT.xyz / 255.0;
                            GlassThingy target = dda_gi_nate(voxels, random_sample, world_pos, face);
                            if (target.hit) {
                                uint funny_index_magic = voxel_indices[target.floored];
                                float3 uv = target.world - target.floored;
                                uint funny_index = funny_index_magic & ~(0b111111 << (32 - 6));
                                uint enabled_faces2 = (funny_index_magic >> (32 - 6)) & 0b111111;
                                uint face2 = global_face(face, dir_sign);
                                int packed_face_index = countbits((enabled_faces2 << (6 - face2)) & 0b111111);
                                float3 normal = normal(face, dir_sign);

                                float2 flat = flatten_uvs(face, dir_sign, uv);
                                uint2 pixels = (uint2)(floor(flat * 4));
                                uint converted_to_flat_index = pixels.x + pixels.y * 4;

                                // src_color = 0.2;

                                if (((enabled_faces2 >> face2) & 1) == 1 && funny_index_magic != -1 && converted_to_flat_index < 16 && converted_to_flat_index >= 0) {

                                    uint current = surface_data_buffer[funny_index + packed_face_index].colors2[converted_to_flat_index];
                                    uint8_t4 unpacked_current = 0;
                                    unpacked_current.x = current & 0xFF;
                                    unpacked_current.y = (current >> 8) & 0xFF;
                                    unpacked_current.z = (current >> 16) & 0xFF;

                                    float3 a = float3(unpacked_current.xyz) / 255.0;
                                    float3 b = (dot(offsets[i], sun.xyz) * 0.5 + 0.5) * shadowed * src_color * 0.5;
                                    float3 output = lerp(a, b, delta * 2.0);

                                    uint8_t4 unpacked = uint8_t4(clamp(output, 0, 1) * 255, 0);
                                    uint packed = 0;
                                    packed |= unpacked.x;
                                    packed |= ((uint)unpacked.y) << 8;
                                    packed |= ((uint)unpacked.z) << 16;
                                    // InterlockedExchange(surface_data_buffer[funny_index + packed_face_index].colors2[converted_to_flat_index], clamp(current + 1, 0, 0xFFFFFF));
                                    InterlockedExchange(surface_data_buffer[funny_index + packed_face_index].colors2[converted_to_flat_index], packed);

                                    // surface_data_buffer[funny_index + packed_face_index].other = clamp(offsets[i], 0, 1) * 255;
                                    // InterlockedCompareExchange
                                    // InterlockedAdd(surface_data_buffer[funny_index + packed_face_index].other.y, adder.y);
                                    // InterlockedAdd(surface_data_buffer[funny_index + packed_face_index].other.z, adder.z);
                                }
                            }
                        }
                    }
                    indexu++;
                }
            }
        }

        AllMemoryBarrier();
    }

    uint8_t temp = voxels[id];
    if ((temp & 8) == 8) {
        temp &= ~((uint8_t)1 << 3);
        voxels[id] = temp;
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