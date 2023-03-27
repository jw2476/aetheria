VERTEX
#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec2 inUV;

layout(location = 0) out vec2 fragUV;

layout(binding = 0) uniform Transform {
    mat4 model;
    mat4 view;
    mat4 proj;
} transform;

void main() {
    gl_Position = transform.proj * transform.view * transform.model * vec4(inPosition, 1.0);
    fragUV = inUV;
}