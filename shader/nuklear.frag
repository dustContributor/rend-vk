#version 330 core

#define IS_FRAGMENT_SHADER 1

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

// Input parameters.
ATTR_LOC(0) in vec2 passTexCoord;
// Nuklear uses RGBA
ATTR_LOC(1) in vec4 passColor;
ATTR_LOC(2) flat in int passInstanceId;

INPUTS_BEGIN
    UNUSED_INPUT(0) // pass data
    UNUSED_INPUT(1) // position
    UNUSED_INPUT(2) // color
    UNUSED_INPUT(3) // texcoord
    USING(INST, MATERIAL)
    UNUSED_INPUT(4) // instance id
INPUTS_END

// Output parameters.
WRITING(outFrag, vec4, 0);

// Textures
DESCRIPTOR(SAMPLER, DEFAULT, 0)
DESCRIPTOR(TEXTURE, DEFAULT, 1)
SAMPLING(matDiffuse, SMP_TEX, 2D, 0)

void main() {
	Material mat = READ(INST, MATERIAL);
	// Compute flipped Y axis tex coord.
	vec2 texCoord = passTexCoord; //flipTexCoord(passTexCoord);
  // Fetch diffuse texel.
	vec4 txDiffuse = texture(SAMPLER_FOR(matDiffuse, 2D, mat.diffuseId, mat.diffuseSamplerId), texCoord);
  outFrag = vec4(txDiffuse.rrr, 1.0) * passColor;
}
