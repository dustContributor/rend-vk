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
WRITING(outOcclusion, float, 0);
// Textures
SAMPLING(gbDepth, SMP_RT, 2D, 0)
SAMPLING(gbLinearDepth, SMP_RT, 2D, 1)

// Total number of direct samples to take at each pixel
#define NUM_SAMPLES (11)

// This is the number of turns around the circle that the spiral pattern makes.  This should be prime to prevent
// taps from lining up.  This particular choice was tuned for NUM_SAMPLES == 9
#define NUM_SPIRAL_TURNS (7)

// If using depth mip levels, the log of the maximum pixel offset before we need to switch to a lower 
// miplevel to maintain reasonable spatial locality in the cache
// If this number is too small (< 3), too many taps will land in the same pixel, and we'll get bad variance that manifests as flashing.
// If it is too high (> 5), we'll get bad performance because we're not using the MIP levels effectively
#define LOG_MAX_OFFSET (3)

// This must be less than or equal to the MAX_MIP_LEVEL defined in SSAO.cpp
#define MAX_MIP_LEVEL (5)


/** World-space AO radius in scene units (r).  e.g., 1.0m */
const float AO_RADIUS = 5.0;
const float AO_RADIUS2 = AO_RADIUS * AO_RADIUS;

/** Bias to avoid AO in smooth corners, e.g., 0.01m */
const float BIAS = 0.01;

const float INTENSITY = 1.2; 
/** INTENSITY / AO_RADIUS^6 */
const float INTENSITY_DIV_R6 =  INTENSITY / pow(AO_RADIUS, 6.0); 


/** Reconstruct camera-space P.xyz from screen-space S = (x, y) in
    pixels and camera-space z < 0.  Assumes that the upper-left pixel center
    is at (0.5, 0.5) [but that need not be the location at which the sample tap 
    was placed!]

    Costs 3 MADD.  Error is on the order of 10^3 at the far plane, partly due to z precision.
  */
vec3 reconstructCSPosition(
    vec2 S, 
    float z, 
    mat4 proj, 
    float width, 
    float height) {
  mat4 P = proj;
  vec4 projInfo = vec4(-2.0f / (width*P[0][0]), 
          -2.0f / (height*P[1][1]),
          ( 1.0f - P[0][2]) / P[0][0], 
          ( 1.0f + P[1][2]) / P[1][1]);
  return vec3((S.xy * projInfo.xy + projInfo.zw) * z, z);
}

/** Read the camera-space position of the point at screen-space pixel ssP + unitOffset * ssR.  Assumes length(unitOffset) == 1 */
vec3 getOffsetPosition(
    ivec2 ssC, 
    vec2 unitOffset, 
    float ssR,
    mat4 proj, 
    float width, 
    float height) {
    // Derivation:
    //  mipLevel = floor(log(ssR / MAX_OFFSET));
    int mipLevel = clamp(int(floor(log2(ssR))) - LOG_MAX_OFFSET, 0, MAX_MIP_LEVEL);

    ivec2 ssP = ivec2(ssR * unitOffset) + ssC;
    
    vec3 P;

    // We need to divide by 2^mipLevel to read the appropriately scaled coordinate from a MIP-map.  
    // Manually clamp to the texture size because texelFetch bypasses the texture unit
    ivec2 mipP = clamp(ssP >> mipLevel, ivec2(0), textureSize(gbLinearDepth, mipLevel) - ivec2(1));
    P.z = texelFetch(gbLinearDepth, mipP, mipLevel).r;
    // Offset to pixel center
    P = reconstructCSPosition(vec2(ssP) + vec2(0.5), P.z, proj, width, height);

    return P;
}

/** Returns a unit vector and a screen-space radius for the tap on a unit disk (the caller should scale by the actual disk radius) */
vec2 tapLocation(int sampleNumber, float spinAngle, out float ssR){
    // Radius relative to ssR
    float alpha = float(sampleNumber + 0.5) * (1.0 / NUM_SAMPLES);
    float angle = alpha * (NUM_SPIRAL_TURNS * 6.28) + spinAngle;

    ssR = alpha;
    return vec2(cos(angle), sin(angle));
}

