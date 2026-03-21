#version 300 es
precision highp float;

in vec2 vUV;
out vec4 fragColor;

uniform sampler2D uPosition;   // world-space position G-buffer
uniform sampler2D uAlbedo;     // albedo G-buffer — alpha 0=sky, 1=model
uniform float     uTime;       // solved_timer (0→5 s); negative = not solved

void main() {
    if (uTime < 0.0) discard;

    // Only illuminate actual model pixels — sky pixels have alpha=0
    float modelMask = texture(uAlbedo, vUV).a;
    if (modelMask < 0.5) discard;

    // Sample world-space position
    vec3 P = texture(uPosition, vUV).xyz;

    float t = uTime;

    // Band sweeps diagonally across the model in world space
    float bandPos    = (t / 1.2) * 7.0 - 2.0;
    float worldCoord = P.x * 0.6 + P.y;
    float dist       = abs(worldCoord - bandPos);
    float band       = max(0.0, 1.0 - dist / 0.5);
    band             = band * band;

    // Fade out after the sweep finishes
    float fade = max(0.0, 1.0 - max(0.0, t - 0.9) / 0.5);

    float shine = band * fade;
    if (shine < 0.005) discard;

    // Warm highlight matching the scene sun colour
    vec3 col = vec3(1.0, 0.94, 0.78);
    fragColor = vec4(col, shine * 0.7);
}
