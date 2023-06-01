FRAGMENT
#version 450

layout (location = 0) in vec2 inUV;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 0) uniform sampler2D colorTexture;

float SCALE_FACTOR = 15;

void main() {
	outColor = texture(colorTexture, inUV);
}
