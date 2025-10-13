#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

PASS_DATA_BEGIN
	USING(PASS, VIEW)
PASS_DATA_END

INPUTS_BEGIN
  USING(PASS, DATA)
	UNUSED_INPUT(1) // vertices
	UNUSED_INPUT(2) // normals
	UNUSED_INPUT(3) // tex coords
  USING(INST, MATERIAL)
  // Always last
  USING(INST, INSTANCE_ID)
INPUTS_END

// Output parameters.
ATTR_LOC(0) out vec3 passEyeDir;
ATTR_LOC(1) flat out int passInstanceId;

void main() {
  // Instance index. Mandatory first line of main.
  passInstanceId = READ(INST, INSTANCE_ID);
  View vw = READ(PASS, VIEW);
  vec2 passTexCoord = texCoordFromVID(gl_VertexIndex);

  mat4 invProj = inverse(vw.proj);
  mat3 invView = transpose(mat3(vw.view));

  vec4 pos = vec4((passTexCoord * 2.0 - 1.0), 0.0, 1.0);
  vec3 unproject = (invProj * pos).xyz;

  passEyeDir = invView * unproject;
  gl_Position = pos;
}
