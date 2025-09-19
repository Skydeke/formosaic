#version 300 es
precision mediump float;

in vec2 v_uv;
flat in vec3 v_normal;
in vec3 v_pos;

out vec4 FragOutput;

uniform vec3 albedoConst;
uniform sampler2D albedoTex;
uniform int isAlbedoMapped;

void main() {
    // --- Albedo ---
    vec2 uv = vec2(v_uv.x, 1.0 - v_uv.y);
    vec4 albedo = (isAlbedoMapped == 1) ? texture(albedoTex, uv) : vec4(albedoConst, 1.0);
    if (albedo.a < 0.99) discard;

    // --- Lighting setup ---
    vec3 N = normalize(v_normal);
    vec3 V = normalize(-v_pos);           // Camera at origin
    vec3 sunDir = normalize(vec3(0.4, 0.8, 0.4)); // sun coming from above-left
    vec3 sunColor = vec3(1.2, 1.1, 0.9);           // Warm sunlight
    vec3 skyColor = vec3(0.5, 0.7, 1.0);           // Ambient sky

    // --- Ambient ---
    float upFacing = max(dot(N, vec3(0.0, 1.0, 0.0)), 0.0);
    vec3 ambient = mix(vec3(0.15), skyColor * 0.4, upFacing * 0.5);

    // --- Diffuse with wrap-around for softer shadows ---
    float NdotL = max(dot(N, sunDir), 0.0);
    float wrap = 0.3;
    float diffuse = max((NdotL + wrap) / (1.0 + wrap), 0.0);

    // --- Rim lighting for shape definition ---
    float rim = 1.0 - max(dot(N, V), 0.0);
    rim = pow(rim, 3.0) * 0.3;

    // --- Combine lighting (no specular) ---
    vec3 lighting = ambient;
    lighting += sunColor * diffuse;
    lighting += skyColor * rim;

    // --- Low-poly style contrast ---
    lighting = pow(lighting, vec3(0.9));

    // --- Optional height variation ---
    float heightVariation = 1.0 + sin(v_pos.y * 0.1) * 0.05;
    vec3 finalColor = albedo.rgb * lighting * heightVariation;

    FragOutput = vec4(finalColor, albedo.a);
}

