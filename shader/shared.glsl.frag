#ifndef SHARED_GLSL
#define SHARED_GLSL

// Numeric constants.
#define NUM_PI 3.1415927
#define NUM_TAU 6.2831855
#define NUM_INV_PI 0.31830987
#define NUM_INV_TAU 0.15915494
#define NUM_SQRT2 1.4142135
/* Various struct definitions. */

struct Frustum
{
  float width;
  float height;
  float invWidth;
  float invHeight;
  float nearPlane;
  float farPlane;
};

struct ViewRays
{
  vec3 bleft;
  float m22;
  vec3 bright;
  float m23;
  vec3 tright;
  float m32;
  vec3 tleft;
  float m33;
};

struct Transform
{
  mat4 mvp;
  mat4 mv;
};

struct TransformExtra
{
  mat4 prevMvp;
};

struct Material
{
  float shininess;
#ifdef IS_VULKAN
  int diffuseId;
  int normalMapId;
#endif
};

struct DirLight
{
  // In camera space.
  vec4 viewDir;
  vec4 color;
  // For hemispheric ambient.
  vec4 skyColor;
  vec4 groundColor;
  // For shadows.
  mat4 invViewShadowProj;
  float cameraHeight;
  float innerRadius;
  float outerRadius;
};

struct StaticShadow
{
  mat4 mvp;
};

struct PointLight
{
  vec3 color;
  float radius;
};

struct SpotLight
{
  // Cosine of cutoff angle in radians.
  float cosCutoffRad;
  // Sine of cutoff angle in radians.
  float sinCutoffRad;
  // Range of the spotlight.
  float range;
  // Inverse range for attenuation.
  float invRange;
  vec3 color;
};

struct Joint
{
  vec3 col0;
  float x;
  vec3 col1;
  float y;
  vec3 col2;
  float z;
};

struct Sky
{
  vec4 EMTPY;
};

/* Basic utility functions.  */

float rand(vec2 co)
{
    return fract(sin(dot(co.xy ,vec2(12.9898,78.233))) * 43758.5453);
}

float luminosity ( vec3 val )
{
  return dot(val, vec3(0.299, 0.587, 0.114));
}

float saturate ( float value )
{
  return clamp(value, 0.0, 1.0);
}

vec2 saturate ( vec2 value )
{
  return clamp(value, vec2(0.0), vec2(1.0));
}

vec3 saturate ( vec3 value )
{
  return clamp(value, vec3(0.0), vec3(1.0));
}

vec4 saturate ( vec4 value )
{
  return clamp(value, vec4(0.0), vec4(1.0));
}

/* Pow utility functions. */

float pow2 ( float val)
{
  return val * val;
}

float pow3 ( float val)
{
  return pow2(val) * val;
}

float pow4 ( float val )
{
  return pow2(val) * pow2(val);
}

/* vec2 overloads */

vec2 pow2 ( vec2 val)
{
  return val * val;
}

vec2 pow3 ( vec2 val)
{
  return pow2(val) * val;
}

vec2 pow4 ( vec2 val )
{
  return pow2(val) * pow2(val);
}

/* vec3 overloads */

vec3 pow2 ( vec3 val)
{
  return val * val;
}

vec3 pow3 ( vec3 val)
{
  return pow2(val) * val;
}

vec3 pow4 ( vec3 val )
{
  return pow2(val) * pow2(val);
}

/* vec4 overloads */

vec4 pow2 ( vec4 val)
{
  return val * val;
}

vec4 pow3 ( vec4 val)
{
  return pow2(val) * val;
}

vec4 pow4 ( vec4 val )
{
  return pow2(val) * pow2(val);
}

float smootherstep( float edge0, float edge1, float x )
{
  // Scale and saturate.
  x = saturate((x - edge0) / (edge1 - edge0));
  // Polynomial.
  return x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
}

// Bilinearly interpolates a point given 4 corner points.
vec3 blerp ( vec3 bleft, vec3 bright, vec3 tleft, vec3 tright, vec2 point )
{
  vec3 b = mix(bleft, bright, point.x); // Lerp horizontal.
  vec3 t = mix(tleft, tright, point.x); // Lerp horizontal.

  return mix(b, t, point.y); // Lerp vertical.
}

/* Normal encoding/decoding functions */

// Decode normal z component from encoded vec2 normal.
vec3 decodeNormal ( vec2 normal )
{
  vec2 fenc = normal * 4.0 - 2.0;
  float f = dot(fenc, fenc);
  /*
  * Had to saturate the input to the sqrt since
  * it was producing negative results for some
  * (not normalized?) normals.
  */
  float g = sqrt(saturate(1.0 - f * 0.25));
  return vec3(fenc * g, 1.0 - f * 0.5);
}

