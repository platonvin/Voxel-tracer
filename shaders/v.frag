#version 460

layout(location=0) in vec3 pos_mid;
// layout(location=1) in vec3  norm;
// layout(location=2) in float mat;

layout(location = 0) out vec4 pos_mat_out;
// layout(location = 1) out vec4 norm_out;

void main() {
    // pos_mat_out == vec4(pos, mat);
    // vec3 v = pow(pos, vec3(4.0));
    // vec3 v;
    // if (pos.x < 0.33) {
    //     v.x = .6;
    // } else if (pos.x > 0.1) {
    //     v.y = .2;
    // }
    // if (pos.x < 0.66) {
    //     v.z = .88;
    // } else if (pos.x > .22) {
    //     v.y += 0.1;
    // }
    pos_mat_out = vec4(pos_mid, 1.0);
    // norm_out = vec4(norm, 1);
    // f_color = vec4(1.0, 0.0, 0.0, 1.0);
}