float sampleAO(
    in ivec2 ssC, 
    in vec3 C, 
    in vec3 n_C, 
    in float ssDiskRadius, 
    in int tapIndex, 
    in float randomPatternRotationAngle,
    mat4 proj,
    float width,
    float height) {
    // Offset on the unit disk, spun for this pixel
    float ssR;
    vec2 unitOffset = tapLocation(tapIndex, randomPatternRotationAngle, ssR);
    ssR *= ssDiskRadius;
        
    // The occluding point in camera space
    vec3 Q = getOffsetPosition(ssC, unitOffset, ssR, proj, width, height);

    vec3 v = Q - C;

    float vv = dot(v, v);
    float vn = dot(v, n_C);

    const float EPSILON = 0.01;
    
    // A: From the HPG12 paper
    // Note large EPSILON to avoid overdarkening within cracks
    // return float(vv < RADIUS2) * max((vn - BIAS) / (EPSILON + vv), 0.0) * RADIUS2 * 0.6;

    // B: Smoother transition to zero (lowers contrast, smoothing out corners). [Recommended]
    float f = max(AO_RADIUS2 - vv, 0.0); 
    return f * f * f * max((vn - BIAS) / (EPSILON + vv), 0.0);

    // C: Medium contrast (which looks better at high radii), no division.  Note that the 
    // contribution still falls off with radius^2, but we've adjusted the rate in a way that is
    // more computationally efficient and happens to be aesthetically pleasing.
    // return 4.0 * max(1.0 - vv * invradius2, 0.0) * max(vn - BIAS, 0.0);

    // D: Low contrast, no division operation
    // return 2.0 * float(vv < radius * radius) * max(vn - BIAS, 0.0);
}

/** Read the camera-space position of the point at screen-space pixel ssP */
vec3 getPosition(ivec2 ssP, float depth, mat4 proj, float width, float height) {
    // Offset to pixel center
    return reconstructCSPosition(vec2(ssP) + vec2(0.5), depth, proj, width, height);
}

/** Reconstructs screen-space unit normal from screen-space position */
vec3 reconstructCSFaceNormal(vec3 C) {
    return normalize(cross(dFdy(C), dFdx(C)));
}

/** Used for packing Z into the GB channels */
float CSZToKey(float z, float farPlane) {
    return clamp(z * (1.0 / farPlane), 0.0, 1.0);
}

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

void main() {
	View view = READ(PASS, VIEW);
	Frustum frustum = READ(PASS, FRUSTUM);
    /** The height in pixels of a 1m object if viewed from 1m away.  
    You can compute it from your projection matrix.  The actual value is just
    a scale factor on radius; you can simply hardcode this to a constant (~500)
    and make your radius value unitless (...but resolution dependent.)  */
    float projScale = frustum.fragmentsPerMeterPlane;
    

    // Pixel being shaded 
    ivec2 ssC = ivec2(gl_FragCoord.xy);
	// Fetch depth 
    float linearDepth = texelFetch(gbLinearDepth, ssC, 0).r;
    // World space point being shaded
    vec3 C = getPosition(ssC, linearDepth, view.proj, frustum.width, frustum.height);

    // Hash function used in the HPG12 AlchemyAO paper
    float randomPatternRotationAngle = (3 * ssC.x ^ ssC.y + ssC.x * ssC.y) * 10;

    // Reconstruct normals from positions. These will lead to 1-pixel black lines
    // at depth discontinuities, however the blur will wipe those out so they are not visible
    // in the final image.
    vec3 n_C = reconstructCSFaceNormal(C);
    
    // Choose the screen-space sample radius
    // proportional to the projected area of the sphere
    float ssDiskRadius = -projScale * AO_RADIUS / C.z;
    
    float sum = 0.0;
    for (int i = 0; i < NUM_SAMPLES; ++i) {
        sum += sampleAO(
            ssC, 
            C, 
            n_C, 
            ssDiskRadius, 
            i, 
            randomPatternRotationAngle,
            view.proj,
            frustum.width,
            frustum.height);
    }

    float A = max(0.0, 1.0 - sum * INTENSITY_DIV_R6 * (5.0 / NUM_SAMPLES));

    // Bilateral box-filter over a quad for free, respecting depth edges
    // (the difference that this makes is subtle)
    if (abs(dFdx(C.z)) < 0.02) {
        A -= dFdx(A) * ((ssC.x & 1) - 0.5);
    }
    if (abs(dFdy(C.z)) < 0.02) {
        A -= dFdy(A) * ((ssC.y & 1) - 0.5);
    }
    
    outOcclusion = A;
}
