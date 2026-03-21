#version 300 es
precision mediump float;

in  vec2 v_uv;
out vec4 fragColor;

uniform vec3  uWarmthColor;
uniform float uWarmth;      // 0=cold, 1=hot
uniform float uHintTier;    // 0/1/2/3
uniform float uTime;

// ── Utilities ─────────────────────────────────────────────────────────────────
float sdCircle(vec2 p, float r) { return length(p) - r; }
float sdRing(vec2 p, float r, float t) { return abs(length(p) - r) - t; }
float sdBox(vec2 p, vec2 b) { vec2 d = abs(p) - b; return length(max(d,0.0)) + min(max(d.x,d.y),0.0); }

float aa(float d) { return smoothstep(0.005, -0.005, d); }

// Arrow shape pointing upward (tip at (0,0), pointing toward negative y)
float arrow(vec2 p, float sz) {
    p /= sz;
    // Triangle head
    float head = max(abs(p.x) - (0.5 - p.y * 0.8), p.y - 0.5);
    head = max(head, -p.y - 0.1);
    // Shaft
    float shaft = sdBox(p - vec2(0.0, 0.6), vec2(0.12, 0.4));
    return min(head, shaft);
}

// ── Main ──────────────────────────────────────────────────────────────────────
void main() {
    if (uHintTier < 0.5) { discard; }

    vec2 uv = v_uv * 2.0 - 1.0;   // [-1,1]
    vec2 aspect_uv = uv;

    // ── 1. Warm/cold vignette (full-screen, very subtle) ──────────────────
    float dist_from_center = length(uv);
    float vignette_strength = 0.12 * uWarmth;

    vec3 warm_tint = vec3(0.8, 0.15, 0.1);
    vec3 cold_tint = vec3(0.1, 0.2, 0.8);
    vec3 tint = mix(cold_tint, warm_tint, uWarmth);

    // Edge vignette: glow at screen edge, colour depends on warmth
    float edge = smoothstep(0.6, 1.3, dist_from_center);
    vec4 vignette = vec4(tint, edge * vignette_strength);

    // ── 2. Compass widget (bottom-left corner) ────────────────────────────
    // Widget sits at NDC (-0.72, 0.72) with radius ~0.16
    vec2 widget_center = vec2(-0.72, 0.72);
    vec2 wp = uv - widget_center;
    float widget_r = 0.15;

    // Outer ring
    float ring_d   = sdRing(wp, widget_r, 0.008);
    float ring_mask = aa(ring_d);

    // Inner warmth fill (arc from bottom, filling proportion = warmth)
    float fill_r   = widget_r * 0.72;
    float fill_d   = sdCircle(wp, fill_r * uWarmth);
    float fill_mask = aa(fill_d) * aa(-sdCircle(wp, fill_r));
    // Colour the fill from cold blue to hot red
    vec3 fill_col = mix(vec3(0.12, 0.25, 0.9), vec3(0.9, 0.15, 0.1), uWarmth);

    // Pulsing outer glow when very warm
    float pulse = 0.0;
    if (uWarmth > 0.8) {
        float p = 0.5 + 0.5 * sin(uTime * 5.5);
        float glow_d = sdCircle(wp, widget_r + 0.02 + p * 0.03);
        pulse = aa(glow_d) * (1.0 - aa(glow_d - 0.025)) * p * 0.6;
    }

    // Tick marks around the ring
    float ticks = 0.0;
    for (int i = 0; i < 8; i++) {
        float a    = float(i) * 3.14159 * 0.25;
        vec2 tick_dir = vec2(cos(a), sin(a));
        float tick_d = abs(dot(wp - tick_dir * widget_r, vec2(-tick_dir.y, tick_dir.x)));
        float tick_len_d = abs(dot(wp - tick_dir * widget_r, tick_dir));
        ticks += aa(max(tick_d - 0.004, tick_len_d - 0.014));
    }

    // Direction arrow — rotates to point toward warmth hemisphere
    // We encode warmth as a simple "up = toward solution" convention
    float arrow_sz   = widget_r * 0.5;
    // Lean the arrow: when warm, point up; when cold, wobble
    float wobble     = sin(uTime * 1.8) * (1.0 - uWarmth) * 0.4;
    float arrow_angle = wobble;
    vec2 rp = vec2(wp.x * cos(arrow_angle) - wp.y * sin(arrow_angle),
                   wp.x * sin(arrow_angle) + wp.y * cos(arrow_angle));
    float arrow_d    = arrow(rp, arrow_sz);
    float arrow_mask = aa(arrow_d);

    // Assemble widget
    vec4 ring_col  = vec4(mix(cold_tint, warm_tint, uWarmth), ring_mask * 0.85);
    vec4 fill_rgba = vec4(fill_col, fill_mask * 0.55);
    vec4 tick_rgba = vec4(vec3(0.7), ticks * 0.35);
    vec4 pulse_col = vec4(tint, pulse);
    vec4 arrow_col = vec4(1.0, 1.0, 1.0, arrow_mask * 0.90);

    // Clip to widget area
    float widget_clip = 1.0 - aa(sdCircle(wp, widget_r + 0.04));

    vec4 widget = ring_col;
    widget = mix(widget, fill_rgba, fill_rgba.a);
    widget = mix(widget, tick_rgba, tick_rgba.a);
    widget = mix(widget, pulse_col, pulse_col.a);
    widget = mix(widget, arrow_col, arrow_col.a);
    widget.a *= widget_clip;

    // ── 3. Warmth text label below compass ───────────────────────────────
    // (rendered as a subtle colour bar, no font needed)
    vec2 bar_center = widget_center + vec2(0.0, widget_r + 0.04);
    vec2 bp = uv - bar_center;
    float bar_d = sdBox(bp, vec2(widget_r, 0.008));
    float bar_mask = aa(bar_d);
    // Bar fill: left=cold, right=warm, cursor at warmth position
    float bar_t = (uv.x - (widget_center.x - widget_r)) / (2.0 * widget_r);
    bar_t = clamp(bar_t, 0.0, 1.0);
    vec3 bar_col = mix(cold_tint, warm_tint, bar_t);
    // Cursor tick
    float cursor_x = widget_center.x - widget_r + uWarmth * 2.0 * widget_r;
    float cursor_d = sdBox(uv - vec2(cursor_x, bar_center.y), vec2(0.005, 0.016));
    float cursor_mask = aa(cursor_d);

    vec4 bar_rgba = vec4(bar_col, bar_mask * 0.6);
    vec4 cursor_rgba = vec4(1.0, 1.0, 1.0, cursor_mask * 0.9);

    // ── Final composite ───────────────────────────────────────────────────
    vec4 result = vignette;
    result = mix(result, widget, widget.a);
    result = mix(result, bar_rgba, bar_rgba.a);
    result = mix(result, cursor_rgba, cursor_rgba.a);

    if (result.a < 0.01) discard;
    fragColor = result;
}
