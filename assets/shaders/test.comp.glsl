#version 450

layout(set = 0, binding = 0) uniform writeonly image2D outColor;
layout(set = 0, binding = 1) uniform Camera {
	vec3 eye;
	vec3 target;
} camera;

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

vec3 SUN_DIRECTION = vec3(0.0, 4.0, 1.0);
float AMBIENT_STRENGTH = 0.2;
float INFINITY = 1/0;
int BOUNCES = 3;
int RAYS_PER_PIXEL = 100;

vec3 PALETTE[32] = {
vec3(0.5234, 0.0658, 0.0242),
vec3(0.6870, 0.1835, 0.0528),
vec3(0.8277, 0.6661, 0.4098),
vec3(0.7818, 0.3889, 0.1701),
vec3(0.4878, 0.1604, 0.0781),
vec3(0.1734, 0.0446, 0.0370),
vec3(0.0446, 0.0161, 0.0265),
vec3(0.3686, 0.0152, 0.0290),
vec3(0.7818, 0.0399, 0.0546),
vec3(0.9323, 0.1835, 0.0119),
vec3(0.9914, 0.4313, 0.0303),
vec3(0.9914, 0.8046, 0.1193),
vec3(0.1247, 0.5795, 0.0718),
vec3(0.0446, 0.2549, 0.0619),
vec3(0.0152, 0.1062, 0.0511),
vec3(0.0060, 0.0415, 0.0446),
vec3(0.0029, 0.0738, 0.2549),
vec3(0.0000, 0.3250, 0.7155),
vec3(0.0210, 0.8122, 0.9158),
vec3(1.0000, 1.0000, 1.0000),
vec3(0.5356, 0.6055, 0.7227),
vec3(0.2632, 0.3345, 0.4647),
vec3(0.1011, 0.1420, 0.2508),
vec3(0.0385, 0.0546, 0.1332),
vec3(0.0152, 0.0199, 0.0546),
vec3(0.0055, 0.0037, 0.0143),
vec3(1.0000, 0.0000, 0.0546),
vec3(0.1390, 0.0356, 0.1511),
vec3(0.4704, 0.0781, 0.2508),
vec3(0.9240, 0.1801, 0.1975),
vec3(0.8122, 0.4820, 0.3112),
vec3(0.5480, 0.2388, 0.1420)
};

struct Sphere {
	vec3 center;
	float radius;
	int material;
};

struct Material {
	vec3 albedo;
	float emission;
	float roughness;
	float metalness;
};

struct Ray {
	vec3 origin;
	vec3 direction;
};

struct HitPayload {
	bool hit;
	int sphere;
	vec3 position;
};

vec2 viewport = vec2(480, 270);

float _random(float s) {
    	return fract(sin(dot(vec2(s), vec2(12.9898,78.233))) * 43758.5453123);
}

// between -1.0 and 1.0
float random(vec2 s) {
	vec2 seed = vec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y) + s; 
    	return (_random(fract(sin(dot(seed.xy, vec2(12.9898,78.233))) * 43758.5453123)) * 2) - 1;
}

vec3 random_unit_sphere(float seed) {
	return normalize(vec3(random(vec2(seed, seed)), random(vec2(seed, seed + 1)), random(vec2(seed, seed + 2))));
}

HitPayload trace_ray(Ray ray, Sphere spheres[5]) {
	float minT = INFINITY;
	HitPayload payload;
	payload.hit = false;
	payload.sphere = 0;
	payload.position = vec3(0.0);

	for (int i = 0; i < 5; i++) {
		vec3 originToCenter = ray.origin - spheres[i].center;
		float a = dot(ray.direction, ray.direction);
		float half_b = dot(originToCenter, ray.direction);
		float c = dot(originToCenter, originToCenter) - spheres[i].radius*spheres[i].radius;
		float discriminant = half_b*half_b - a*c;
		
		float t = (-half_b - sqrt(abs(discriminant))) / a;
		vec3 hitPoint = ray.origin + ray.direction*t;

		bool overwrite = discriminant >= 0 && t < minT && t >= 0; // Ray hit + closest object so far + in front of camera
		if (overwrite) {
			payload.hit = true;
			payload.position = hitPoint;
			payload.sphere = i;
			minT = t;
		}
	}
	
	return payload;
}

