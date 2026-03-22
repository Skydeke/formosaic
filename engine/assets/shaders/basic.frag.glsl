#version 300 es
precision mediump float;

in vec2       v_uv;
flat in vec3  v_normal;
in vec3       v_pos;
in vec4       v_color;

layout(location = 0) out vec4 gAlbedo;
layout(location = 1) out vec4 gNormalMetalness;
layout(location = 2) out vec4 gPositionRoughness;

uniform vec3      albedoConst;
uniform sampler2D albedoTex;
uniform bool      isAlbedoMapped;
uniform bool      uHasVertexColors;

void main() {
    vec4 albedo;
    if (isAlbedoMapped) {
        // GLB/glTF UV origin is top-left, matching OpenGL texture memory layout
        // when loaded top-to-bottom. No V-flip required.
        albedo = texture(albedoTex, v_uv);
    } else if (uHasVertexColors) {
        albedo = v_color;
    } else {
        albedo = vec4(albedoConst, 1.0);
    }

    if (albedo.a < 0.01) discard;

    gAlbedo            = albedo;
    gNormalMetalness   = vec4(normalize(v_normal), 0.0);
    gPositionRoughness = vec4(v_pos, 0.5);
}
