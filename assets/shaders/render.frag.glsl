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
	vec4 baseColor;
} material;

vec3 LIGHT_POS = vec3(0.0, 10.0, 10.0);
float AMBIENT_STRENGTH = 0.1;
float INFINITY = 1.0 / 0.0;

vec3 PALETTE[32] = {
	vec3(0.7451, 0.2902, 0.1843),
	vec3(0.8431, 0.4627, 0.2627),
	vec3(0.9176, 0.8314, 0.6667),
	vec3(0.8941, 0.6510, 0.4471),
	vec3(0.7216, 0.4353, 0.3137),
	vec3(0.4510, 0.2431, 0.2235),
	vec3(0.2431, 0.1529, 0.1922),
	vec3(0.6353, 0.1490, 0.2000),
	vec3(0.8941, 0.2314, 0.2667),
	vec3(0.9686, 0.4627, 0.1333),
	vec3(0.9961, 0.6824, 0.2039),
	vec3(0.9961, 0.9059, 0.3804),
	vec3(0.3882, 0.7804, 0.3020),
	vec3(0.2431, 0.5373, 0.2824),
	vec3(0.1490, 0.3608, 0.2588),
	vec3(0.0980, 0.2353, 0.2431),
	vec3(0.0706, 0.3059, 0.5373),
	vec3(0.0000, 0.6000, 0.8588),
	vec3(0.1725, 0.9098, 0.9608),
	vec3(1.0000, 1.0000, 1.0000),
	vec3(0.7529, 0.7961, 0.8627),
	vec3(0.5451, 0.6078, 0.7059),
	vec3(0.3529, 0.4118, 0.5333),
	vec3(0.2275, 0.2667, 0.4000),
	vec3(0.1490, 0.1686, 0.2667),
	vec3(0.0941, 0.0784, 0.1451),
	vec3(1.0000, 0.0000, 0.2667),
	vec3(0.4078, 0.2196, 0.4235),
	vec3(0.7098, 0.3137, 0.5333),
	vec3(0.9647, 0.4588, 0.4784),
	vec3(0.9098, 0.7176, 0.5882),
	vec3(0.7608, 0.5216, 0.4118)
};

void main() {
    vec4 baseColor = material.baseColor;
    vec3 normal = normalize(fragNormal);
    vec3 lightDirection = normalize(LIGHT_POS - fragPos);
    float diffuse = max(dot(normal, lightDirection), 0.0);

    float brightness = AMBIENT_STRENGTH + diffuse;
    
    
    vec3 color = (baseColor * brightness).rgb;
    float minPaletteLength = INFINITY;
    for (int i = 0; i < 32; i++) {
	 if (length(PALETTE[i] - color) < minPaletteLength) {
	 	minPaletteLength = length(PALETTE[i] - color);
		outColor = vec4(PALETTE[i], baseColor.a);
	 }
    }
}
