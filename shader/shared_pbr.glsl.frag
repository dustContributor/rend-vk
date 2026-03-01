#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#ifdef IS_VULKAN
#extension GL_EXT_shader_explicit_arithmetic_types : require 
#endif

#ifndef SHARED_GLSL
#include "shared.glsl.frag"
#endif

vec3 FresnelSchlick(float cosTheta, vec3 f0, vec3 f90)
{
    return mix(f0, f90, pow(saturate(1.0 - cosTheta), 5.0));
}

float D_GGX(float NdotH, float roughness) {
    float a = NdotH * roughness;
    float k = roughness / (1.0 - NdotH * NdotH + a * a);
    return k * k;
}

float V_SmithGGXCorrelated(float NdotV, float NdotL, float roughness) {
    float a = roughness;
    float GGXV = NdotL * (NdotV * (1.0 - a) + a);
    float GGXL = NdotV * (NdotL * (1.0 - a) + a);
    return 0.5 / max(GGXV + GGXL, NUM_MIN_NORMAL);
}

vec3 doLighting(
    vec3 lightColor, 
    vec3 L, /* direction towards light */
    float multiplier, /* shadow map sample * light falloff */
    vec3 V, /* direction towards camera */
    vec3 N, /* normal */
    float metallic,
    float roughness,
    vec3 diffuse /* simple lambert diffuse (normalized by dividing by pi later) */
) {    
    vec3 F0 = mix(vec3(0.04), diffuse, metallic);
    // halfway vector between V and L
    vec3 H = normalize(V + L);
    
    float NdotL = saturate(dot(N, L));
    
    float NdotV = saturate(dot(N, V));
    float VdotH = saturate(dot(V, H));
    float NdotH = saturate(dot(N, H));
    
    float r2 = roughness * roughness;
    vec3 F = FresnelSchlick(VdotH, F0, vec3(1, 1, 1));
    // roughness normal distribution, specular higlight
    float D = D_GGX(NdotH, r2);
    // visibility function
    float G = V_SmithGGXCorrelated(NdotV, NdotL, r2);
    
    float mul = multiplier * NdotL * NUM_INV_PI;
    
    float specular = D * G;
    // blend based on fresnel
    vec3 light = mix(diffuse, vec3(specular), F); 
    // multiply by incoming light.
    return lightColor * mul * light;
}