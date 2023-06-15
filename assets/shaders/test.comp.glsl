#version 450

layout(set = 0, binding = 0) uniform writeonly image2D outColor;
layout(set = 0, binding = 1) uniform Camera {
	vec3 eye;
	vec3 target;
} camera;
layout(set = 0, binding = 2) uniform Time {
	float time;
	float delta;
} time;

struct Vertex {
	vec3 position;
};

layout(set = 1, binding = 0) buffer Vertices {
	Vertex vertices[];	
} vertices;

struct Mesh {
	int first_vertex;
	int num_vertices;
	int material;
};

layout(set = 1, binding = 1) buffer Meshes {
	int numMeshes;
	Mesh meshes[];
} meshes;

struct Material {
	vec3 albedo;
	float emission;
	float roughness;
	float metalness;
};

layout(set = 1, binding = 2) buffer Materials {
	Material materials[];
} materials;

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

float INFINITY = 1/0;

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

struct Ray {
	vec3 origin;
	vec3 direction;
};

struct HitPayload {
	bool hit;
	vec3 normal;
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

vec3 random_unit_sphere(float seed) {
	return normalize(vec3(random(vec2(seed, seed)), random(vec2(seed, seed + 1)), random(vec2(seed, seed + 2))));
}

struct TriangleHit {
	bool hit;
	vec3 position;
	float t;
};

float max3(vec3 v) {
	return max(max(v.x, v.y), v.z);
}

TriangleHit triangle_hit(Ray ray, Triangle triangle) {
	vec3 edge1 = triangle.v1.position - triangle.v0.position;
	vec3 edge2 = triangle.v2.position - triangle.v0.position;
	vec3 normal = normalize(cross(edge1, edge2));
	float normalMax = max3(normal);
	vec3 free = vec3(bvec3(normal.x == normalMax, normal.y == normalMax, normal.z == normalMax));
	mat4 transform = inverse(mat4(
		vec4(edge1, 0),
		vec4(edge2, 0),
		vec4(free, 0),
		vec4(triangle.v0.position, 1)
	));

	Ray r;
	r.origin = vec3(transform * vec4(ray.origin, 1.0));
	r.direction = vec3(transform * vec4(ray.direction, 1.0));
	float t = -(r.origin.z/r.direction.z);

	vec3 baryHit = r.origin + t * r.direction;

	TriangleHit hit;
	hit.hit = t >= 0 && 0 <= baryHit.x && baryHit.x <= 1 && 0 <= baryHit.y && baryHit.y <= 1 && baryHit.x + baryHit.y <= 1;
	hit.position = ray.origin + t * ray.direction;
	hit.t = t;
	return hit;
}

HitPayload trace_ray(Ray ray) {
	float minT = INFINITY;
	HitPayload payload;
	payload.hit = false;

	for (int meshIdx = 0; meshIdx < meshes.numMeshes; meshIdx++) {
		Mesh mesh = meshes.meshes[meshIdx];
		for (int vertexIdx = mesh.first_vertex; vertexIdx < mesh.first_vertex + mesh.num_vertices; vertexIdx += 3) {
			Triangle triangle;
			triangle.v0 = vertices.vertices[vertexIdx];
			triangle.v1 = vertices.vertices[vertexIdx + 1];
			triangle.v2 = vertices.vertices[vertexIdx + 2];
			
			TriangleHit hit = triangle_hit(ray, triangle);
			bool overwrite = hit.hit && hit.t < minT && hit.t > 0;
			minT = min(minT, hit.t);
			payload.hit = payload.hit || overwrite;
			payload.position = mix(payload.position, hit.position, float(overwrite));
			vec3 normal = cross(triangle.v1.position - triangle.v0.position, triangle.v2.position - triangle.v0.position);
			payload.normal = mix(payload.normal, normal, float(overwrite));
			payload.material = payload.material * int(!overwrite) + mesh.material * int(overwrite);
		}
	}

	return payload;
}

/*float distributionGGX(vec3 normal, vec3 halfway, float roughness) {
    float a2     = roughness*roughness;
    float NdotH  = max(dot(normal, halfway), 0.0);
    float NdotH2 = NdotH*NdotH;
	
    float nom    = a2;
    float denom  = (NdotH2 * (a2 - 1.0) + 1.0);
    denom        = PI * denom * denom;
	
    return nom / denom;
}

float geometrySchlickGGX(float NdotV, float k) {
    float nom   = NdotV;
    float denom = NdotV * (1.0 - k) + k;
	
    return nom / denom;
}
  
float geometrySmith(vec3 normal, Ray incoming, Ray outgoing, float roughness) {
    float k = (roughness*roughness)/2.0;
    float NdotV = max(dot(normal, outgoing.direction), 0.0);
    float NdotL = max(dot(normal, -incoming.direction), 0.0);
    float ggx1 = geometrySchlickGGX(NdotV, k);
    float ggx2 = geometrySchlickGGX(NdotL, k);
	
    return ggx1*ggx2;
}

vec3 fresnelSchlick(vec3 normal, vec3 halfway, vec3 albedo, float metalness) {
    vec3 F0 = mix(vec3(0.04), albedo, metalness);
    return F0 + (1.0 - F0) * pow(1.0 - dot(normal, halfway), 5.0);
}

vec3 get_brdf(Material material, Ray incoming, Ray outgoing, vec3 normal) {
	vec3 halfway = normalize(-incoming.direction + outgoing.direction);
	float ndf = distributionGGX(normal, halfway, material.roughness);
	float g = geometrySmith(normal, incoming, outgoing, material.roughness);
	vec3 f = fresnelSchlick(normal, halfway, material.albedo, material.metalness);
	vec3 numerator = ndf * g * f;
	float denominator = 4.0 * max(dot(normal, -incoming.direction), 0.0) * max(dot(normal, outgoing.direction), 0.0) + 0.0001;
	vec3 specular = numerator / denominator;
	vec3 kSpecular = f;
	vec3 kDiffuse = vec3(1.0) - kSpecular;
	kDiffuse *= 1.0 - material.metalness;

	return (kDiffuse * material.albedo / PI + specular) * max(dot(normal, outgoing.direction), 0.0);
}	

vec3 per_pixel(Ray incoming) {
	vec3 totalColor = vec3(0.0);
	HitPayload hit = trace_ray(incoming, spheres);
	if (!hit.hit) { return vec3(0.0, 0.0, 0.0); }
	
	Mesh mesh = meshes[hit.sphere];
	Material material = materials[mesh.material];

	if (material.emission != 0) {
		return material.albedo * material.emission;
	}

	vec3 normal = normalize(hit.position - sphere.center);

	int numRays = 0;
	for (int i = 0; i < 5; i++) {
		if (materials[spheres[i].material].emission != 0) {
			Ray outgoing;
			outgoing.origin = hit.position + normal;
			outgoing.direction = normalize(spheres[i].center - outgoing.origin);

			vec3 color = get_brdf(material, incoming, outgoing, normal);

			HitPayload lightHit = trace_ray(outgoing, spheres);

			if (!lightHit.hit) {
				continue;
			}

			Sphere lightSphere = spheres[lightHit.sphere];
			Material lightMaterial = materials[lightSphere.material];

			float distance = length(hit.position - outgoing.origin);
			float light = lightMaterial.emission / (distance*distance);

			totalColor += (color * light);
			numRays++;
		}

	}

	return clamp(totalColor / numRays, vec3(0.0), vec3(1.0));
}

float getPaletteDistance(vec3 a, vec3 b) {
	return length(a - b) * (1 - (0.95 * dot(normalize(a), normalize(b))));
}*/

void main() {
 	vec2 pixelPos = vec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y) - viewport/2;
	Ray ray;
	ray.direction = normalize(camera.target - camera.eye);
	vec3 u = normalize(cross(ray.direction, vec3(0, 1, 0)));
	vec3 v = normalize(cross(ray.direction, u));
	ray.origin = camera.eye + u*pixelPos.x + -v*pixelPos.y;
	
	HitPayload hit = trace_ray(ray);

	/*vec3 color = per_pixel(ray);

	vec4 outputColor = vec4(color, 1.0);
    	float minPaletteDistance = INFINITY;
    	for (int i = 0; i < 32; i++) {
		float paletteDistance = getPaletteDistance(PALETTE[i], color.rgb);
		if (paletteDistance < minPaletteDistance) {
	 		minPaletteDistance = paletteDistance;
			outputColor = vec4(PALETTE[i], 1.0);
	 	}
    	}*/	

	imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), vec4(float(hit.hit)));
}
