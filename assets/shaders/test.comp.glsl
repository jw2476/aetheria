#version 450

layout(set = 0, binding = 0) uniform Camera {
	vec3 eye;
	vec3 target;
} camera;
layout(set = 0, binding = 1) uniform Time {
	float time;
	float delta;
} time;

struct Vertex {
	vec3 position;
	vec3 normal;
};

layout(set = 1, binding = 0) uniform writeonly image2D outColor;
layout(std140, set = 1, binding = 1) buffer Vertices {
	Vertex vertices[];	
} vertices;

layout(std140, set = 1, binding = 2) buffer Indicies {
	int indicies[];
} indicies;

struct Mesh {
	int first_index;
	int num_indicies;
	int material;
	vec3 minAABB;
	vec3 maxAABB;
	mat4 transform;
};

layout(std140, set = 1, binding = 3) buffer Meshes {
	int numMeshes;
	Mesh meshes[];
} meshes;

layout(std140, set = 1, binding = 4) buffer Materials {
	vec3 materials[];
} materials;


struct Light {
	vec3 position;
	float strength;
	vec3 color;
};

layout(std140, set = 1, binding = 5) buffer Lights {
	int numLights;
	Light lights[];
} lights;

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

float INFINITY = 1.0/0.0;
float EPSILON = 0.000001;
vec3 AMBIENT = vec3(0.094, 0.094, 0.106);

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
	vec3(0.7608, 0.5216, 0.4118),
};

struct Ray {
	vec3 origin;
	vec3 direction;
};

struct HitPayload {
	bool hit;
	vec3 normal;
  int mesh;
	int material;
	vec3 position;
	float t;
};

struct Triangle {
	Vertex v0;
	Vertex v1;
	Vertex v2;
};

vec2 viewport = vec2(480, 270);
const float PI = 3.14159265359;

float _random(float s) {
    	return fract(sin(dot(vec2(s), vec2(12.9898,78.233))) * 43758.5453123);
}

// between -1.0 and 1.0
float random(vec2 s) {
	vec2 seed = vec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y) + s + time.time; 
    	return (_random(fract(sin(dot(seed.xy, vec2(12.9898,78.233))) * 43758.5453123)) * 2) - 1;
}

float random_zero_one(vec2 s) {	
	vec2 seed = vec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y) + s + time.time; 
    	return _random(fract(sin(dot(seed.xy, vec2(12.9898,78.233))) * 43758.5453123));
}

vec3 random_unit_sphere(float seed) {
	return normalize(vec3(random(vec2(seed, seed)), random(vec2(seed, seed + 1)), random(vec2(seed, seed + 2))));
}

struct TriangleHit {
	bool hit;
	vec3 position;
	vec3 normal;
	float t;
};

TriangleHit triangle_hit(Ray ray, Triangle triangle) {	
	TriangleHit hit;
	hit.hit = false;

	vec3 a = inverse(mat3(
		-ray.direction, 
		triangle.v0.position - triangle.v2.position, 
		triangle.v1.position - triangle.v2.position)
	) * (ray.origin - triangle.v2.position);

	vec4 b = vec4(a, 1.0 - a.y - a.z);

	hit.hit = all(greaterThanEqual(b, vec4(0.0))) && all(lessThanEqual(b.yzw, vec3(1.0)));
	hit.position = ray.origin + b.x * ray.direction;
	hit.normal = triangle.v0.normal * b.y + triangle.v1.normal * b.z + triangle.v2.normal * b.w;
	hit.t = b.x;
	return hit;
}

Vertex vertex_transform(Vertex v, mat4 transform) {
	v.position = vec3(transform * vec4(v.position, 1.0));
	v.normal = mat3(transpose(inverse(transform))) * v.normal;
	return v;
}

bool intersects_box(Ray ray, vec3 bmin, vec3 bmax) {
	float tx1 = (bmin.x - ray.origin.x) / ray.direction.x, tx2 = (bmax.x - ray.origin.x) / ray.direction.x;
	float tmin = min( tx1, tx2 ), tmax = max( tx1, tx2 );
	float ty1 = (bmin.y - ray.origin.y) / ray.direction.y, ty2 = (bmax.y - ray.origin.y) / ray.direction.y;
	tmin = max( tmin, min( ty1, ty2 ) ), tmax = min( tmax, max( ty1, ty2 ) );
	float tz1 = (bmin.z - ray.origin.z) / ray.direction.z, tz2 = (bmax.z - ray.origin.z) / ray.direction.z;
	tmin = max( tmin, min( tz1, tz2 ) ), tmax = min( tmax, max( tz1, tz2 ) );
	return tmax >= tmin && tmax > 0;
}

