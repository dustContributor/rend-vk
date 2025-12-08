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
  float previousLevel;
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
WRITING(outLinearZ, float, 0);
// Textures
SAMPLING(gbLinearDepth, SMP_RT, 2D, 0)

void main() {
  int previousLevel = int(READ_CONST(previousLevel));
  ivec2 ssP = ivec2(gl_FragCoord.xy);
  ivec2 depthSize = textureSize(gbLinearDepth, 0);
  ivec2 coords = clamp(ssP * 2 + ivec2(ssP.y & 1, ssP.x & 1), ivec2(0), depthSize - ivec2(1));
  float depth = texelFetch(gbLinearDepth, coords, 0).r;
  outLinearZ = depth;
}
