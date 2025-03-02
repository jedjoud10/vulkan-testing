#include <other.slang>

[[vk::binding(0, 0)]]
RWTexture3D<uint8_t> voxels;

[shader("compute")]
[numthreads(8, 8, 8)]
void main(uint3 id: SV_DispatchThreadID) {
    int base = (int)(noise((float2)id.xz * 0.1) * 15) + id.y - 15;
    bool reflective = false;
    bool refractive = false;


    uint8_t raw = 0;

    if (hash12(floor(id.xz / 4)) > 0.99) {
        if (hash13(id) > 0.5) {
            base -= 10;
        }

        if (hash12(id.xz) > 0.5) {
            refractive = true;
        } else {
            reflective = true;
        }
    }
    
    bool active = base < 0;
    raw |= active ? 1 : 0;
    raw |= reflective ? 2 : 0;
    raw |= refractive ? 4 : 0;
    voxels[id] = raw;
}