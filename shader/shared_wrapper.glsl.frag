#ifndef SHARED_WRAPPER_GLSL
#define SHARED_WRAPPER_GLSL
/* 
  Both extensions are enabled so:
  1. GL_GOOGLE_include_directive is used by glslangValidator on both 
    Vulkan and GL, in GL it's used to pre-process all the shaders.
  2. GL_ARB_shading_language_include is used so the GL driver can 
    interpret the line directives the pre-processor generates.
*/
#ifndef EXT_INCLUDE_GLSL
#define EXT_INCLUDE_GLSL
#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 
#endif

#ifdef IS_VULKAN
#include "shared_vulkan.glsl.frag"
#else
#include "shared_opengl.glsl.frag"
#endif

#endif // SHARED_WRAPPER_GLSL