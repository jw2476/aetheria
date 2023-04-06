#version 450

layout(location = 0) in vec3 inPos;
layout(location = 1) in vec2 inUV;
layout(location = 2) in vec3 inNormal;

layout(location = 0) out vec3 fragPos;
layout(location = 1) out vec2 fragUV;
layout(location = 2) out vec3 fragNormal;

layout(set = 0, binding = 0) uniform Camera {
    mat4 view;
    mat4 proj;
} camera;

layout(set = 1, binding = 0) uniform Material {
    vec4 baseColorFactor;
} material;

layout(set = 1, binding = 1) uniform sampler2D baseColorTexture;

layout(set = 2, binding = 0) uniform Mesh {
    mat4 model;
} transform;

void main() {
    gl_Position = camera.proj * camera.view * transform.model * vec4(inPos, 1.0);
    fragPos = (transform.model * vec4(inPos, 1.0)).xyz;
    fragUV = inUV;
    fragNormal = mat3(transpose(inverse(transform.model))) * inNormal;
}