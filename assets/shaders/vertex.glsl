VERTEX
#version 450

layout(location = 0) in vec3 inPos;
layout(location = 1) in vec2 inUV;
layout(location = 2) in vec3 inNormal;

layout(location = 0) out vec3 fragPos;
layout(location = 1) out vec2 fragUV;
layout(location = 2) out vec3 fragNormal;

layout(binding = 0) uniform Transform {
    mat4 model;
    mat4 view;
    mat4 proj;
} transform;

void main() {
    gl_Position = transform.proj * transform.view * transform.model * vec4(inPos, 1.0);
    fragPos = (transform.model * vec4(inPos, 1.0)).xyz;
    fragUV = inUV;
    fragNormal = mat3(transpose(inverse(transform.model))) * inNormal;
}