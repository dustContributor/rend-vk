#version 330 core

#define IS_FRAGMENT_SHADER 1

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

// Input parameters.
ATTR_LOC(0) in vec2 passTexCoord;
ATTR_LOC(1) in vec3 passNormal;
ATTR_LOC(2) in vec3 passViewPos;
ATTR_LOC(3) in vec3 passProjPos;
ATTR_LOC(4) in vec3 passPrevProjPos;
ATTR_LOC(5) flat in int passInstanceId;

PASS_DATA_BEGIN
	USING(PASS, VIEW)
	USING(PASS, TIMING)
PASS_DATA_END

INPUTS_BEGIN
	USING(PASS, DATA)
  UNUSED_INPUT(1)
  UNUSED_INPUT(2)
  UNUSED_INPUT(3)
  UNUSED_INPUT(4)
  USING(INST, MATERIAL)
  UNUSED_INPUT(7)
INPUTS_END

// Output parameters.
WRITING(outAlbedo, vec4, 0);
WRITING(outNormal, vec2, 1);
WRITING(outMisc, vec2, 2);
WRITING(outVelocity, vec2, 3);

// Textures
DESCRIPTOR(SAMPLER, DEFAULT, 0)
DESCRIPTOR(TEXTURE, DEFAULT, 1)
SAMPLING(matDiffuse, SMP_TEX, 2D, 0)
SAMPLING(matNormal, SMP_TEX, 2D, 1)
SAMPLING(matMetallic, SMP_TEX, 2D, 2)

void main() {
	Material mat = READ(INST, MATERIAL);
	// Compute flipped Y axis tex coord.
	vec2 texCoord = flipTexCoord(passTexCoord) * mat.scaling;
	vec4 txDiffuse = texture(SAMPLER_FOR(matDiffuse, 2D, mat.img0, mat.smp0), texCoord); 
	vec4 txNormal = texture(SAMPLER_FOR(matNormal, 2D, mat.img1, mat.smp1), texCoord); 
	vec4 txMetallic = texture(SAMPLER_FOR(matMetallic, 2D, mat.img2, mat.smp2), texCoord); 
	vec3 pertNormal = perturbNormal(passNormal.xyz, txNormal.xyz, passViewPos, texCoord);
	// write gbuffer
	outAlbedo = vec4(pow(txDiffuse.xyz, vec3(2.2)), 0.0);
	outNormal = encodeNormal(pertNormal);
	// r = metalness, g = roughness
	outMisc = txMetallic.xy;
	// // Velocity buffer for motion blur
	outVelocity = ((passPrevProjPos.xy / passPrevProjPos.z) - (passProjPos.xy / passProjPos.z)).xy;
}