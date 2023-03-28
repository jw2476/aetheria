FRAGMENT
#version 450

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec2 fragUV;
layout(location = 2) in vec3 fragNormal;

layout(location = 0) out vec4 outColor;

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

vec3 LIGHT_POS = vec3(0.0, 10.0, 10.0);
float AMBIENT_STRENGTH = 0.1;

void main() {
    vec3 normal = normalize(fragNormal);
    vec3 lightDirection = normalize(LIGHT_POS - fragPos);
    float diffuse = max(dot(normal, lightDirection), 0.0);

    float brightness = AMBIENT_STRENGTH + diffuse;
    
    outColor = texture(baseColorTexture, fragUV) * material.baseColorFactor * brightness;
}