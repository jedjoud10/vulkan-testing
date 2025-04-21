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
    //base += noisy * 0.5;
    //base += hash13(id) * 4;
    bool reflective = false;
    bool refractive = false;

    if (hash13(id * 3.432) > 0.95 && id.y == 40) {
        //base = -2;
    }


    if (hash12(floor(id.xz / 4)) > 0.99) {
        base -= 10;
        if (hash13(id) > 0.8) {
            base -= 30;
            //refractive = true;
        }

        // TODO: reflections don't work with the current shadow surface optimization1!!!
        if (hash12(id.xz) > 0.2) {
            //base -= 30;
            reflective = true;
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

uint calculate_enabled_faces(uint3 id) {
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

static const uint SHADOW_ITER_COUNT = 32;
static const uint SHADOW_SAMPLES_PER_TICK = 2;
static const float SHADOW_TEMPORAL_LERP_ACCUMULATOR_FACTOR = 8.0;
static const bool SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR = true;
static const float SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR_FACTOR = 5.0;
static const float SHADOW_TEMPORAL_CLAMPED_ACCUMULATOR_MARGIN = 0.4;
static const float SHADOW_ANGLE_SPREAD_FACTOR = 0.08;

float3 hemispherenate(int i, float3 n, int size, int face) {
    return 0.0;
    /*
    uint2 local = uint2(i % size, i / size);
    float2 uv = local / (float)size;
    float3 fuck = 0.0;

    if (face == 0) {
        fuck = float3()
    } else if (face == 1) {
    } else {

    }

    return normalize(fuck);
    */
}

// TODO: need to calculate how "close" we get to the surface...
float3 dda_shadownate(
    RWTexture3D<uint8_t> voxels,
    float3 ray_dir,
    float3 ray_pos,
) {
    float3 floored_pos = floor(ray_pos);
    float3 inv_dir = 1 / ray_dir;
    float3 dir_sign = sign(ray_dir);
    float3 side_dist = (floored_pos - ray_pos + 0.5 + 0.5 * dir_sign);
    Fetcher fetcher = Fetcher(voxels);
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
    Fetcher fetcher = Fetcher(voxels);
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
    if ((voxels[id - uint3(0, 1, 0)] & 1) == 0 && (voxels[id] & 1) == 1 && id.y > 0 && (tick % 128) == 0) {
        voxels[id - uint3(0, 1, 0)] = voxels[id];
        voxels[id] = 0;
    }
    */

    bool empty = (voxels[id] & 1) == 0;

    if (empty) {
        voxel_indices[id] = INVALID;
        return;
    }

    // TREAT THIS AS -1 SINCE WE DO SOME BIT TWIDDLING AT THE END
    // Contains both the block index and enabled faces bitset
    uint packed_index = INVALID;

    if (tick == 0) {
        uint enabled_faces = calculate_enabled_faces(id);

        if (enabled_faces > 0) {
            uint block_index = counter[0].add(countbits(enabled_faces), MemoryOrder.Relaxed);
            packed_index = block_index;
            packed_index |= enabled_faces << (32 - 6);
            voxel_indices[id] = packed_index;

            for (int i = 0; i < countbits(enabled_faces); i++) {
                SurfaceData data = SurfaceData();
                data.colors[0] = uint8_t4(0);
                surface_data_buffer[block_index + i] = data;
            }
        } else {
            packed_index = INVALID;
            voxel_indices[id] = INVALID;
        }
    } else {
        packed_index = voxel_indices[id];
    }

    
    float3 diff = normalize((float3)id - position.xyz + 0.5);
    bool block_visible = dot(forward.xyz, diff) > 0.0;

    // Unpack the block index and enabled faces bitset
    uint block_index = packed_index & ~(0b111111 << (32 - 6));
    uint enabled_faces = (packed_index >> (32 - 6)) & 0b111111;

    float delta = delta_raw;

    if (!empty && enabled_faces > 0 && block_visible && packed_index != INVALID) {
        uint packed_face_index = 0;
        for (int i = 0; i < 6; i++) {
            if (((enabled_faces >> i) & 1) == 1) {
                // More strict check to make sure that the face faces the camera
                // Causes buggy reflections, but we can deal with that for now...
                bool face_visible_camera = dot(offsets[i], diff) < 0.0;
                if (face_visible_camera) {
                    // Read cached texel data
                    SurfaceData data = surface_data_buffer[block_index + packed_face_index];

                    // Execute the shadow calculation for every possible texel...
                    for (int k = 0; k < 16; k++) {
                        int p = k;

                        // Skip the texel if it doesn't face the sun
                        if (dot(offsets[i], sun.xyz) <= 0) {
                            data.colors[p] = uint8_t4(0);
                            continue;
                        }


                        uint2 uv = uint2(p % 4, p / 4);
                        uint3 unflattened = unflatten_uvs(i / 2, i % 2 == 0, uv);
                        
                        // ts so slow twin...
                        // ts pmo... ong...
                        float3 shadow_color = 0.0;
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

                    // Write new texel data
                    surface_data_buffer[block_index + packed_face_index] = data;
                }

                // TODO: Figure out how to do the bit magic but in reverse order?
                packed_face_index++;
            }
        }
    }


    
    /*
    if (!empty) {
        float3 diff = normalize((float3)id - position.xyz + 0.5);
        bool block_visible = dot(forward.xyz, diff) > 0.0;

        float d = distance(id, position.xyz);
        bool block_in_range = d < 50;
        block_in_range = true;

        float delta = delta_raw;

        // we do some funky stuff... delta needs to be accounted for
        if (tick % 4 == 3) {
            delta /= (1.0 / 4.0);
        } else {
            delta /= (3.0 / 4.0);
        }

        //bool force = tick % 4 == 3 && hash13(group_id * 31.231 + tick * 3.21312) > 0.9;
        bool force = false;
        // bool block_in_range = distance_ranges[0].x < d && d < distance_ranges[0].y;
        bool blockinate = block_in_range && (block_visible || force);

        uint funny_index_magic = voxel_indices[id];
        uint funny_index = funny_index_magic & ~(0b111111 << (32 - 6));
        uint enabled_faces = (funny_index_magic >> (32 - 6)) & 0b111111;

        if (enabled_faces > 0 && blockinate && original != INVALID) {
            uint indexu = 0;

            for (int i = 0; i < 6; i++) {
                bool face_visible_camera = dot(offsets[i], diff) < 0.0;
                // TODO: needed to have this be always true for GI stuff...
                face_visible_camera = true;
                if (((enabled_faces >> i) & 1) == 1) {
                    if (face_visible_camera || force) {
                        // Per surface texel shadows!!!! Cached!!!
                        // This works!!!! Finally!!!
                        SurfaceData data = surface_data_buffer[original + indexu];
                        for (int k = 0; k < SURFACE_TEXELS_MODIFIED_PER_TICK; k++) {
                            // jarvis... randominate this shit....
                            int p = 0;
                            if (RANDOMIZED_SURFACE_TEXELS) {
                                p = hash(k * 423 + tick * 32 + id.x + id.y * 2 + id.z * 4) % 16;
                            } else {
                                p = k;
                            }

                            if (dot(offsets[i], sun.xyz) <= 0) {
                                data.colors[p] = uint8_t4(0);
                                continue;
                            } else {
                                data.colors[p] = uint8_t4(255);
                                continue;
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

                            //output = 1.0;
                            //output = (tick % 16) / 16.0;
                            
                            data.colors[p] = uint8_t4(clamp(output, 0, 1) * 255, 0);
                            //data.colors[p] = (tick % 16)
                            //float3 old_gi = unpack_gi_color(data.colors2[p]);
                            //data.colors2[p] = pack_gi_color(lerp(old_gi, 0, 0.1 * delta));
                        }


                        surface_data_buffer[original + indexu] = data;
                    }

                    indexu++;
                }
            }
        }
        */


        /*
        if (enabled_faces > 0 && original != -1) {
            uint indexu = 0;
            for (int i = 0; i < 6; i++) {
                if (((enabled_faces >> i) & 1) == 1) {
                    SurfaceData data = surface_data_buffer[original + indexu];
                    for (int k = 0; k < 16; k++) {
                        float3 old_gi = unpack_gi_color(data.colors2[k]);
                        data.colors2[k] = pack_gi_color(lerp(old_gi, 0, 0.3 * delta));
                    }

                    surface_data_buffer[original + indexu] = data;

                    indexu++;
                }
            }
        }

        int indexu = 0;
        uint enabled_faces2 = calculate_enabled_faces(id, position.xyz, forward.xyz);
        if (enabled_faces2 > 0 && DO_THE_FUNKY_GI) {
            for (int i = 0; i < 6; i++) {
                if (((enabled_faces2 >> i) & 1) == 1) {
                    for (int k = 0; k < 64; k++) {
                        // jarvis... randominate this shit....
                        int p = k % 16;
                        //int p = hash(k * 423 + tick * 32 + id.x + id.y * 2 + id.z * 4) % 16;
                        uint2 uv = uint2(p % 4, p / 4);
                        uint3 unflattened = unflatten_uvs(i / 2, i % 2 == 0, uv);

                        uint3 thingymajig = id;
                        float3 world_pos;
                        float3 dir_sign;
                        uint face = i;

                        Fetcher fetcher;
                        fetcher.max_size = size;
                        fetcher.voxels = voxels;

                        uint block_index = original;
                        uint face_index = indexu;
                        uint texel_index = p;

                        // assumes that there is color "accumulation" or "bleeding" between the indirection bounces
                        float3 shadowed = 0.0;

                        if (original != -1) {
                            shadowed = surface_data_buffer[block_index + face_index].colors[texel_index].xyz / 255.0;
                        } else {
                            shadowed = 1.0;
                        }
                        float3 src_color = shadowed;

                        float3 random_sample = hash33((id + unflattened / 4.0) * 232.342 + tick * 43.23 + i * 34.42342 + k * 43.2342) - 0.5;
                        if (dot(random_sample, offsets[face]) < 0) {
                            random_sample = -random_sample;
                        }
                        random_sample = normalize(random_sample);

                        world_pos = thingymajig + 0.125 + offsets[face] * 0.15 + unflattened / 4.0 + random_sample * 0.35;
                        face = 0;
                        dir_sign = sign(random_sample);

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

                            // update bounce values...
                            thingymajig = target.floored;
                            face = face2;
                            block_index = funny_index;
                            face_index = packed_face_index;
                            texel_index = converted_to_flat_index;

                            // deposit a bit of the color on this object, and then continue onto the next one!
                            if (((enabled_faces2 >> face2) & 1) == 1 && funny_index_magic != -1 && converted_to_flat_index < 16 && converted_to_flat_index >= 0) {

                                uint current = surface_data_buffer[block_index + face_index].colors2[converted_to_flat_index];
                                float3 a = unpack_gi_color(current);
                                float3 b = src_color;
                                float3 output = lerp(a, b, delta * 1);

                                uint packed = pack_gi_color(output);
                                InterlockedExchange(surface_data_buffer[block_index + face_index].colors2[converted_to_flat_index], packed);
                            }
                        }
                        
                    }
                    indexu++;
                }
            }
        }

        AllMemoryBarrier();
        */
    // }


    /*
    // this needs to be its own kernel bruh....
    uint8_t temp = voxels[id];
    if ((temp & 8) == 8) {
        temp &= ~((uint8_t)1 << 3);
        voxels[id] = temp;
    }
    */
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