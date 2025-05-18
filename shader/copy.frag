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

vec3 crosshair(vec2 texCoord, vec3 outColor) {
   const vec3 color = vec3(1);
   const float loLim = 0.019;
   const float hiLim = 0.021;
   float dist = length(texCoord - vec2(0.5));
   if(dist < 0.001) {
      return color;
   }
   if (dist < hiLim && dist > loLim) {
      float ndist = (dist - loLim) / (hiLim - loLim);
      ndist = abs((ndist - 0.5) * 2.0);
      return mix(color, outColor, max(0.75, ndist));
   }
   return outColor;
}

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
   outColor = crosshair(texCoord, outColor);
   outColor = pow(outColor, vec3(1.0/2.2));
   outColor = colorIfNaN(outColor);
   outColor = colorIfInf(outColor);
   outFrag = vec4(outColor, 1);
}
