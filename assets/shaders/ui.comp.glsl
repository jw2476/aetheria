#version 450

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D outColor;
layout(set = 0, binding = 1, rgba8) uniform readonly image2D renderColor;
layout(set = 0, binding = 2) uniform sampler2D fontAtlas;

struct Rectangle {
    vec4 color;
    uvec2 origin;
    uvec2 extent;
    uint radius;
    int atlasID;
};

layout(std140, set = 0, binding = 3) buffer Rectangles {
  int numRectangles;
  Rectangle rectangles[];
} rectangles;

const vec2 EXTENT = vec2(480, 270);

bool hitsRectangle(Rectangle rectangle) {
    vec2 halfSize = rectangle.extent / 2;
    vec2 positionWithRectOrigin = abs(gl_GlobalInvocationID.xy - (rectangle.origin + halfSize));
    vec2 positionWithCornerOrigin = positionWithRectOrigin - (halfSize - vec2(rectangle.radius));
    vec2 displacement = vec2(
        max(0, positionWithCornerOrigin.x),
        max(0, positionWithCornerOrigin.y)
    );
    float distance = length(displacement);
    return distance <= rectangle.radius;
}

void main() {
    vec4 color = imageLoad(renderColor, ivec2(gl_GlobalInvocationID.xy));

    for (int i = 0; i < rectangles.numRectangles; i++) {
        Rectangle rectangle = rectangles.rectangles[i];
        if (hitsRectangle(rectangle)) {
            float glyph = 1;
            uvec2 fromOrigin = uvec2(gl_GlobalInvocationID.xy) - rectangle.origin;

            if (rectangle.atlasID != -1) {
                glyph = length(texture(fontAtlas, fromOrigin + uvec2(6 * rectangle.atlasID, 0)).rgb);
                glyph = min(glyph, 1.0);
            }

            float alpha = rectangle.color.a * glyph;
            color = vec4(rectangle.color.rgb * alpha + color.rgb * (1 - alpha), alpha);
            //color = vec4(rectangle.color.rgb * glyph + color.rgb * (1 - glyph), glyph);
        }
    }

    color.r = pow(color.r, 2.2);
    color.g = pow(color.g, 2.2);
    color.b = pow(color.b, 2.2);

    imageStore(outColor, ivec2(gl_GlobalInvocationID.xy), color);
}
