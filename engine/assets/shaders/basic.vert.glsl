#version 300 es
precision highp float;
precision highp int;

layout (location = 0) in vec3 pos;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec3 norm;
layout (location = 3) in ivec4 boneIndices;
layout (location = 4) in vec4 boneWeights;
layout (location = 5) in vec4 vertColor;

out vec3 v_pos;
out vec2 v_uv;
out vec3 v_normal;
out vec4 v_color;

uniform mat4 uVP;
uniform mat4 uModel;
uniform int uBoneCount;
uniform mat4 uBones[128];

void main() {
    vec4 skinnedPos = vec4(pos, 1.0);
    vec3 skinnedNorm = norm;

    if (uBoneCount > 0) {
        vec4 blendedPos = vec4(0.0, 0.0, 0.0, 0.0);
        vec3 blendedNorm = vec3(0.0, 0.0, 0.0);
        bool hasInfluence = false;
        for (int i = 0; i < 4; i++) {
            int boneIdx = boneIndices[i];
            float weight = boneWeights[i];
            if (boneIdx >= 0 && weight > 0.0) {
                mat4 bone = uBones[boneIdx];
                blendedPos += weight * (bone * vec4(pos, 1.0));
                blendedNorm += weight * mat3(bone) * norm;
                hasInfluence = true;
            }
        }

        if (hasInfluence) {
            skinnedPos = blendedPos;
            skinnedNorm = blendedNorm;
        }
    }

    vec4 worldPos = uModel * skinnedPos;
    gl_Position   = uVP   * worldPos;
    v_pos    = worldPos.xyz;
    v_uv     = uv;
    v_normal = transpose(inverse(mat3(uModel))) * skinnedNorm;
    v_color  = vertColor;
}
