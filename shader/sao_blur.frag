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
  vec4 blurAxis;
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
WRITING(outOcclusion, float, 0);
// Textures
SAMPLING(gbPackPos, SMP_RT, 2D, 0)
SAMPLING(gbOcclusion, SMP_RT, 2D, 1)

// Tunable Parameters:

/** Increase to make depth edges crisper. Decrease to reduce flicker. */
#define EDGE_SHARPNESS     (1.0)

/** Step in 2-pixel intervals since we already blurred against neighbors in the
    first AO pass.  This constant can be increased while R decreases to improve
    performance at the expense of some dithering artifacts. 
    
    Morgan found that a scale of 3 left a 1-pixel checkerboard grid that was
    unobjectionable after shading was applied but eliminated most temporal incoherence
    from using small numbers of sample taps.
    */
#define SCALE               (2)

/** Filter radius in pixels. This will be multiplied by SCALE. */
#define R                   (4)

const float[] gaussian = {0.153170, 0.144893, 0.122649, 0.092902, 0.062970};

/** Returns a number on (0, 1) */
float unpackKey(vec2 p) {
    return p.x * (256.0 / 257.0) + p.y * (1.0 / 257.0);
}

void main() {
  ivec2 ssC = ivec2(gl_FragCoord.xy);
  vec2 rawKey = texelFetch(gbPackPos, ssC, 0).xy;
  float key = unpackKey(rawKey);

  ivec2 axis = ivec2(READ_CONST(blurAxis).xy);

  float rawOcc = texelFetch(gbOcclusion, ssC, 0).x;
  float totalWeight = gaussian[0];
  float sum = rawOcc * totalWeight;

  for (int r = -R; r <= R; ++r) {
    if (r != 0) {
      ivec2 coords = ivec2(ssC + axis * (r * SCALE));
      rawKey = texelFetch(gbPackPos, coords, 0).xy;
      rawOcc = texelFetch(gbOcclusion, coords, 0).x;
      float tapKey = unpackKey(rawKey);
      float weight = 0.3 + gaussian[abs(r)];
      weight *= max(0.0, 1.0
          - (EDGE_SHARPNESS * 2000.0) * abs(tapKey - key)
      );
      sum += rawOcc * weight;
      totalWeight += weight;
    }
  }
  const float epsilon = 0.0001;
  outOcclusion = sum / (totalWeight + epsilon);	
}
