#version 330 core

#define IS_FRAGMENT_SHADER 1

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

// Input parameters.
ATTR_LOC(0) in vec2 passTexCoord;
ATTR_LOC(1) flat in int passInstanceId;

PASS_DATA_BEGIN
	USING(PASS, VIEWRAY)
	USING(PASS, FRUSTUM)
PASS_DATA_END

INPUTS_BEGIN
	USING(PASS, DATA)
	UNUSED_INPUT(1)
	UNUSED_INPUT(2)
	UNUSED_INPUT(3)
	USING(INST, DIRLIGHT)
INPUTS_END

// Output parameters.
WRITING(outLightAcc, vec3, 0);

// Textures
DESCRIPTOR(SAMPLER, DEFAULT, 0)
SAMPLING(gbAlbedo, SMP_RT, 2D, 0)
SAMPLING(gbNormal, SMP_RT, 2D, 1)
SAMPLING(gbMisc, SMP_RT, 2D, 2)
SAMPLING(gbDepth, SMP_RT, 2D, 3)

void main() {
	Frustum frustum = READ(PASS, FRUSTUM);
	ViewRay viewRay = READ(PASS, VIEWRAY);
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
	// View space light direction.
	DirLight dirLight = READ(INST, DIRLIGHT);
	vec3 lightDir = normalize(dirLight.viewDir.xyz);
	// Light color
	vec3 lightColor = dirLight.color.xyz;

	// Cos angle incidence of light.
	float cosAngle = dot( normal, lightDir );
	// Influence factor to lerp for hemispheric ambient.
	float influence = dot( normal, vec3(0, 1.0, 0) ) * 0.5 + 0.5;
	// Diffuse light term.
	vec3 diffuse = max(0.0, cosAngle ) * lightColor;
	// Specular term.
	vec3 specular = computeSpecular(viewPos, lightDir, normal, specIntensity, shininess) * lightColor;
	// Hemisperic ambient term.
	vec3 ambient = mix( dirLight.groundColor.xyz, dirLight.skyColor.xyz, influence ) * lightColor;

	outLightAcc = txAlbedo.xyz * diffuse + ambient + specular;
}