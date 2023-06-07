COMPUTE

#version 450

layout(set = 0, binding = 0) uniform writeonly image2D outColor;

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

void main() {
	imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), vec4(1.0, 0.0, 0.0, 1.0));
}
