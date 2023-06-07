COMPUTE

#version 450

layout(set = 0, binding = 0) uniform writeonly image2D outColor;

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

void main() {
	
	vec3 color = vec3(gl_GlobalInvocationID.x / 1920.0, gl_GlobalInvocationID.y / 1080.0, 0.0);
	mat3 invert = mat3(vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0));
	imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), vec4(invert * color, 1.0));
}
