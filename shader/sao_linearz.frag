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
WRITING(outLinearZ, float, 0);
// Textures
SAMPLING(gbDepth, SMP_RT, 2D, 0)

void main() {
  Frustum frustum = READ(PASS, FRUSTUM);
  float depth = texelFetch(gbDepth, ivec2(gl_FragCoord.xy), 0).r;
  float near = frustum.nearPlane;
  float far = frustum.farPlane;
  outLinearZ = (near * far) / (far + depth * (near - far));
}
