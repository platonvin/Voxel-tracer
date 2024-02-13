#version 460

layout(location = 0) in vec2 position;
// layout(location = 0) in vec3 position;
// layout(location = 2) in vec3 normal;
// layout(location = 1) in uint palette_color;

// layout(location=0) out vec3  pos;
// layout(location=1) out vec3  norm;
// layout(location=2) out float mat;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);

    // pos = position;
    // norm = normal;
    // mat = float(palette_color);
}