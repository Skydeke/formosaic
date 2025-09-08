#version 300 es
precision mediump float;

layout(location = 0) in vec3 aPos;
layout(location = 1) in vec3 aColor;

uniform mat4 uVP;     // view-projection (camera)
uniform mat4 uModel;  // model transform (entity/world transform)

out vec3 FragColor;

void main() {
    FragColor = aColor;
    gl_Position = uVP * uModel * vec4(aPos, 1.0);
}
