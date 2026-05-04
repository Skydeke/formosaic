#version 300 es
precision highp float;

in vec2 vUV;
out vec4 fragColor;

uniform sampler2D uPosition;   // world-space position G-buffer
uniform sampler2D uAlbedo;     // albedo G-buffer — alpha 0=sky, 1=model
uniform float     uTime;       // solved_timer (0→5 s); negative = not solved

float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

void main() {
    if (uTime < 0.0) discard;

    // Only illuminate actual model pixels — sky pixels have alpha=0
    vec4 albedo = texture(uAlbedo, vUV);
    float modelMask = albedo.a;
    if (modelMask < 0.5) discard;

    // Sample world-space position
    vec3 P = texture(uPosition, vUV).xyz;

    float t = uTime;

    // Band sweeps diagonally across the model in world space
    float bandPos    = (t / 1.2) * 7.0 - 2.0;
    float worldCoord = P.x * 0.6 + P.y;
    float dist       = abs(worldCoord - bandPos);
    float edge       = smoothstep(0.55, 0.0, dist);
    float core       = smoothstep(0.18, 0.0, dist);
    float band       = edge * edge;

    // Fade out after the sweep finishes
    float fade = max(0.0, 1.0 - max(0.0, t - 0.9) / 0.5);

    float shimmer = smoothstep(0.2, 1.0, hash12(P.xz * 9.0 + uTime * 1.7));
    float sparkle = shimmer * core;
    float trail = smoothstep(0.9, 0.0, dist) * 0.35;

    float shine = (band + core * 0.85 + sparkle * 0.5 + trail) * fade;
    if (shine < 0.005) discard;

    // Warm highlight matching the scene sun colour
    vec3 col = vec3(1.0, 0.94, 0.78);
    vec3 glow = vec3(0.55, 0.62, 0.95) * (sparkle * 0.35);
    vec3 target = clamp(col + glow, 0.0, 1.2);
    float intensity = clamp(shine * 0.7, 0.0, 1.0);
    fragColor = vec4(target * intensity, intensity);
}
