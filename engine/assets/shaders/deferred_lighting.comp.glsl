#version 310 es
precision mediump float;
precision mediump int;

layout(local_size_x = 32, local_size_y = 32) in;

// G-Buffer — rgba16f (guaranteed image format in GLES 3.1)
layout(rgba16f, binding = 0) uniform highp readonly  image2D gAlbedo;
layout(rgba16f, binding = 1) uniform highp readonly  image2D gNormalMetalness;
layout(rgba16f, binding = 2) uniform highp readonly  image2D gPositionRoughness;
layout(rgba16f, binding = 3) uniform highp writeonly image2D gOutput;

// Lighting config — driven from SceneContext::lights each frame.
uniform vec3  uClearColor;
uniform vec3  uSunDir;
uniform vec3  uSunColor;
uniform vec3  uSkyColor;
uniform float uAmbientMin;

void main() {
    ivec2 coord = ivec2(gl_GlobalInvocationID.xy);
    ivec2 size  = imageSize(gAlbedo);
    if (coord.x >= size.x || coord.y >= size.y) { return; }

    vec4 albedoData = imageLoad(gAlbedo,            coord);
    vec4 normMetal  = imageLoad(gNormalMetalness,    coord);

    float alpha = albedoData.a;

    // Sky / unwritten pixels — write the configured background colour.
    if (alpha < 0.01) {
        imageStore(gOutput, coord, vec4(uClearColor, 1.0));
        return;
    }

    vec3 albedoColor = albedoData.rgb;
    vec3 N           = normalize(normMetal.xyz);
    vec3 sunDir      = normalize(uSunDir);

    // Hemisphere ambient
    float upFacing = max(dot(N, vec3(0.0, 1.0, 0.0)), 0.0);
    vec3  ambient  = mix(vec3(uAmbientMin), uSkyColor * 0.4, upFacing * 0.5);

    // Diffuse with wrap-around for softer low-poly shading
    float wrap    = 0.3;
    float diffuse = max((dot(N, sunDir) + wrap) / (1.0 + wrap), 0.0);

    vec3 lighting = ambient + uSunColor * diffuse;
    lighting      = pow(lighting, vec3(0.9)); // mild contrast

    imageStore(gOutput, coord, vec4(albedoColor * lighting, 1.0));
}
