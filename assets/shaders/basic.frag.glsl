#version 300 es
precision mediump float;

in vec3 FragColor;
out vec4 FragOutput;

void main() {
    FragOutput = vec4(FragColor, 1.0);
}
