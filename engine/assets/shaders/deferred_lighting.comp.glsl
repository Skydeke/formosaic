#version 310 es
precision highp float;
precision highp int;

layout(local_size_x = 32, local_size_y = 32) in;

// G-Buffer
layout(rgba16f, binding = 0) uniform highp readonly  image2D gAlbedo;
layout(rgba16f, binding = 1) uniform highp readonly  image2D gNormalMetalness;
layout(rgba16f, binding = 2) uniform highp readonly  image2D gPositionRoughness;
layout(rgba16f, binding = 3) uniform highp writeonly image2D gOutput;

// Lighting config
uniform vec3  uClearColor;
uniform vec3  uSunDir;
uniform vec3  uSunColor;
uniform vec3  uSkyColor;
uniform float uAmbientMin;
uniform vec3  uCameraPos;

// ─────────────────────────────────────────────────────────────
vec3 acesFilm(vec3 x) {
    float a = 2.51;
    float b = 0.03;
    float c = 2.43;
    float d = 0.59;
    float e = 0.14;

    return clamp(
        (x * (a * x + b)) /
        (x * (c * x + d) + e),
        0.0,
        1.0
    );
}

void main() {

    ivec2 coord = ivec2(gl_GlobalInvocationID.xy);
    ivec2 size  = imageSize(gAlbedo);

    if (coord.x >= size.x || coord.y >= size.y) {
        return;
    }

    vec4 albedoData   = imageLoad(gAlbedo, coord);
    vec4 normMetal    = imageLoad(gNormalMetalness, coord);
    vec4 posRoughData = imageLoad(gPositionRoughness, coord);

    float alpha = albedoData.a;

    // Sky/background
    if (alpha < 0.01) {
        imageStore(gOutput, coord, vec4(uClearColor, 1.0));
        return;
    }

    vec3 albedo = albedoData.rgb;

    // Keep your dark-material lift
    float albedoLuma = dot(albedo, vec3(0.299, 0.587, 0.114));

    albedo = mix(
        albedo,
        max(albedo, vec3(0.06)),
        1.0 - smoothstep(0.0, 0.15, albedoLuma)
    );

    vec3 N = normalize(normMetal.xyz);

    float metalness = clamp(normMetal.a, 0.0, 1.0);

    vec3 worldPos = posRoughData.rgb;

    float roughness = clamp(posRoughData.a, 0.04, 1.0);

    vec3 sunDir = normalize(uSunDir);

    vec3 V = normalize(uCameraPos - worldPos);

    // ─────────────────────────────────────────────────────────
    // FIXED FRESNEL
    //
    // Non-metals use neutral 4% reflectance.
    // Metals use albedo as reflectance.
    // This removes green grazing-angle shifts.
    // ─────────────────────────────────────────────────────────
    vec3 F0 = mix(
        vec3(0.04),
        albedo,
        metalness
    );

    float NoV = max(dot(N, V), 0.0);

    float fCoef = pow(1.0 - NoV, 5.0);

    vec3 F = F0 + (vec3(1.0) - F0) * fCoef;

    // ─────────────────────────────────────────────────────────
    // Ambient
    // ─────────────────────────────────────────────────────────
    float upFacing =
        dot(N, vec3(0.0, 1.0, 0.0)) * 0.5 + 0.5;

    vec3 skyAmbient =
        uSkyColor * 0.22 * upFacing;

    vec3 groundColor =
        vec3(0.25, 0.20, 0.15);

    vec3 groundAmbient =
        groundColor * 0.3 * (1.0 - upFacing);

    vec3 ambient =
        skyAmbient +
        groundAmbient +
        vec3(uAmbientMin * 0.38);

    // ─────────────────────────────────────────────────────────
    // Diffuse
    // ─────────────────────────────────────────────────────────
    float wrap = 0.3;

    float nDotL =
        (dot(N, sunDir) + wrap) /
        (1.0 + wrap);

    float diffuse =
        max(nDotL, 0.15);

    diffuse =
        diffuse * diffuse *
        (3.0 - 2.0 * diffuse);

    vec3 diffColour =
        diffuse *
        (vec3(1.0) - F);

    // ─────────────────────────────────────────────────────────
    // FIXED SPECULAR
    // ─────────────────────────────────────────────────────────
    vec3 H = normalize(V + sunDir);

    float nDotH = max(dot(N, H), 0.0);

    float vDotH = max(dot(V, H), 0.0);

    float shininess =
        mix(2.0, 64.0, 1.0 - roughness);

    float specular =
        pow(nDotH, shininess) * vDotH;

    vec3 specColour =
        specular *
        F *
        uSunColor;

    vec3 sun =
        diffColour +
        specColour;

    // ─────────────────────────────────────────────────────────
    // Rim
    // ─────────────────────────────────────────────────────────
    vec3 rimDir =
        normalize(-sunDir + vec3(0.0, 0.4, 0.0));

    float rimDot =
        max(dot(N, rimDir), 0.0);

    float rim =
        pow(rimDot, 4.0) * 0.09;

    vec3 rimCol =
        vec3(0.40, 0.50, 0.75) * rim;

    // ─────────────────────────────────────────────────────────
    // Fill
    // ─────────────────────────────────────────────────────────
    float fillDiff =
        max(dot(N, -sunDir), 0.0) * 0.07;

    vec3 fill =
        vec3(0.45, 0.38, 0.30) * fillDiff;

    vec3 lighting =
        ambient +
        sun +
        fill +
        rimCol;

    // Mild saturation
    float luma =
        dot(lighting, vec3(0.299, 0.587, 0.114));

    lighting =
        mix(vec3(luma), lighting, 1.05);

    // KEEP your original final albedo tint
    vec3 colour =
        acesFilm(albedo * lighting);

    // Gamma
    colour =
        pow(
            max(colour, vec3(0.0)),
            vec3(1.0 / 2.2)
        );

    imageStore(
        gOutput,
        coord,
        vec4(colour, alpha)
    );
}
