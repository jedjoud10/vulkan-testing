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
    int base = (int)(noise((float2)id.xz * 0.1) * 15) + id.y - 15;
    //base += hash13(id) * 4;
    bool reflective = false;
    bool refractive = false;


    uint8_t raw = 0;

    if (hash12(floor(id.xz / 4)) > 0.99) {
        if (hash13(id) > 0.8) {
            base -= 20;
        }

        if (hash12(id.xz) > 0.5) {
            //refractive = true;
        } else {
            //reflective = true;
        }
    }
    
    bool active = base < 0;
    raw |= active ? 1 : 0;
    raw |= reflective ? 2 : 0;
    raw |= refractive ? 4 : 0;
    voxels[id] = raw;
}

[shader("compute")]
[numthreads(8, 8, 8)]
void main2(uint3 id: SV_DispatchThreadID) {
    if ((voxels[id - uint3(0, 1, 0)] & 1) == 0 && (voxels[id] & 1) == 1 && id.y > 0) {
        //voxels[id - uint3(0, 1, 0)] = voxels[id];
        //voxels[id] = 0;
    }

    AllMemoryBarrier();

    if ((voxels[id] & 1) == 1) {
        static const int3[] offsets = {
            int3(-1, 0, 0),
            int3(1, 0, 0),
            int3(0, -1, 0),
            int3(0, 1, 0),
            int3(0, 0, -1),
            int3(0, 0, 1),
        };

        uint enabled_faces = 0;
        for (int i = 0; i < 6; i++) {
            if ((voxels[id + offsets[i]] & 1) == 0) {
                enabled_faces |= 1 << i;
            }
        }

        if (enabled_faces > 0) {
            uint original = 0;
            InterlockedAdd(counter[0], countbits(enabled_faces), original);

            uint indexu = 0;
            for (int i = 0; i < 6; i++) {
                if (((enabled_faces >> i) & 1) == 1) {
                    SurfaceData data;

                    data.colors[0] = 0;
                    data.colors[1] = 0;
                    data.colors[2] = 0;
                    data.colors[3] = 0;

                    if (i == 0) {
                        for (int k = 0; k < 4; k++) {
                            data.colors[k] = uint8_t4(hash33(id + k) * 255, 0);
                        }
                    }
               
                    
                    surface_data_buffer[original + indexu] = data;
                    indexu++;
                }
            }

            uint some_magic = original;
            some_magic |= enabled_faces << (31 - 6);
            voxel_indices[id] = some_magic;
        } else {
            voxel_indices[id] = -1;
        }
    }    
}