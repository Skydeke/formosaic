#version 300 es
precision mediump float;

in vec2       v_uv;
flat in vec3  v_normal;
in vec3       v_pos;

layout(location = 0) out vec4 gAlbedo;
layout(location = 1) out vec4 gNormalMetalness;
layout(location = 2) out vec4 gPositionRoughness;

uniform vec3      albedoConst;
uniform sampler2D albedoTex;
uniform int       isAlbedoMapped;

void main() {
    vec2 uv     = vec2(v_uv.x, 1.0 - v_uv.y);
    vec4 albedo = (isAlbedoMapped == 1) ? texture(albedoTex, uv) : vec4(albedoConst, 1.0);
    if (albedo.a < 0.99) discard;

    gAlbedo            = albedo;
    gNormalMetalness   = vec4(normalize(v_normal), 0.0);
    gPositionRoughness = vec4(v_pos, 0.5);
}
