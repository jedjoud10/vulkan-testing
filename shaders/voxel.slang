#include <other.slang>

[[vk::binding(0, 0)]]
RWTexture3D<int> voxels;

[shader("compute")]
[numthreads(8, 8, 8)]
void main(uint3 id: SV_DispatchThreadID) {
    int base = (int)(noise((float2)id.xz * 0.1) * 15) + id.y - 15;

    if (hash12(floor(id.xz / 4)) > 0.99) {
        base -= 10;
    }

    voxels[id] = base;
}