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
int BOUNCES = 2;
int RAYS_PER_PIXEL = 300;

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
	vec3 albedo;
	float emission;
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

// between -1.0 and 1.0
float random(vec2 s) {
	vec2 seed = vec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y) + s; 
    	return (fract(sin(dot(seed.xy, vec2(12.9898,78.233))) * 43758.5453123) * 2) - 1;
}

vec3 random_unit_sphere(float seed) {
	return normalize(vec3(random(vec2(seed, seed)), random(vec2(seed, seed + 1)), random(vec2(seed, seed + 2))));
}

HitPayload trace_ray(Ray ray, Sphere spheres[3]) {
	float minT = INFINITY;
	HitPayload payload;
	payload.hit = false;
	payload.sphere = 0;
	payload.position = vec3(0.0);

	for (int i = 0; i < 3; i++) {
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

vec3 per_pixel(Ray ray, Sphere spheres[3]) {
	vec3 totalColor = vec3(0.0);
	for (int numRay = 0; numRay < RAYS_PER_PIXEL; numRay++) {
		Ray r = ray;
		vec3 color = vec3(1.0);
		float light = 0.0;
		for (int i = 0; i < BOUNCES; i++) {
			HitPayload hit = trace_ray(r, spheres);
			if (!hit.hit) {
				color *= vec3(0.5294, 0.8078, 0.9216);
				light += 0.7;
				break;
			}

			Sphere sphere = spheres[hit.sphere];
			vec3 normal = normalize(hit.position - sphere.center);
			color *= sphere.albedo;
			light += sphere.emission;

			r.origin = hit.position + normal;
			r.direction = normalize(random_unit_sphere(numRay * BOUNCES + i) + normal);
		}
		totalColor += (color * light) / RAYS_PER_PIXEL;
	}
	return totalColor;
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

	Sphere spheres[3];
	spheres[0].center = vec3(0, 0, 0);
	spheres[0].radius = 50.0;
	spheres[0].albedo = vec3(1.0);
	spheres[0].emission = 3.0;
	spheres[1].center = vec3(100.0, 0.0, 150);
	spheres[1].radius = 50.0;
	spheres[1].albedo = vec3(1.0, 0.0, 1.0);
	spheres[1].emission = 0.0;
	spheres[2].center = vec3(-100.0, 50.0, 25);
	spheres[2].radius = 25.0;
	spheres[2].albedo = vec3(0.0, 1.0, 0.0);
	spheres[2].emission = 0.0;

	vec3 color = per_pixel(ray, spheres);

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
