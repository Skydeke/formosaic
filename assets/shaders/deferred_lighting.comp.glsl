#version 310 es
precision mediump float;
precision mediump int;

layout(local_size_x = 32, local_size_y = 32) in;

// G-Buffer — rgba16f (guaranteed image format in GLES 3.1)
layout(rgba16f, binding = 0) uniform highp readonly  image2D gAlbedo;
layout(rgba16f, binding = 1) uniform highp readonly  image2D gNormalMetalness;
layout(rgba16f, binding = 2) uniform highp readonly  image2D gPositionRoughness;
layout(rgba16f, binding = 3) uniform highp writeonly image2D gOutput;

// Must match gl::ClearColor in game_engine
const vec3 BG_COLOR = vec3(0.02, 0.03, 0.05);

void main() {
    ivec2 coord = ivec2(gl_GlobalInvocationID.xy);
    ivec2 size  = imageSize(gAlbedo);
    if (coord.x >= size.x || coord.y >= size.y) { return; }

    vec4 albedoData   = imageLoad(gAlbedo,            coord);
    vec4 posRoughData = imageLoad(gPositionRoughness,  coord);
    vec4 normMetal    = imageLoad(gNormalMetalness,    coord);

    float alpha = albedoData.a;

    // Sky / unwritten pixels — write the background colour so the blit is stable
    if (alpha < 0.01) {
        imageStore(gOutput, coord, vec4(BG_COLOR, 1.0));
        return;
    }

    vec3 albedoColor = albedoData.rgb;
    vec3 N           = normalize(normMetal.xyz);

    // Warm sun from above-left
    vec3 sunDir   = normalize(vec3(0.4, 0.8, 0.4));
    vec3 sunColor = vec3(1.2, 1.1, 0.9);
    vec3 skyColor = vec3(0.5, 0.7, 1.0);

    // Hemisphere ambient
    float upFacing = max(dot(N, vec3(0.0, 1.0, 0.0)), 0.0);
    vec3  ambient  = mix(vec3(0.15), skyColor * 0.4, upFacing * 0.5);

    // Diffuse with wrap-around for softer low-poly shading
    float wrap    = 0.3;
    float diffuse = max((dot(N, sunDir) + wrap) / (1.0 + wrap), 0.0);

    vec3 lighting   = ambient + sunColor * diffuse;
    lighting        = pow(lighting, vec3(0.9)); // mild contrast

    vec3 finalColor = albedoColor * lighting;

    imageStore(gOutput, coord, vec4(finalColor, 1.0));
}
