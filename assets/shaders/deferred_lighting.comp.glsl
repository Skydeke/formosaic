#version 310 es
precision mediump float;
precision mediump int;

layout(local_size_x = 32, local_size_y = 32) in;

// G-Buffer inputs
layout(rgba16f, binding = 0) uniform highp readonly image2D gAlbedo;
layout(rgba32f, binding = 1) uniform highp readonly image2D gNormalMetalness;
layout(rgba32f, binding = 2) uniform highp readonly image2D gPositionRoughness;

// Output image (lit scene)
layout(rgba16f, binding = 3) uniform highp writeonly image2D gOutput;

// Add this uniform
uniform vec3 cameraPos;

void main() {
    ivec2 coord = ivec2(gl_GlobalInvocationID.xy);

    // Load G-buffer data
    vec4 albedoData   = imageLoad(gAlbedo, coord);
    vec4 posRoughData = imageLoad(gPositionRoughness, coord);
    vec4 normMetal    = imageLoad(gNormalMetalness, coord);

    vec3 albedoColor = albedoData.rgb;
    float alpha      = albedoData.a;

    // Discard transparent
    if(alpha < 0.99) {
        imageStore(gOutput, coord, vec4(0.0,0.0,0.0,1.0));
        return;
    }

    // Geometry info
    vec3 fragPos = posRoughData.xyz;
    vec3 v_normal  = normalize(normMetal.xyz);
    vec3 v_pos = posRoughData.xyz;

    // --- Lighting setup ---
    vec3 N = normalize(v_normal);
    vec3 V = normalize(cameraPos - v_pos);  // Correct view direction
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
    vec3 finalColor = albedoColor * lighting * heightVariation;

    imageStore(gOutput, coord, vec4(finalColor, alpha));
}

