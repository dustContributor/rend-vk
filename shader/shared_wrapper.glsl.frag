/* 
  Both extensions are enabled so:
  1. GL_GOOGLE_include_directive is used by glslangValidator on both 
    Vulkan and GL, in GL it's used to pre-process all the shaders.
  2. GL_ARB_shading_language_include is used so the GL driver can 
    interpret the line directives the pre-processor generates.
*/
#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#ifdef IS_VULKAN
#include "shared_vulkan.glsl.frag"
#else
#include "shared_opengl.glsl.frag"
#endif