#version 300 es
precision mediump float;

in vec3 v_normal_ws;  // world-space normal (from vert)
in vec3 v_pos_ws;
in float v_outline_intensity;

out vec4 fragColor;

uniform vec3  uGlowColor;
uniform float uAlpha;
uniform float uTime;

void main() {
    // Fresnel rim glow — brightest where normal is perpendicular to view
    // (approximated in world-space since we don't have view matrix here)
    float rim     = clamp(1.0 - abs(v_normal_ws.z), 0.0, 1.0);
    rim           = pow(rim, 1.6);

    float throb   = 0.5 + 0.5 * sin(uTime * 2.4);
    float sparkle = 0.5 + 0.5 * sin(uTime * 11.0 + v_pos_ws.x * 15.0 + v_pos_ws.y * 10.0);
    float pulse   = v_outline_intensity * throb * (0.8 + 0.2 * sparkle);

    vec3 col = uGlowColor + rim * vec3(0.4, 0.6, 0.4);
    col      = mix(col, vec3(1.0), rim * 0.35);

    float a  = uAlpha * pulse * (0.5 + rim * 0.5);
    if (a < 0.01) discard;
    fragColor = vec4(col, a);
}
