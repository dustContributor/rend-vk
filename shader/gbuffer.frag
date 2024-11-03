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

INPUTS_BEGIN
    UNUSED_INPUT(0)
    UNUSED_INPUT(1)
    UNUSED_INPUT(2)
    UNUSED_INPUT(3)
    UNUSED_INPUT(4)
    USING(INST, MATERIAL)
    UNUSED_INPUT(6)
    UNUSED_INPUT(7)
INPUTS_END

// Output parameters.
WRITING(outAlbedo, vec4, 0);
WRITING(outNormal, vec2, 1);
WRITING(outMisc, vec3, 2);
WRITING(outVelocity, vec2, 3);

// Textures
DESCRIPTOR(SAMPLER, DEFAULT, 0)
DESCRIPTOR(TEXTURE, DEFAULT, 1)
SAMPLING(matDiffuse, SMP_TEX, 2D, 0)
SAMPLING(matNormal, SMP_TEX, 2D, 1)

void main() {
	Material mat = READ(INST, MATERIAL);
	// Compute flipped Y axis tex coord.
	vec2 texCoord = flipTexCoord(passTexCoord) * mat.scaling;
	// Fetch material shininess.
	float shininess = mat.shininess;
	// Fetch diffuse texel. 
	vec4 txDiffuse = texture(SAMPLER_FOR(matDiffuse, 2D, mat.diffuseId, mat.diffuseSamplerId), texCoord); 
	// Fetch normal texel. 
	vec4 txNormal = texture(SAMPLER_FOR(matNormal, 2D, mat.normalId, mat.normalSamplerId), texCoord); 
	// // Get specular map factor.
	float fspec = txNormal.w;
	// // Perturbed normal.
	vec3 pertNormal = perturbNormal(passNormal.xyz, txNormal.xyz, passViewPos, texCoord);
	// Output to gbuffer.
	outAlbedo = vec4(pow(txDiffuse.xyz, vec3(2.2)), fspec);
	outNormal = encodeNormal(pertNormal);
	// // Map shininess and store it.
	outMisc.x = storeShininess(shininess);
	// // Velocity buffer for motion blur
	outVelocity = ((passPrevProjPos.xy / passPrevProjPos.z) - (passProjPos.xy / passProjPos.z)).xy;
}