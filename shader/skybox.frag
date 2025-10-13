#version 330 core

#define IS_FRAGMENT_SHADER 1

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

// Input parameters.
ATTR_LOC(0) in vec3 passEyeDir;
ATTR_LOC(1) flat in int passInstanceId;

PASS_DATA_BEGIN
	USING(PASS, VIEW)
PASS_DATA_END

INPUTS_BEGIN
  USING(PASS, DATA)
	UNUSED_INPUT(1) // vertices
	UNUSED_INPUT(2) // normals
	UNUSED_INPUT(3) // tex coords
  USING(INST, MATERIAL)
  UNUSED_INPUT(5) // instance id
INPUTS_END

// Output parameters.
WRITING(outLightAcc, vec3, 0);

// Textures
DESCRIPTOR(SAMPLER, DEFAULT, 0)
DESCRIPTOR(TEXTURE, CUBE, 1)
SAMPLING(matEnv, SMP_TEX, Cube, 0)


void main () {
	Material mat = READ(INST, MATERIAL);
  outLightAcc = texture(SAMPLER_FOR(matEnv, Cube, mat.diffuseId, mat.diffuseSamplerId), passEyeDir).xyz;
}
