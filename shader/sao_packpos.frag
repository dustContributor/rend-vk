#version 330 core

#define IS_FRAGMENT_SHADER 1

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

/*
* NOTICE: 
* 
* This implementation of screen-space ambient obscurance is based on the work of [McGuire et al., 2012], 
* adapting the originally released under the OSI-approved 'Modified BSD' license reference code from G3D.
* For a detailed explanation of the method, please refer to the original paper: 
* https://research.nvidia.com/sites/default/files/pubs/2012-06_Scalable-Ambient-Obscurance/McGuire12SAO.pdf
* 
* Thanks to the original authors for making this technique available!
*/

PASS_DATA_BEGIN
	USING(PASS, VIEW)
	USING(PASS, FRUSTUM)
PASS_DATA_END

INPUTS_BEGIN
  USING(PASS, DATA)
  UNUSED_INPUT(1)
  UNUSED_INPUT(2)
  UNUSED_INPUT(3)
  UNUSED_INPUT(4)
INPUTS_END

// Input parameters.
ATTR_LOC(0) in vec2 passTexCoord;
ATTR_LOC(1) flat in int passInstanceId;

// Output parameters.
WRITING(outPackPos, vec2, 0);
// Textures
SAMPLING(gbLinearDepth, SMP_RT, 2D, 0)

/** Used for packing Z into the GB channels */
vec2 packKey(float key) {
    // Round to the nearest 1/256.0
    float temp = floor(key * 256.0);

    // Integer part
    float x = temp * (1.0 / 256.0);

    // Fractional part
    float y = key * 256.0 - temp;
    return vec2(x,y);
}

/** Reconstruct camera-space P.xyz from screen-space S = (x, y) in
    pixels and camera-space z < 0.  Assumes that the upper-left pixel center
    is at (0.5, 0.5) [but that need not be the location at which the sample tap 
    was placed!]
  */
vec3 reconstructCSPosition(vec2 S, float z, mat4 proj, float width, float height) {
  //  where P is the projection matrix that maps camera space points  to [-1, 1] x [-1, 1]
  mat4 P = proj;
  vec4 projInfo = vec4(-2.0f / (width*P[0][0]), 
          -2.0f / (height*P[1][1]),
          ( 1.0f - P[0][2]) / P[0][0], 
          ( 1.0f + P[1][2]) / P[1][1]);
  return vec3((S.xy * projInfo.xy + projInfo.zw) * z, z);
}

/** Read the camera-space position of the point at screen-space pixel ssP */
vec3 getPosition(ivec2 ssP, float depth, mat4 proj, float width, float height) {
    // Offset to pixel center
    return reconstructCSPosition(vec2(ssP) + vec2(0.5), depth, proj, width, height);
}

/** Used for packing Z into the GB channels */
float CSZToKey(float z, float farPlane) {
    return clamp(z * (1.0 / farPlane), 0.0, 1.0);
}

void main() {
	Frustum frustum = READ(PASS, FRUSTUM);
	View view = READ(PASS, VIEW);
  // Pixel being shaded 
  ivec2 ssC = ivec2(gl_FragCoord.xy);
  float depth = texelFetch(gbLinearDepth, ssC, 0).r;
  // World space point being shaded
  vec3 C = getPosition(
    ssC,
    depth,
    view.proj, 
    frustum.width, 
    frustum.height);
  float key = CSZToKey(C.z, frustum.farPlane);
  outPackPos = vec2(packKey(key).xy);
}
