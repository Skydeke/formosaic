#version 300 es
precision mediump float;

layout(location = 0) in vec3 pos;
layout(location = 2) in vec3 norm;

out vec3  v_normal_ws;
out vec3  v_pos_ws;
out float v_outline_intensity;

uniform mat4  uVP;
uniform mat4  uModel;
uniform float uOutlineWidth;
uniform float uTime;

void main() {
    vec3 world_norm    = normalize(mat3(uModel) * norm);
    vec4 world_pos     = uModel * vec4(pos, 1.0);
    world_pos.xyz     += world_norm * uOutlineWidth;
    gl_Position        = uVP * world_pos;
    v_normal_ws        = world_norm;
    v_pos_ws           = world_pos.xyz;
    v_outline_intensity = 0.6 + 0.4 * sin(uTime * 3.0);
}
