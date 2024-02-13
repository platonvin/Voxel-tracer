    #version 460

layout(location = 0) out vec4 pos_mat_out;
// layout(location=0) in vec3  pos;
// layout(location=1) in vec3  norm;
// layout(location=2) in float mat;

// layout(location = 1) out vec4 norm_out;

void main() {
    // pos_mat_out == vec4(pos, mat);
    pos_mat_out = vec4(1.0, 0.0, 0.0, 1.0);
    // norm_out = vec4(norm, 1);
    // f_color = vec4(1.0, 0.0, 0.0, 1.0);
}