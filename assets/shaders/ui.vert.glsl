VERTEX
#version 450

layout(location = 0) in vec2 inPos;
layout(location = 1) in vec4 inColor;
layout(location = 2) in vec2 inUV;

layout(location = 0) out vec2 fragPos;
layout(location = 1) out vec4 fragColor;
layout(location = 2) out vec2 fragUV;

vec3 srgb_to_linear(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 lower = srgb / vec3(12.92);
    vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
    return mix(higher, lower, cutoff);
}

void main() {
    gl_Position = vec4(inPos, 0.0, 1.0);
    fragPos = inPos;
    fragColor = vec4(srgb_to_linear(inColor.rgb), inColor.a);
    fragUV = inUV;
}