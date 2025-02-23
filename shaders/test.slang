[[vk::binding(0, 0)]]
RWTexture2D<float4> output;

[shader("compute")]
[numthreads(32, 32, 1)]
void main(uint3 thread_id: SV_DispatchThreadID) {
    output[thread_id.xy] = float4((float2)thread_id.xy / 100.0, 0, 0);
}