// Encode vec3 normal in vec2.
vec2 encodeNormal ( vec3 normal )
{
  float f = inversesqrt(8.0 * normal.z + 8.0);
  return normal.xy * f + 0.5;
}

/* Texcoord utility functions. */

// Flips Y tex coord.
vec2 flipTexCoord ( vec2 texCoord )
{
  return vec2(texCoord.x, 1.0 - texCoord.y);
}

/* Misc. */

// Returns color if any of the 'val' components are NaN.
vec3 colorIfNaN ( vec3 val, vec3 color )
{
  return any(isnan(val.xyz)) ? color : val;
}

// This version colors NaNs with pink by default.
vec3 colorIfNaN ( vec3 val )
{
  return colorIfNaN(val, vec3(1.0, 0.0, 1.0));
}

// Returns color if any of the 'val' components are infinity.
vec3 colorIfInf ( vec3 val, vec3 color )
{
  return any(isinf(val.xyz)) ? color : val;
}

// This version colors Infs with cyan by default.
vec3 colorIfInf ( vec3 val )
{
  return colorIfInf(val, vec3(0.0, 1.0, 1.0));
}

// Convertes NDC depth to view space depth.
float toViewDepth ( Frustum frust, float depth )
{
  float near = frust.nearPlane;
  float far = frust.farPlane;
  return near * far / ((depth * (far - near)) - far);
}

vec2 texCoordFromVID ( int vertexId )
{
  return vec2(float((vertexId << 1) & 2), float(vertexId & 2));
}

vec3 blerpViewRay ( ViewRays viewRays, vec2 texCoord )
{
  return blerp(viewRays.bleft, viewRays.bright,
    viewRays.tleft, viewRays.tright,
    texCoord);
}

vec3 computeViewPos ( Frustum frustum, vec3 viewRay, float depth )
{
  // Compute view space depth.
  float viewDepth = toViewDepth(frustum, depth);
  // Compute view space position and return it.
  return viewRay * viewDepth;
}

vec3 computeViewPos ( Frustum frustum, ViewRays viewRays, vec2 texCoord, float depth )
{
  // Compute view space position and return it.
  return computeViewPos(frustum, blerpViewRay(viewRays, texCoord), depth);
}

/* Color format functions. */

vec3 rgbToYCbCr ( vec3 rgb )
{
  float r = rgb.x;
  float g = rgb.y;
  float b = rgb.z;

  float y = 0.299 * r + 0.587 * g + 0.114 * b;
  float cb = 0.5 + ( -0.168 * r - 0.331 * g + 0.5 * b );
  float cr = 0.5 + ( 0.5 * r - 0.418 * g - 0.081 * b );

  return vec3(y, cb, cr);
}

vec3 yCbCrToRgb ( vec3 ycbcr )
{
  float y = ycbcr.x;
  float cb = ycbcr.y;
  float cr = ycbcr.z;

  float r = y + 1.402 * (cr - 0.5);
  float g = y - 0.344 * (cb - 0.5) - 0.714 * (cr - 0.5);
  float b = y + 1.772 * (cb - 0.5);

  return vec3(r,g,b);
}

vec2 packYCbCr ( vec3 ycbcr )
{
  int hi = int(clamp(ycbcr.y * 15.0, 0.0, 15.0));
  int lo = int(clamp(ycbcr.z * 15.0, 0.0, 15.0));

  int pack = ((hi << 4) | lo);

  return vec2(ycbcr.x, float(pack) * (1.0 / 255.0));
}

vec3 unpackYCbCr ( vec2 ycbcr )
{
  int hi = ( int(ycbcr.y * 255.0) >> 4) & 0xF;
  int lo = int(ycbcr.y * 255.0) & 0xF;

  float cb = float(hi) * (1.0 / 15.0);
  float cr = float(lo) * (1.0 / 15.0);

  return vec3( ycbcr.x, cb, cr);
}

vec3 rgbToHsv( vec3 rgb )
{
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = rgb.g < rgb.b ? vec4(rgb.bg, K.wz) : vec4(rgb.gb, K.xy);
    vec4 q = rgb.r < rgb.x ? vec4(p.xyw, rgb.r) : vec4(rgb.r, p.yzx);

    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsvToRgb( vec3 hsv )
{
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(hsv.xxx + K.xyz) * 6.0 - K.www);
    return hsv.z * mix(K.xxx, saturate(p - K.xxx), hsv.y);
}

/* Data packing/unpacking functions. */

