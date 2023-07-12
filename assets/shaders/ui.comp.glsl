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
    rectangle.extent = vec2(50, 50);
    rectangle.radius = 25;

    vec2 halfSize = rectangle.extent / 2;
    vec2 center = rectangle.origin + halfSize;
    vec2 pixelPosition = abs(gl_GlobalInvocationID.xy - center);
    vec2 shrunkCornerPosition = halfSize - rectangle.radius;
    vec2 displacement = pixelPosition - shrunkCornerPosition;
    displacement.x = max(0, displacement.x);
    displacement.y = max(0, displacement.y);
    float distance = length(displacement) - rectangle.radius;

    bool hit = distance < 0;
    vec4 color = vec4(vec3(float(hit)), 0.3);

    vec4 renderPixel = imageLoad(renderColor, ivec2(gl_GlobalInvocationID.xy));
    vec4 outputColor = vec4(color.rgb * color.a + renderPixel.rgb * (1 - color.a), 1.0);
    imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), outputColor);
}
