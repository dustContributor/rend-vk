#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (set = 0, binding = 0) uniform sampler smpler;
layout (set = 1, binding = 0) uniform texture2D txture;

layout (location = 0) in vec2 passTexCoord;

layout (location = 0) out vec4 outColor;

vec3 colorIfNaN (vec3 val, vec3 color) {
  return any(isnan(val.xyz)) ? color : val;
}

void main() {
   vec2 tc = passTexCoord;
   vec3 color = texture(sampler2D(txture, smpler), vec2(tc.x, tc.y)).xyz;
   outColor = vec4(color, 1);
}
