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

// ── ACES-inspired filmic tone-map (operates on linear light) ─────────────
vec3 acesFilm(vec3 x) {
    // Narkowicz 2015 ACES approximation
    float a = 2.51;
    float b = 0.03;
    float c = 2.43;
    float d = 0.59;
    float e = 0.14;
    return clamp((x*(a*x+b))/(x*(c*x+d)+e), 0.0, 1.0);
}

void main() {
    ivec2 coord = ivec2(gl_GlobalInvocationID.xy);
    ivec2 size  = imageSize(gAlbedo);
    if (coord.x >= size.x || coord.y >= size.y) { return; }

    vec4 albedoData = imageLoad(gAlbedo,         coord);
    vec4 normMetal  = imageLoad(gNormalMetalness, coord);

    float alpha = albedoData.a;

    // Sky / unwritten pixels — write the configured background colour.
    if (alpha < 0.01) {
        imageStore(gOutput, coord, vec4(uClearColor, 1.0));
        return;
    }

    vec3 albedo = albedoData.rgb;
    // Lift very dark albedo so models don't vanish into the near-black background.
    // This is a perceptual floor: black geometry gets a small but visible colour.
    float albedoLuma = dot(albedo, vec3(0.299, 0.587, 0.114));
    albedo = mix(albedo, max(albedo, vec3(0.06)), 1.0 - smoothstep(0.0, 0.15, albedoLuma));
    vec3 N      = normalize(normMetal.xyz);
    vec3 sunDir = normalize(uSunDir);

    // ── Hemisphere ambient (sky + warm ground bounce) ─────────────────────
    // upFacing: 1.0 = fully up, 0.0 = fully down
    float upFacing    = dot(N, vec3(0.0, 1.0, 0.0)) * 0.5 + 0.5;
    vec3  skyAmbient  = uSkyColor * 0.22 * upFacing;
    vec3  groundColor = vec3(0.20, 0.15, 0.10);  // subtle warm bounce
    vec3  gndAmbient  = groundColor * uAmbientMin * (1.0 - upFacing);
    vec3  ambient     = skyAmbient + gndAmbient + vec3(uAmbientMin * 0.38);

    // ── Key (sun) light — wrap lighting for soft low-poly shading ────────
    float wrap    = 0.3;
    float nDotL   = (dot(N, sunDir) + wrap) / (1.0 + wrap);
    float diffuse = max(nDotL, 0.0);
    // Sharpen the falloff so lit/shadow boundary pops on flat faces
    diffuse       = diffuse * diffuse * (3.0 - 2.0 * diffuse); // smoothstep
    vec3  sun     = uSunColor * diffuse;

    // ── Cool rim — subtle silhouette pop ─────────────────────────────────
    vec3  rimDir  = normalize(-sunDir + vec3(0.0, 0.4, 0.0));
    float rimDot  = max(dot(N, rimDir), 0.0);
    float rim     = pow(rimDot, 4.0) * 0.09;
    vec3  rimCol  = vec3(0.40, 0.50, 0.75) * rim;

    // ── Warm fill — opposite the sun ─────────────────────────────────────
    float fillDiff = max(dot(N, -sunDir), 0.0) * 0.07;
    vec3  fill     = vec3(0.45, 0.38, 0.30) * fillDiff;

    vec3 lighting = ambient + sun + fill + rimCol;

    // ── HDR colour grading ────────────────────────────────────────────────
    // Very mild saturation nudge
    float luma     = dot(lighting, vec3(0.299, 0.587, 0.114));
    lighting       = mix(vec3(luma), lighting, 1.05);

    // Filmic tone-map
    vec3 colour    = acesFilm(albedo * lighting);

    // sRGB gamma encode
    colour         = pow(max(colour, vec3(0.0)), vec3(1.0 / 2.2));

    imageStore(gOutput, coord, vec4(colour, 1.0));
}
