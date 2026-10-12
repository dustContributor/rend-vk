#version 330 core

#define IS_FRAGMENT_SHADER 1

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"
#include "shared_pbr.glsl.frag"

// Input parameters.
ATTR_LOC(0) in vec2 passTexCoord;
ATTR_LOC(1) flat in int passInstanceId;

PASS_DATA_BEGIN
	USING(PASS, VIEW)
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

DESCRIPTOR(SAMPLER, DEFAULT, 0)
// Textures
SAMPLING(gbAlbedo, SMP_RT, 2D, 0)
SAMPLING(gbNormal, SMP_RT, 2D, 1)
SAMPLING(gbMisc, SMP_RT, 2D, 2)
SAMPLING(gbDepth, SMP_RT, 2D, 3)
SAMPLING(gbCascade0, SMP_RT, 2DShadow, 4)
SAMPLING(gbCascade1, SMP_RT, 2DShadow, 5)
SAMPLING(gbCascade2, SMP_RT, 2DShadow, 6)
SAMPLING(gbCascade3, SMP_RT, 2DShadow, 7)
SAMPLING(gbOcclusion, SMP_RT, 2D, 8)

const float MIN_SHADOW_DIFFUSE = 0.15;

float mapLightSpaceDepth(float v, float depthBias) {
	#if IS_VULKAN
	return clamp(v + depthBias, 0.0, 1.0);
	#else 
	return clamp(v * 0.5 + 0.5 + depthBias, 0.0, 1.0);
	#endif
}

uint selectCascade(float viewDepth, vec4 splits) {
	uint cascadei = 0;
	for (uint i = 0u; i < DIR_LIGHT_CASCADES; ++i) {
		cascadei += uint(viewDepth >= splits[i]);
	}
	// clamp in case the view depth goes beyond the last split
	return min(cascadei, DIR_LIGHT_CASCADES - 1u);
}

float sampleCascade(uint cascadei, vec3 coords) {
	if (cascadei == 3u) {
	  return texture(gbCascade3, coords);
	} 
	if (cascadei == 2u) {
		return texture(gbCascade2, coords);
	}
	if (cascadei == 1u) {
		return texture(gbCascade1, coords);
	}
	return texture(gbCascade0, coords);
}

void main() {
	Frustum frustum = READ(PASS, FRUSTUM);
	ViewRay viewRay = READ(PASS, VIEWRAY);
	View view = READ(PASS, VIEW);
	// Light attributes
	vec3 lightDir = READ(INST, DIRLIGHT).viewDir.xyz;
	vec3 lightColor = READ(INST, DIRLIGHT).color.xyz;
	vec4 cascadeSplits = READ(INST, DIRLIGHT).cascadeSplits;
	vec3 groundColor = READ(INST, DIRLIGHT).groundColor.xyz;
	vec3 skyColor = READ(INST, DIRLIGHT).skyColor.xyz;
	// Fetch albedo texel.
	vec4 txAlbedo = texture(gbAlbedo, passTexCoord); 
	// Fetch g buffer normal and decode it.
	vec3 normal = decodeNormal(texture(gbNormal, passTexCoord).xy);
	// Fetch depth 
	float depth = texture(gbDepth, passTexCoord).x;
	// Compute view space position (true right-handed: z < 0 in front of the camera).
	vec3 viewPos = computeViewPos(frustum, viewRay, passTexCoord, depth);
	// Positive view space distance of this fragment.
	float viewDepth = toViewDepth(frustum, depth);

	uint lastCascade = DIR_LIGHT_CASCADES - 1u;
	float cascadeFar = cascadeSplits[lastCascade];
	float fadeStart = mix(cascadeSplits[lastCascade - 1u], cascadeFar, 0.75);
	float shadowFade = clamp((cascadeFar - viewDepth) / max(cascadeFar - fadeStart, 1e-5), 0.0, 1.0);

	uint cascadei = selectCascade(viewDepth, cascadeSplits);

	vec4 worldPos = view.invView * vec4(viewPos, 1.0);
	// Read directly from macro otherwise it forces scratch usage on AMD
	vec4 cascadePos = READ(INST, DIRLIGHT).cascadeViewProjs[cascadei] * worldPos;
	float cascadeBias = READ(INST, DIRLIGHT).cascadeBiases[cascadei];
	vec3 lightSpacePos = cascadePos.xyz / cascadePos.w;
	float lightSpaceDepth = mapLightSpaceDepth(lightSpacePos.z, cascadeBias);
	vec3 shadowmapCoords = vec3(lightSpacePos.xy * 0.5 + 0.5, lightSpaceDepth);

	// 1.0 means fully lit
	float inShadow = mix(1.0, sampleCascade(cascadei, shadowmapCoords), shadowFade);

  // Dir to eye.
  vec3 nmEyeDir = normalize( -viewPos ); 

	vec2 metalMap = texture(gbMisc, passTexCoord).xy; 

	outLightAcc = doLighting(
		lightColor,
		lightDir,
		inShadow,
		nmEyeDir,
		normal,
		metalMap.x,
		metalMap.y,
		txAlbedo.xyz
	);

	// Influence factor to lerp for hemispheric ambient, sky is always "up"
	float influence = clamp(dot(normal, vec3(0,1,0)) * 0.5 + 0.5, 0.0, 1.0);
		// Hemisperic ambient term.
	vec3 ambient = mix(groundColor, skyColor, influence);
	float occlusion = texture(gbOcclusion, passTexCoord).x;

	outLightAcc += (ambient * occlusion);
}
