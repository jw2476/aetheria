COMPUTE

#version 450

layout(set = 0, binding = 0) uniform writeonly image2D outColor;
layout(set = 0, binding = 1) uniform Camera {
	vec3 eye;
	vec3 target;
} camera;

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

vec3 SUN_DIRECTION = vec3(0.0, 1.0, 1.0);
float AMBIENT_STRENGTH = 0.2;
float INFINITY = 1/0;

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

struct Sphere {
	vec3 center;
	float radius;
	vec4 color;
};

struct Ray {
	vec3 origin;
	vec3 direction;
};

void main() {
 	vec2 pixelPos = vec2(gl_GlobalInvocationID.x * 4, gl_GlobalInvocationID.y * 4);
	Ray ray;
	ray.origin = vec3(pixelPos, 0.0) + camera.eye;
	ray.direction = vec3(0, 0, 1);

	Sphere spheres[3];
	spheres[0].center = vec3(1920/2, 1080/2, 100);
	spheres[0].radius = 600.0;
	spheres[0].color = vec4(1.0);
	spheres[1].center = vec3(400.0, 300.0, 50);
	spheres[1].radius = 50.0;
	spheres[1].color = vec4(1.0, 0.0, 1.0, 1.0);
	spheres[2].center = vec3(1500.0, 800.0, 25);
	spheres[2].radius = 25.0;
	spheres[2].color = vec4(0.0, 1.0, 0.0, 1.0);

	vec4 color = vec4(0.0);

	for (int i = 0; i < 3; i++) {
		vec3 originToCenter = ray.origin - spheres[i].center;
		float a = dot(ray.direction, ray.direction);
		float half_b = dot(originToCenter, ray.direction);
		float c = dot(originToCenter, originToCenter) - spheres[i].radius*spheres[i].radius;
		float discriminant = half_b*half_b - a*c;
		
		float t = (-half_b - sqrt(abs(discriminant))) / a;
		vec3 hitPoint = ray.origin + ray.direction*t;
		vec3 normal = normalize(hitPoint - spheres[i].center);

		float sun = max(dot(normal, normalize(-SUN_DIRECTION)), 0.0);
		float brightness = AMBIENT_STRENGTH + sun;
		
		color = color * float(discriminant < 0);
		color += spheres[i].color * brightness * float(discriminant >= 0);
	}


	vec4 outputColor;
    	float minPaletteLength = INFINITY;
    	for (int i = 0; i < 32; i++) {
		if (length(PALETTE[i] - color.rgb) < minPaletteLength) {
	 		minPaletteLength = length(PALETTE[i] - color.rgb);
			outputColor = vec4(PALETTE[i], color.a);
	 	}
    	}

	mat3 invert = mat3(vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0));
	
	for (int x = 0; x < 4; x++) {
		for (int y = 0; y < 4; y++) {
			imageStore(outColor, ivec2(gl_GlobalInvocationID.xy * 4 + vec2(x, y)), vec4(invert * outputColor.rgb, outputColor.a));
		}
	}
}
