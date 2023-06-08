COMPUTE

#version 450

layout(set = 0, binding = 0) uniform writeonly image2D outColor;

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

vec3 CENTER = vec3(1920.0 / 2.0, 1080.0 / 2.0, 100.0);
float RADIUS = 400.0;

vec3 SUN_DIRECTION = vec3(0.0, -1.0, -1.0);
float AMBIENT_STRENGTH = 0.0;

void main() {
 	vec2 pixelPos = vec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y);
	vec3 rayOrigin = vec3(pixelPos, 0.0);
	vec3 rayDirection = vec3(0.0, 0.0, 1.0);
	
	vec3 originToCenter = rayOrigin - CENTER;
	float a = dot(rayDirection, rayDirection);
	float b = 2.0 * dot(originToCenter, rayDirection);
	float c = dot(originToCenter, originToCenter) - RADIUS*RADIUS;
	float discriminant = b*b - 4*a*c;
	
	float t = (-b-sqrt(abs(discriminant))) / (2.0*a);
	vec3 hitPoint = rayOrigin + rayDirection*t;
	vec3 normal = normalize(hitPoint - CENTER);

	float sun = max(dot(normal, normalize(SUN_DIRECTION)), 0.0);
	float brightness = AMBIENT_STRENGTH + sun;

	vec3 color = vec3(brightness * float(discriminant >= 0), 0.0, 0.0);
	mat3 invert = mat3(vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0));
	imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), vec4(invert * color, 1.0));
}
