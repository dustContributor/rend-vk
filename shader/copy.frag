#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

DESCRIPTOR(SAMPLER, DEFAULT, 0)
SAMPLING(gbLightAcc, SMP_RT, 2D, 0)
SAMPLING(gbNormal, SMP_RT, 2D, 1)
SAMPLING(gbAlbedo, SMP_RT, 2D, 2)
SAMPLING(gbMisc, SMP_RT, 2D, 3)
// SAMPLING(gbDepth, SMP_RT, 2D, 4)

// Input parameters.
ATTR_LOC(0) in vec2 passTexCoord;

PASS_DATA_BEGIN
    USING(PASS, VIEWRAY)
    USING(PASS, FRUSTUM)
PASS_DATA_END

INPUTS_BEGIN
	USING(PASS, DATA)
INPUTS_END

// Output parameters.
ATTR_LOC(0) out vec4 outFrag;

void main() {
   vec2 texCoord = apiTexCoord(passTexCoord);
	vec4 txAlbedo = texture(gbAlbedo, texCoord).xyzw;
	vec3 txLightAcc = texture(gbLightAcc, texCoord).xyz;
	vec3 txNormal = texture(gbNormal, texCoord).xyz;
	vec3 txMisc = texture(gbMisc, texCoord).xyz;

	Frustum frustum = READ(PASS, FRUSTUM);
	ViewRay viewRay = READ(PASS, VIEWRAY);

   vec3 outColor = txLightAcc.xyz;
   float luminosity = luminosity(outColor);
   outColor *= (luminosity / (luminosity + 1.0));
   outColor = pow(outColor, vec3(1.0/2.2));
   outColor = colorIfNaN(outColor);
   outColor = colorIfInf(outColor);
   outFrag = vec4(outColor, 1);
}
