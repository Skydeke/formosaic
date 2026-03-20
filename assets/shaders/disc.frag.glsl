#version 300 es
precision mediump float;

in float v_dist;
in float v_time;

out vec4 fragColor;

void main() {
    if (v_dist > 1.0) discard;

    float d    = v_dist;
    float t    = v_time;

    // ── Outer dashed ring ─────────────────────────────────────────────────
    float ring = smoothstep(0.92, 0.88, d) * smoothstep(1.0, 0.96, d);

    // Rotating dashes — 12 segments
    float angle  = atan(gl_FragCoord.y - gl_FragCoord.y, 1.0); // dummy, use coord
    // Use position-derived angle from v_dist + gl_FragCoord trick
    // Actually we need the angle in disc space — approximate using gl_FragCoord
    // We pass the angle via v_dist packing trick won't work — use time-based dash
    float dash   = step(0.5, fract(d * 12.0 + t * 0.4));
    ring *= (0.4 + 0.6 * dash);

    // ── Inner translucent fill ─────────────────────────────────────────────
    float inner  = smoothstep(0.5, 0.0, d) * 0.12;

    // ── Concentric pulsing rings ───────────────────────────────────────────
    float pulse1 = fract(d * 3.0 - t * 0.6);
    float pulse2 = fract(d * 3.0 - t * 0.6 + 0.33);
    float prings = (smoothstep(0.0, 0.1, pulse1) * smoothstep(0.2, 0.1, pulse1)
                  + smoothstep(0.0, 0.1, pulse2) * smoothstep(0.2, 0.1, pulse2)) * 0.18;
    prings *= smoothstep(1.0, 0.6, d); // fade at edge

    // ── Centre glow dot ────────────────────────────────────────────────────
    float centre = smoothstep(0.08, 0.0, d) * 0.7;

    float alpha  = (ring + inner + prings + centre);
    if (alpha < 0.01) discard;

    // Colour: bright cyan-blue tint
    vec3 col = mix(vec3(0.2, 0.5, 1.0), vec3(0.4, 0.9, 1.0), 1.0 - d);
    fragColor = vec4(col, alpha * 0.75);
}