vec3 per_pixel(Ray ray, Sphere spheres[5], Material materials[3]) {
	vec3 totalColor = vec3(0.0);
	for (int numRay = 0; numRay < RAYS_PER_PIXEL; numRay++) {
		Ray r = ray;
		vec3 color = vec3(1.0);
		float light = 0.0;
		for (int i = 0; i < BOUNCES; i++) {
			HitPayload hit = trace_ray(r, spheres);
			if (!hit.hit) {
				if (length(color) == 1.0) {
					color = vec3(0.0, 0.0, 0.0);
				} else {
					light += 0.5;
				} 
				break;
			}
			
			Sphere sphere = spheres[hit.sphere];
			Material material = materials[sphere.material];
			vec3 normal = normalize(hit.position - sphere.center);
			color *= mix(material.albedo, vec3(1.0), material.metalness);
			light += material.emission;

			r.origin = hit.position + normal;
			vec3 scatter = normalize(random_unit_sphere(numRay * BOUNCES + i) + normal);
			vec3 reflection = reflect(ray.direction, normal);
			r.direction = mix(reflection, scatter, material.roughness);
		}
		totalColor += (color * light) / RAYS_PER_PIXEL;
	}
	return clamp(totalColor, vec3(0.0), vec3(1.0));
}

float getPaletteDistance(vec3 a, vec3 b) {
	return length(a - b) * (1 - (0.5 * dot(normalize(a), normalize(b) + 0.5)));
}

void main() {
 	vec2 pixelPos = vec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y) - viewport/2;
	Ray ray;
	ray.direction = normalize(camera.target - camera.eye);
	vec3 u = normalize(cross(ray.direction, vec3(0, 1, 0)));
	vec3 v = normalize(cross(ray.direction, u));
	ray.origin = camera.eye + u*pixelPos.x + -v*pixelPos.y;

	Material materials[3];
	materials[0].albedo = vec3(1.0, 1.0, 1.0);
	materials[0].emission = 10.0;
	materials[0].roughness = 1.0;
	materials[0].metalness = 1.0;

	materials[1].albedo = vec3(0.0, 1.0, 0.0);
	materials[1].emission = 0.0;
	materials[1].roughness = 0.9;
	materials[1].metalness = 0.0;

	materials[2].albedo = vec3(0.6, 0.6, 0.6);
	materials[2].emission = 0.0;
	materials[2].roughness = 1.0;
	materials[2].metalness = 0.0;

	Sphere spheres[5];
	spheres[0].center = vec3(300, 0, 0);
	spheres[0].radius = 50.0;
	spheres[0].material = 0;

	spheres[1].center = vec3(100.0, 0.0, 150);
	spheres[1].radius = 50.0;
	spheres[1].material = 1;

	spheres[2].center = vec3(0, 1000, 0);
	spheres[2].radius = 970.0;
	spheres[2].material = 2;

	spheres[3].center = vec3(100, -300.0, 50);
	spheres[3].radius = 25.0;
	spheres[3].material = 0;

	spheres[4].center = vec3(30.0, 50.0, 12);
	spheres[4].radius = 25.0;
	spheres[4].material = 1;

	vec3 color = per_pixel(ray, spheres, materials);

	vec4 outputColor = vec4(color, 1.0);
    	float minPaletteDistance = INFINITY;
    	for (int i = 0; i < 32; i++) {
		float paletteDistance = getPaletteDistance(PALETTE[i], color.rgb);
		if (paletteDistance < minPaletteDistance) {
	 		minPaletteDistance = paletteDistance;
			outputColor = vec4(PALETTE[i], 1.0);
	 	}
    	}	

	imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), outputColor);
}
