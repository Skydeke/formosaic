#version 300 es
precision mediump float;

layout (location = 0) in vec3 pos;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec3 norm;
layout (location = 5) in vec4 vertColor;

out vec2 v_uv;
flat out vec3 v_normal;
out vec3 v_pos;
out vec4 v_color;

uniform mat4 uVP;
uniform mat4 uModel;

void main() {
    vec4 worldPos = uModel * vec4(pos, 1.0);
    gl_Position   = uVP   * worldPos;
    v_pos    = worldPos.xyz;
    v_uv     = uv;
    v_normal = normalize(mat3(uModel) * norm);
    v_color  = vertColor;
}
