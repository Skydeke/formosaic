#version 300 es
precision mediump float;

// Full-screen quad positions (NDC, -1..1)
layout(location = 0) in vec2 pos;

out vec2 v_uv;

void main() {
    gl_Position = vec4(pos, 0.0, 1.0);
    v_uv = pos * 0.5 + 0.5;
}
