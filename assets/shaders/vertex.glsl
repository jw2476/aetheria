VERTEX
#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inUV;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 fragUV;

layout(binding = 0) uniform Transform {
    mat4 model;
    mat4 view;
    mat4 proj;
} transform;

void main() {
    gl_Position = transform.proj * transform.view * transform.model * vec4(inPosition, 0.0, 1.0);
    fragColor = inColor;
    fragUV = inUV;
}