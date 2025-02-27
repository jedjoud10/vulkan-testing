[[vk::binding(0, 0)]]
RWTexture3D<uint8_t> voxels;

[shader("compute")]
[numthreads(8, 8, 8)]
void main(uint3 id: SV_DispatchThreadID, uniform float2 screen, uniform matrix<float, 4, 4> mat, uniform float3 position) {
    voxels[id] = 0;
}