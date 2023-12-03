#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

INPUTS_BEGIN
	UNUSED_INPUT(0) // pass data
	UNUSED_INPUT(1) // vertices
	UNUSED_INPUT(2) // normals
	UNUSED_INPUT(3) // tex coords
	UNUSED_INPUT(4) // dir lights
  // Always last
  USING(INST, INSTANCE_ID)
INPUTS_END

// Output parameters.
ATTR_LOC(0) out vec2 passTexCoord;
ATTR_LOC(1) flat out int passInstanceId;

void main() {
  // Instance index. Mandatory first line of main.
  passInstanceId = READ(INST, INSTANCE_ID);
  passTexCoord = texCoordFromVID(gl_VertexIndex);
  gl_Position = vec4((passTexCoord * 2.0 - 1.0), 0.0, 1.0);
}