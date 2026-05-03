#version 300 es
precision highp float;
precision highp int;

in vec3       v_pos;
in vec2       v_uv;
flat in vec3  v_normal;
in vec4       v_color;

layout(location = 0) out vec4 gAlbedo;
layout(location = 1) out vec4 gNormalMetalness;
layout(location = 2) out vec4 gPositionRoughness;

uniform vec3      albedoConst;
uniform sampler2D albedoTex;
uniform bool      isAlbedoMapped;
uniform bool      uHasVertexColors;
uniform vec3      uCameraPos;
uniform float     uMetallicFactor;
uniform float     uRoughnessFactor;

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

    vec3 N = normalize(v_normal);

    // Output to G-Buffer
    gNormalMetalness = vec4(normalize(N), uMetallicFactor);
    gPositionRoughness = vec4(v_pos, uRoughnessFactor);
}
