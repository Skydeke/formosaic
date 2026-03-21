#version 300 es
precision mediump float;

layout (location = 0) in vec3 pos;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec3 norm;

out vec2 v_uv;
flat out vec3 v_normal;
out vec3 v_pos;

uniform mat4 uVP;
uniform mat4 uModel;

void main() {
    // Transform position to world space
    vec4 modelCoord = uModel * vec4(pos, 1.0);
    gl_Position = uVP * modelCoord;

    // Pass to fragment shader
    v_pos = modelCoord.xyz;
    v_uv = uv; // Already correct; no need to swap x/y unless your texture is flipped

    // Transform normal (rotation + uniform scale)
    v_normal = normalize(mat3(uModel) * norm);
}

