#version 300 es
precision mediump float;

layout(location = 0) in vec2 pos;  // unit-disc in local XZ

uniform mat4 uVP;
uniform vec3 uDiscCenter;    // world-space centre (model origin)
uniform vec3 uDiscNormal;    // world-space normal (= solution axis)
uniform float uDiscRadius;
uniform float uTime;

out float v_dist;   // distance from disc centre (normalised 0..1)
out float v_time;

// Build a rotation matrix that takes (0,1,0) → uDiscNormal.
mat3 rotationFromNormal(vec3 n) {
    vec3 up  = abs(n.y) < 0.99 ? vec3(0,1,0) : vec3(1,0,0);
    vec3 right = normalize(cross(up, n));
    vec3 fwd   = cross(n, right);
    return mat3(right, n, fwd);
}

void main() {
    mat3 rot = rotationFromNormal(normalize(uDiscNormal));
    vec3 local = vec3(pos.x, 0.0, pos.y) * uDiscRadius;
    vec3 world  = uDiscCenter + rot * local;

    gl_Position = uVP * vec4(world, 1.0);
    v_dist = length(pos);
    v_time = uTime;
}
