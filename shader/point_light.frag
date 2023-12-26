#version 330 core

#define IS_FRAGMENT_SHADER 1

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

// Input parameters.
ATTR_LOC(0) flat in int passInstanceId;
ATTR_LOC(1) flat in vec3 passViewPosCenter;
ATTR_LOC(2) flat in float passInvRadius;
ATTR_LOC(3) flat in vec3 passLightColor;

PASS_DATA_BEGIN
	USING(PASS, VIEWRAY)
	USING(PASS, FRUSTUM)
PASS_DATA_END

INPUTS_BEGIN
	USING(PASS, DATA)
	UNUSED_INPUT(1)
	UNUSED_INPUT(2)
	UNUSED_INPUT(3)
  USING(INST, TRANSFORM)
	USING(INST, POINTLIGHT)
INPUTS_END

// Output parameters.
WRITING(outLightAcc, vec3, 0);

// Textures
DESCRIPTOR(SAMPLER, DEFAULT, 0)
SAMPLING(gbAlbedo, SMP_RT, 2D, 0)
SAMPLING(gbNormal, SMP_RT, 2D, 1)
SAMPLING(gbMisc, SMP_RT, 2D, 2)
SAMPLING(gbDepth, SMP_RT, 2D, 3)

void main () {
	Frustum frustum = READ(PASS, FRUSTUM);
	ViewRay viewRay = READ(PASS, VIEWRAY);
	// Compute sampling coord.
	vec2 passTexCoord = vec2(gl_FragCoord.x * frustum.invWidth, gl_FragCoord.y * frustum.invHeight);
	// Fetch shininess value.
	float shininess = texture(gbMisc, passTexCoord).x;
	// Fetch albedo texel.
	vec4 txAlbedo = texture(gbAlbedo, passTexCoord).xyzw;
	// Fetch specular intensity.
	float specIntensity = txAlbedo.w;
	// Fetch g buffer normal and decode it.
	vec3 normal = decodeNormal(texture(gbNormal, passTexCoord).xy);
  // Fetch depth 
	float depth = texture(gbDepth, passTexCoord).x;
	// Compute view space position.
	vec3 viewPos = computeViewPos(frustum, viewRay, passTexCoord, depth);
	// Light direction.
	vec3 lightDir = normalize(passViewPosCenter - viewPos);

	// Attenuation factor.
	float attenuation = quadraticAttenuation(passViewPosCenter, viewPos, passInvRadius);
	// Diffuse light term.
	vec3 diffuse = computeDiffuse(normal, lightDir) * attenuation * passLightColor;
	// Specular term.
	vec3 specular = computeSpecular(viewPos, lightDir, normal, specIntensity, shininess) * attenuation * passLightColor;

	// Output to light accumulation buffer.
	outLightAcc = txAlbedo.xyz * diffuse + specular;
}
