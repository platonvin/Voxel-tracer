#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in uint mat;
// layout(location = 2) in uint palette_color;

layout(location=0) out vec3  pos;
// layout(location=1) out vec3  norm;
// layout(location=2) out float mat;

vec3 ray_dir = normalize(vec3(-5, -3.4, 2));
vec3 horizline = normalize(vec3(1,-1,0));
vec3 vertiline = normalize(cross(ray_dir, horizline));
vec3 camera_pos = vec3 (5, 3.4, -2);

void main() {
	float view_width  = 1920 / 128; //in block_diags
	float view_height = 1080 / 128; //in blocks

    vec3 vertexRelativeToCameraPos = position - camera_pos;
    vec3 clip_coords;
    clip_coords.x = dot(vertexRelativeToCameraPos, horizline) / view_width  / 3;
    clip_coords.y = dot(vertexRelativeToCameraPos, vertiline) / view_height / 3;
    clip_coords.z = dot(vertexRelativeToCameraPos, ray_dir) / 1000; //TEMP

    gl_Position  = vec4(clip_coords.xy, clip_coords.z+0.5, 1.0);

    pos = clip_coords + normal/10;
    // pos = position;
    // norm = normal;
    // mat = float(palette_color);
}