HitPayload trace_ray(Ray ray, vec3 target) {
	HitPayload payload;
	payload.hit = false;
  payload.mesh = -1;
	payload.material = -1;
	payload.position = vec3(0.0);
	payload.normal = vec3(0.0);

	float minT = INFINITY;

	for (int meshIdx = 0; meshIdx < meshes.numMeshes; meshIdx++) {
		Mesh mesh = meshes.meshes[meshIdx];
		
    if (!intersects_box(ray, mesh.minAABB, mesh.maxAABB)) { continue; }
    if (all(greaterThan(target, mesh.minAABB)) && all(lessThan(target, mesh.maxAABB))) { continue; }

		for (int indexIdx = mesh.first_index; indexIdx < (mesh.first_index + mesh.num_indicies); indexIdx += 3) {
			Triangle triangle;
			triangle.v0 = vertices.vertices[indicies.indicies[indexIdx]];
			triangle.v1 = vertices.vertices[indicies.indicies[indexIdx + 1]];
			triangle.v2 = vertices.vertices[indicies.indicies[indexIdx + 2]];
			triangle.v0 = vertex_transform(triangle.v0, mesh.transform);
			triangle.v1 = vertex_transform(triangle.v1, mesh.transform);
			triangle.v2 = vertex_transform(triangle.v2, mesh.transform);
			
			TriangleHit hit = triangle_hit(ray, triangle);
			bool overwrite = hit.hit && hit.t < minT;
				
			if (overwrite) {
				minT = hit.t;
				payload.hit = true;
        payload.mesh = meshIdx;
				payload.material = mesh.material;
				payload.position = hit.position;
				payload.normal = normalize(hit.normal);
			}
			
			/*minT = minT * float(!overwrite) + hit.t * float(overwrite);
			payload.hit = payload.hit || hit.hit;
			payload.material = payload.material * int(!overwrite) + mesh.material * int(overwrite);
			payload.position = mix(payload.position, hit.position, float(overwrite));
			payload.normal = mix(payload.normal, normalize(hit.normal), float(overwrite));*/
		}
	}

	payload.t = minT;
	return payload;
}

vec3 per_pixel(Ray incoming) {
	HitPayload hit = trace_ray(incoming, vec3(INFINITY));
	if (!hit.hit) { return vec3(0.0, 0.0, 0.0); }

  Mesh mesh = meshes.meshes[hit.mesh];
	vec3 material = materials.materials[hit.material];
	
	Ray outgoing;
	outgoing.origin = hit.position + hit.normal;

	vec3 diffuse = vec3(0.0);

	for (int i = 0; i < lights.numLights; i++) {
		Light light = lights.lights[i];
		float distance = length(light.position - hit.position);

		outgoing.direction = normalize(light.position - hit.position); 

		float lightContribution;
		lightContribution = light.strength / (distance*distance);
		lightContribution = min(lightContribution, 1.0);

		if (lightContribution < 0.125) { continue; }

		lightContribution *= max(dot(hit.normal, outgoing.direction), 0.0);
		lightContribution = round(lightContribution * 4) / 4;

		HitPayload hit2 = trace_ray(outgoing, light.position);
		bool lightVisible = !hit2.hit || (hit2.t > distance);
		diffuse += light.color * lightContribution * (0.4 + 0.8 * float(lightVisible));
	}

	if (length(diffuse) < 0.05) { diffuse = AMBIENT; }
	vec3 color = material * diffuse;

	if (length(color) > 1.0) { color = normalize(color); }
	return color;
}

float ZOOM = 1 / 0.8;

void main() {
 	vec2 pixelPos = vec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y) - viewport/2;
	Ray ray;
	ray.direction = normalize(camera.target - camera.eye);
	vec3 u = normalize(cross(ray.direction, vec3(0, 1, 0)));
	vec3 v = normalize(cross(ray.direction, u));
	ray.origin = camera.eye - u*pixelPos.x * ZOOM + v*pixelPos.y * ZOOM;
	
	vec3 color = per_pixel(ray);
	vec4 outputColor = vec4(color, 1.0);

	imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), outputColor);
}