// Unpacks four unsinged byte values from an uint into a
// uvec4 with separate values.
uvec4 unpackByte1x4 ( uint v )
{
  uint x = v & 255u;
  uint y = (v >> 8u) & 255u;
  uint z = (v >> 16u) & 255u;
  uint w = (v >> 24u) & 255u;
  return uvec4(x,y,z,w);
}

/* Normal mapping functions. */

/*
* Apparently AMD driver doesn't likes if dFd* calls exist in anything but fragment
* shaders, regardless of the functions being actually used or not. So can't append
* these functions to any other kind of shader.
*/
#ifdef FRAGMENT_SHADER

  mat3 cotangentFrame ( vec3 normal, vec3 viewPos, vec2 texCoord )
  {
    // Get edge vectors of the pixel triangle
    vec3 dp1 = vec3(dFdx(viewPos));
    vec3 dp2 = vec3(dFdy(viewPos));
    vec2 duv1 = vec2(dFdx(texCoord));
    vec2 duv2 = vec2(dFdy(texCoord));

    // Solve the linear system
    vec3 dp2perp = cross(dp2, normal);
    vec3 dp1perp = cross(normal, dp1);
    vec3 T = dp2perp * duv1.x + dp1perp * duv2.x;
    vec3 B = dp2perp * duv1.y + dp1perp * duv2.y;

    // Construct a scale-invariant frame
    float invmax = inversesqrt(max(dot(T,T),dot(B,B)));
    return mat3( T * invmax, B * invmax, normal );
  }

  // vNormal -> view space vertex normal.
  // txNormal -> normal map texel.
  // viewPos -> view space vertex.
  vec3 perturbNormal ( vec3 vNormal, vec3 txNormal, vec3 viewPos, vec2 texCoord )
  {
    // Sign expansion, more precise than (normal * 2) - 1.
    txNormal = txNormal * (255.0/127.0) - 128.0/127.0;
    mat3 tbn = cotangentFrame(vNormal, viewPos, texCoord);
    return normalize(tbn * txNormal);
  }

#endif /* FRAGMENT_SHADER define. */

/* Post process */

vec3 chromaticAberration (
  sampler2D smp,
  vec2 texCoord,
  vec3 texel,
  vec3 toCenterTint,
  vec3 awayCenterTint,
  float strength
  )
{
  vec2 dirToCenter =  - vec2(0.5, 0.5);

  vec2 offset = dirToCenter * strength;
  vec2 offsetTo = texCoord + offset;
  vec2 offsetAway = texCoord - offset;

  vec3 toTexel = texture(smp, offsetTo).xyz * toCenterTint;
  vec3 awayTexel = texture(smp, offsetAway).xyz * awayCenterTint;

  return texel + toTexel + awayTexel;
}

vec3 normalizeLuminosity(vec3 reference, vec3 after)
{
  float refLum = luminosity(reference);
  float aftLum = luminosity(after);

  return after * (aftLum == 0.0 ? 1.0 : (refLum/aftLum));
}
/* Lighting related functions. */

float normalizeShininess ( float shininess )
{
  return (shininess+1.0) * NUM_INV_TAU;
}

float storeShininess ( float shininess )
{
  float scale = 127.0 / 255.0;
  return saturate(((shininess*scale)+128.0) / 255.0);
}

float remapShininess ( float shininess )
{
  return clamp(pow(shininess * 2.0, 8.0), 0.0, 255.0);
}

float computeSpecular ( vec3 viewPos, vec3 lgtDir, vec3 viewNormal, float specIntensity, float shininess )
{
  // Dir to eye.
  vec3 nmEyeDir = normalize( -viewPos );
  // Half vector.
  vec3 hVector = normalize(lgtDir + nmEyeDir);
  // Calculate specular.
  float tmpSpec = saturate(dot(viewNormal, hVector));
  shininess = remapShininess(shininess);

  // Calculate fresnel.
  //float base = 1.0 - tmpSpec;
  //float exp = pow(base, 5);
  //float fresnel = 0.018 + (1-0.018)*exp;
  // Factor in shininess and specular map.
  return specIntensity * pow(tmpSpec, shininess) * normalizeShininess(shininess);
}

float computeDiffuse ( vec3 normal, vec3 lgtDir )
{
  return max(0.0, dot(normal, lgtDir));
}

float linearAttenuation ( vec3 lgtCenter, vec3 viewPos, float invRadius )
{
  float att = length(lgtCenter - viewPos) * invRadius;
  return max(0.0, 1.0 - att);
}

float quadraticAttenuation ( vec3 lgtCenter, vec3 viewPos, float invRadius )
{
  float att = sqrt(length(lgtCenter - viewPos)*invRadius);
  return max(0.0, 1.0 - att);
}

#endif // SHARED_GLSL