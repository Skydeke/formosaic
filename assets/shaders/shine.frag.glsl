#version 300 es
precision highp float;

in vec2 vUV;
out vec4 fragColor;

uniform sampler2D uPosition;   // world-space position G-buffer
uniform float     uTime;       // solved_timer (0→5 s); negative = not solved

void main() {
    if (uTime < 0.0) discard;

    // Sample world-space position
    vec3 P = texture(uPosition, vUV).xyz;

    // If position is zero (sky / unwritten pixel) skip
    if (dot(P, P) < 0.0001) discard;

    float t = uTime;

    // Band sweeps diagonally across the model in world space
    float bandPos   = (t / 1.2) * 7.0 - 2.0;
    float worldCoord = P.x * 0.6 + P.y;
    float dist       = abs(worldCoord - bandPos);
    float bandWidth  = 0.5;
    float band       = max(0.0, 1.0 - dist / bandWidth);
    band             = band * band;

    // Fade out after the sweep finishes
    float fade = max(0.0, 1.0 - max(0.0, t - 0.9) / 0.5);

    float shine = band * fade;
    if (shine < 0.005) discard;

    // Warm highlight matching the scene's sun colour (1.2, 1.1, 0.9)
    vec3 col = vec3(1.0, 0.94, 0.78);
    fragColor = vec4(col, shine * 0.7);
}
