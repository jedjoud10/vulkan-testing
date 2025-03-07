#include <other.slang>
#include <lighting.slang>

[[vk::binding(0, 0)]]
RWTexture3D<uint8_t> voxels;

[[vk::binding(1, 0)]]
RWStructuredBuffer<SurfaceData> surface_data_buffer;

[[vk::binding(2, 0)]]
RWTexture3D<uint> voxel_indices;

[[vk::binding(3, 0)]]
RWStructuredBuffer<uint> counter;

[shader("compute")]
[numthreads(8, 8, 8)]
void main(uint3 id: SV_DispatchThreadID) {
    int base = (int)(noise((float2)id.xz * 0.1) * 10) + id.y - 15;
    //base += hash13(id) * 4;
    bool reflective = false;
    bool refractive = false;

    if (hash13(id * 3.432) > 0.95 && id.y == 40) {
        //base = -2;
    }


    if (hash12(floor(id.xz / 4)) > 0.99) {
        if (hash13(id) > 0.8) {
            base -= 10;
            refractive = true;
        }

        /*
        if (hash12(id.xz) > 0.5) {
            refractive = true;
        } else {
            //reflective = true;
        }
        */
    }

    Voxel voxel;
    voxel.active = base < 0;
    voxel.reflective = reflective;
    voxel.refractive = refractive;
    voxels[id] = voxel.into_raw();
}

[shader("compute")]
[numthreads(8, 8, 8)]
void main2(uint3 id: SV_DispatchThreadID, uniform float4 forward, uniform float4 position, uniform float4 sun) {
    if ((voxels[id - uint3(0, 1, 0)] & 1) == 0 && (voxels[id] & 1) == 1 && id.y > 0) {
        //voxels[id - uint3(0, 1, 0)] = voxels[id];
        //voxels[id] = 0;
    }

    AllMemoryBarrier();

    float3 diff = normalize((float3)id - position.xyz + 0.5);
    bool block_visible = dot(forward.xyz, diff) > 0.0;
    if ((voxels[id] & 1) == 1 && block_visible) {
        static const int3[] offsets = {
            int3(1, 0, 0),
            int3(-1, 0, 0),
            int3(0, 1, 0),
            int3(0, -1, 0),
            int3(0, 0, 1),
            int3(0, 0, -1),
        };

        uint enabled_faces = 0;

        for (int i = 0; i < 6; i++) {
            Voxel neighbour = Voxel.from_raw(voxels[id + offsets[i]]);
            bool face_visible_neighbour = !neighbour.active || neighbour.refractive;
            bool face_visible_camera = dot(offsets[i], diff) < 0.0;
            if (face_visible_neighbour && face_visible_camera) {
                enabled_faces |= 1 << i;
            }
        }

        //enabled_faces = 0b100101;
        if (enabled_faces > 0) {
            uint original = 0;
            InterlockedAdd(counter[0], countbits(enabled_faces), original);

            uint indexu = 0;
            for (int i = 0; i < 6; i++) {
                if (((enabled_faces >> i) & 1) == 1) {
                    SurfaceData data;

                    /*
                    for (int s = 0; s < 8; s++) {
                        if (raymarch_do_the_funny(voxels, normalize(sun + (hash33(s * float3(2.12321, 5.3434, -34.2323))-0.5)*0.2), (float3)id + (float3)offsets[i] * 1 + 0.5)) {
                            totals++;
                        }
                    }
                    */

                    float3 color = 0;
                    for (int k = 0; k < 16; k++) {
                        uint2 uv = uint2(k % 4, k / 4);
                        uint3 unflattened = unflatten_uvs(i / 2, i % 2 == 0, uv);
                        float3 world_pos = id + 0.125 + (unflattened / 4.0);
                        float3 shadow_color = dda_shadownate(voxels, normalize(sun.xyz), world_pos + offsets[i] * 0.15, 0.2);
                        color = shadow_color;
                        data.colors[k] = uint8_t4(clamp(color, 0, 1) * 255, 0);
                    }
                    
                    surface_data_buffer[original + indexu] = data;
                    indexu++;
                }
            }

            uint some_magic = original;
            some_magic |= enabled_faces << (32 - 6);
            voxel_indices[id] = some_magic;
        } else {
            voxel_indices[id] = -1;
        }
    }    
}