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
    int base = (int)(noise((float2)id.xz * 0.1) * 10) + id.y - 15;
    base = id.y - 15;
    //base += hash13(id) * 4;
    bool reflective = false;
    bool refractive = false;

    if (hash13(id * 3.432) > 0.95 && id.y == 40) {
        //base = -2;
    }


    if (hash12(floor(id.xz / 4)) > 0.99) {
        if (hash13(id) > 0.8) {
            base -= 30;
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

    bool block_visible = dot(forward, diff) > 0.0;
    if (!block_visible) {
        //return 0;
    }

    uint enabled_faces = 0;

    for (int i = 0; i < 6; i++) {
        Voxel neighbour = Voxel.from_raw(voxels[id + offsets[i]]);
        bool face_visible_neighbour = !neighbour.active || neighbour.refractive;
        bool face_visible_camera = dot(offsets[i], diff) < 0.0;
        if (face_visible_neighbour /* && face_visible_camera */) {
            enabled_faces |= 1 << i;
        }
    }

    return enabled_faces;
}

[shader("compute")]
[numthreads(8, 8, 8)]
void main2(uint3 id: SV_DispatchThreadID, uniform float4 forward, uniform float4 position, uniform float4 sun) {
    if ((voxels[id - uint3(0, 1, 0)] & 1) == 0 && (voxels[id] & 1) == 1 && id.y > 0 && (voxels[id] & 4) == 4) {
        voxels[id - uint3(0, 1, 0)] = voxels[id];
        voxels[id] = 0;
    }

    AllMemoryBarrier();

    uint enabled_faces = calculate_enabled_faces(id, position.xyz, forward.xyz);
    // enabled_faces = 0b100101;

    voxel_indices[id] = -1;
    if (enabled_faces == 0) {
        return;
    }

    uint original = 0;
    if ((voxels[id] & 1) == 1 && enabled_faces > 0) {
        original = counter[0].add(countbits(enabled_faces), MemoryOrder.Relaxed);
        uint some_magic = original;
        some_magic |= enabled_faces << (32 - 6);
        voxel_indices[id] = some_magic;

        for (int i = 0; i < countbits(enabled_faces); i++) {
            SurfaceData data;
            data.other = 0;
            surface_data_buffer[original + i] = data;
        }
    }

    AllMemoryBarrier();
    if ((voxels[id] & 1) == 1 && enabled_faces > 0) {
        uint indexu = 0;
        for (int i = 0; i < 6; i++) {
            if (((enabled_faces >> i) & 1) == 1) {

                /*
                SurfaceData data;
                for (int k = 0; k < 16; k++) {
                    uint2 uv = uint2(k % 4, k / 4);
                    uint3 unflattened = unflatten_uvs(i / 2, i % 2 == 0, uv);
                    float3 world_pos = id + 0.25 + offsets[i] * 0.25 + unflattened / 4.0 - sun.xyz * 0.05;
                    // FIXME: still broken btw... on {x,y,z} = 0...
                    float3 shadow_color = dda_shadownate(voxels, sun.xyz, world_pos);
                    data.colors[k] = uint8_t4(clamp(shadow_color, 0, 1) * 255, 0);
                }

                surface_data_buffer[original + indexu] = data;
                */

                float3 amogsun = sun.xyz;
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

                /*
                float3 world_pos = id + 0.5 + offsets[i] * 1.0 - amogsun * 0.1;
                float3 reflected = reflect(-amogsun, normalize(offsets[i]));
                uint face = 0;
                float3 dir_sign = sign(reflected);
                uint3 target = dda_shadownate2(voxels, reflected, world_pos, face);

                SurfaceData data;
                if (all(target > 0) && dot((float3)offsets[i], sun.xyz) > 0) {
                    data.other = clamp(offsets[i], 0, 1) * 255;
                } else {
                    data.other = 0;
                }

                //data.other = clamp(reflected, 0, 1) * 255;
                //data.other = id;
                surface_data_buffer[original + indexu] = data;
                */

                indexu++;
            }
        }
    }

    AllMemoryBarrier();
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