#ifdef IS_EXTERNAL_COMPILER
#extension GL_GOOGLE_include_directive : require 
#else
#extension GL_ARB_shading_language_include : require
#endif

#ifdef IS_VULKAN
#include "shared_vulkan.glsl.frag"
#else
#include "shared_opengl.glsl.frag"
#endif