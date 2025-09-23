#version 300 es
precision mediump float;

in vec2 v_uv;
flat in vec3 v_normal;
in vec3 v_pos;

layout(location = 0) out vec4 gAlbedo;            // rgb = albedo, a = alpha
layout(location = 1) out vec4 gNormalMetalness;   // xyz = normal, w = metalness
layout(location = 2) out vec4 gPositionRoughness; // xyz = position, w = roughness

uniform vec3 albedoConst;
uniform sampler2D albedoTex;
uniform int isAlbedoMapped;

float roughness = 0.5;
float metalness = 0.0;

void main() {
    // --- Albedo ---
    vec2 uv = vec2(v_uv.x, 1.0 - v_uv.y);
    vec4 albedo = (isAlbedoMapped == 1) ? texture(albedoTex, uv) : vec4(albedoConst, 1.0);
    if (albedo.a < 0.99) discard;

    gPositionRoughness = vec4(v_pos, roughness);
    gNormalMetalness   = vec4(normalize(v_normal), metalness);
    gAlbedo            = albedo;
}

