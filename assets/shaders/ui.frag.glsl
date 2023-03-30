FRAGMENT
#version 450

layout(location = 0) in vec2 fragPos;
layout(location = 1) in vec4 fragColor;
layout(location = 2) in vec2 fragUV;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 0) uniform sampler2D colorTexture;

void main() {
    outColor = texture(colorTexture, fragUV) * fragColor;
}