#version 450

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D outColor;
layout(set = 0, binding = 1, rgba8) uniform readonly image2D renderColor;

const vec2 EXTENT = vec2(480, 270);

struct Rectangle {
    vec2 origin;
    vec2 extent;
    float radius;
};

void main() {
    Rectangle rectangle;
    rectangle.origin = vec2(100, 100);
    rectangle.extent = vec2(50, 25);
    rectangle.radius = 8;

    vec2 pixelPos = gl_GlobalInvocationID.xy - rectangle.origin; // Center on rectangle origin
    pixelPos = vec2(abs(pixelPos.x), abs(pixelPos.y)); // Limit to top-right quadrant
    pixelPos -= rectangle.extent / 2; // Align rectangle to origin
    pixelPos -= vec2(rectangle.radius);
    vec2 displacement = vec2(max(0, pixelPos.x), max(0, pixelPos.y));
    float distance = length(displacement);
    
    bool hit = distance < rectangle.radius;
    vec4 color = vec4(vec3(float(hit)), 0.3);

    vec4 renderPixel = imageLoad(renderColor, ivec2(gl_GlobalInvocationID.xy));
    vec4 outputColor = vec4(color.rgb * color.a + renderPixel.rgb * (1 - color.a), 1.0);
    imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), outputColor);
}