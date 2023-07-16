#version 450

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D outColor;
layout(set = 0, binding = 1, rgba8) uniform readonly image2D renderColor;

struct Rectangle {
    vec4 color;
    vec2 origin;
    vec2 extent;
    float radius;
};

layout(std140, set = 0, binding = 2) buffer Rectangles {
  int numRectangles;
  Rectangle rectangles[];
} rectangles;

const vec2 EXTENT = vec2(480, 270);

bool hitsRectangle(Rectangle rectangle) {
    vec2 halfSize = rectangle.extent / 2;
    vec2 center = rectangle.origin + halfSize;
    vec2 pixelPosition = abs(gl_GlobalInvocationID.xy - center);
    vec2 shrunkCornerPosition = halfSize - rectangle.radius;
    vec2 displacement = pixelPosition - shrunkCornerPosition;
    displacement.x = max(0, displacement.x);
    displacement.y = max(0, displacement.y);
    float distance = length(displacement) - rectangle.radius;

    return distance < 0;
}

void main() {
    vec4 color = imageLoad(renderColor, ivec2(gl_GlobalInvocationID.xy));
    for (int i = 0; i < rectangles.numRectangles; i++) {
        Rectangle rectangle = rectangles.rectangles[i];
        if (hitsRectangle(rectangle)) {
            color = vec4(rectangle.color.rgb * rectangle.color.a + color.rgb * (1 - rectangle.color.a), rectangle.color.a);
        }
    }

    imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), color);
}
