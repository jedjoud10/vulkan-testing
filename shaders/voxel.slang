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
    if ((voxels[id - uint3(0, 1, 0)] & 1) == 0 && (voxels[id] & 1) == 1 && id.y > 0) {
        voxels[id - uint3(0, 1, 0)] = voxels[id];
        voxels[id] = 0;
    }

    AllMemoryBarrier();

    uint enabled_faces = calculate_enabled_faces(id, position.xyz, forward.xyz);
    // enabled_faces = 0b100101;

    if (enabled_faces == 0) {
        voxel_indices[id] = -1;
        return;
    }

    if ((voxels[id] & 1) == 1 && enabled_faces > 0) {
        uint original = 0;
        InterlockedAdd(counter[0], countbits(enabled_faces), original);
        uint some_magic = original;
        some_magic |= enabled_faces << (32 - 6);
        voxel_indices[id] = some_magic;

        for (int i = 0; i < countbits(enabled_faces); i++) {
            SurfaceData data;
            data.colors[0] = 0;
            surface_data_buffer[original + i] = data;
        }
    }

    AllMemoryBarrier();

    if ((voxels[id] & 1) == 1 && enabled_faces > 0) {
        uint indexu = 0;
        for (int i = 0; i < 6; i++) {
            if (((enabled_faces >> i) & 1) == 1) {


                /*
                float3 color = 0;
                for (int k = 0; k < 16; k++) {
                    uint2 uv = uint2(k % 4, k / 4);
                    uint3 unflattened = unflatten_uvs(i / 2, i % 2 == 0, uv);
                    float3 world_pos = id + offsets[i] * 0.25 + unflattened / 4.0;
                    // FIXME: still broken btw... on {x,y,z} = 0...
                    float3 shadow_color = 0.0;
                    


                    float3 ray_dir = -normalize(position.xyz - world_pos);
                    float3 reflected = reflect(ray_dir, offsets[i]);


                    
                    uint3 target = dda_shadownate2(voxels, reflected, world_pos);
                }
                */

                /*
                float3 world_pos = id + offsets[i] * 0.25;
                float3 ray_dir = -normalize(position.xyz - world_pos);
                float3 reflected = reflect(ray_dir, offsets[i]);
                uint face = 0;
                float3 dir_sign = sign(ray_dir);
                uint3 target = dda_shadownate2(voxels, reflected, world_pos, face);
                */

                uint3 target = id;
                uint funny_index_magic = voxel_indices[target];
                uint funny_index = funny_index_magic & ~(0b111111 << (32 - 6));
                uint enabled_faces2 = (funny_index_magic >> (32 - 6)) & 0b111111;
                // uint face2 = global_face(face, dir_sign);

                // uint face2 = global_face(face, dir_sign);
                uint face2 = 0;

                uint enabled_faces2_copy = (enabled_faces2 << (6 - face2)) & 0b111111;
                int packed_face_index = countbits(enabled_faces2_copy);

                if (((enabled_faces2 >> face2) & 1) == 1) {
                    SurfaceData data;
                    data.colors[0] = uint8_t4(target, 0);
                    surface_data_buffer[funny_index + packed_face_index] = data;
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