#ifndef SHARED_OPENGL_GLSL
#define SHARED_OPENGL_GLSL

#ifdef IS_EXTERNAL_COMPILER
#extension GL_GOOGLE_include_directive : require 
#else
#extension GL_ARB_shading_language_include : require
#endif

//#pragma optionNV(strict on)
#extension GL_ARB_shading_language_420pack : require

#include "shared.glsl.frag"

// Vertex attribute locations.
#define ATTRIB_LOC_POSITION 0
#define ATTRIB_LOC_NORMAL 	1
#define ATTRIB_LOC_COLOR 	2
#define ATTRIB_LOC_TEXCOORD 3
#define ATTRIB_LOC_JOINT_WEIGHT 4
#define ATTRIB_LOC_INSTANCE_ID 5
// Render target indices.
#define RT_0 0
#define RT_1 1
#define RT_2 2
#define RT_3 3
#define RT_4 4
#define RT_5 5
#define RT_6 6
#define RT_7 7
// Texture sampler bindings.
#define TEX_DIFFUSE 0
#define TEX_NORMAL 1
#define TEX_GLOW 2
#define TEX_ENV 3
// Texture sampler bindings.
#define SMP_TEX_0  0
#define SMP_TEX_1  1
#define SMP_TEX_2  2
#define SMP_TEX_3  3
#define SMP_TEX_4  4
#define SMP_TEX_5  5
#define SMP_TEX_6  6
#define SMP_TEX_7  7
// FBO attachment bindings.
#define SMP_RT_0  16
#define SMP_RT_1  17
#define SMP_RT_2  18
#define SMP_RT_3  19
#define SMP_RT_4  20
#define SMP_RT_5  21
#define SMP_RT_6  22
#define SMP_RT_7  23
#define SMP_RT_8  24
#define SMP_RT_9  25
#define SMP_RT_10  26
// Shadow sampler bindings.
#define SMP_SHW_0  24
#define SMP_SHW_1  25
#define SMP_SHW_2  26
#define SMP_SHW_3  27
#define SMP_SHW_4  28
#define SMP_SHW_5  29
#define SMP_SHW_6  30
#define SMP_SHW_7  31
// UBO binding points.
#define BIND_UBO_TRANSFORMS 0
#define BIND_UBO_MATERIAL 1
#define BIND_UBO_DIRLIGHT 2
#define BIND_UBO_FRUSTUM 3
#define BIND_UBO_VIEWRAY 4
#define BIND_UBO_POINTLIGHT 5
#define BIND_UBO_SPOTLIGHT 6
#define BIND_UBO_JOINT 7
#define BIND_UBO_SKY 8
#define BIND_UBO_STATIC_SHADOW 9
#define BIND_UBO_TRANSFORM_EXTRA 10
// UBO names
#define UBO_TRANSFORMS_NAME	TransformBlock
#define UBO_MATERIAL_NAME	MaterialBlock
#define	UBO_DIRLIGHT_NAME	DirLightBlock
#define UBO_FRUSTUM_NAME	FrustumBlock
#define	UBO_VIEWRAY_NAME	ViewRayBlock
#define	UBO_POINTLIGHT_NAME PointLightBlock
#define UBO_SPOTLIGHT_NAME  SpotLightBlock
#define UBO_JOINT_NAME  JointBlock
#define UBO_SKY_NAME  SkyBlock
#define	UBO_STATIC_SHADOW_NAME	StaticShadowBlock
#define UBO_TRANSFORM_EXTRA_NAME TransformExtraBlock
/*** UBO size limits. ****/
// 64kB size limit for UBO
#define UBO_MAX_SIZE 65536
#define UBO_TRANSFORM_SIZE 128
#define UBO_MATERIAL_SIZE 16
#define UBO_DIRLIGHT_SIZE 128
#define UBO_FRUSTUM_SIZE 32
#define UBO_VIEWRAY_SIZE 64
#define UBO_POINTLIGHT_SIZE 16
#define UBO_SPOTLIGHT_SIZE 32
#define UBO_JOINT_SIZE 48
#define UBO_SKY_SIZE 96
#define UBO_STATIC_SHADOW_SIZE 64
#define UBO_TRANSFORM_EXTRA_SIZE 64
// divided by size of Transform struct.
#define MAX_UBO_TRANSFORMS	UBO_MAX_SIZE / UBO_TRANSFORM_SIZE
// divided by size of Material struct.
#define MAX_UBO_MATERIALS	UBO_MAX_SIZE / UBO_MATERIAL_SIZE
// divided by size of PointLight struct
#define MAX_UBO_POINTLIGHTS UBO_MAX_SIZE / UBO_POINTLIGHT_SIZE
// divided by size of SpotLight struct.
#define MAX_UBO_SPOTLIGHTS UBO_MAX_SIZE / UBO_SPOTLIGHT_SIZE
// divided by size of Joint.
#define MAX_UBO_JOINTS UBO_MAX_SIZE / UBO_JOINT_SIZE
// divided by size of StaticShadow struct.
#define MAX_UBO_STATIC_SHADOW UBO_MAX_SIZE / UBO_STATIC_SHADOW_SIZE
// divided by size of TransformExtra struct.
#define MAX_UBO_TRANSFORM_EXTRA UBO_MAX_SIZE / UBO_TRANSFORM_EXTRA_SIZE

#define READ_INST_TRANSFORM_MACRO transforms[passInstanceId]
#define READ_INST_MATERIAL_MACRO materials[passInstanceId]
#define READ_INST_DIRLIGHT_MACRO dirLight
#define READ_INST_FRUSTUM_MACRO frustum
#define READ_INST_VIEWRAY_MACRO viewRays
#define READ_INST_POINTLIGHT_MACRO pointLights[passInstanceId]
#define READ_INST_SPOTLIGHT_MACRO spotLights[passInstanceId]
#define READ_INST_JOINT_MACRO joints[passInstanceId]
#define READ_INST_SKY_MACRO sky
#define READ_INST_STATIC_SHADOW_MACRO staticShadows[passInstanceId]
#define READ_INST_TRANSFORM_EXTRA_MACRO transformExtras[passInstanceId]
// UBO macro expansions.
#define USING_UBO_TRANSFORM_MACRO layout ( std140, binding = BIND_UBO_TRANSFORMS ) uniform UBO_TRANSFORMS_NAME {  Transform[MAX_UBO_TRANSFORMS] transforms; }
#define USING_UBO_MATERIAL_MACRO layout ( std140, binding = BIND_UBO_MATERIAL ) uniform UBO_MATERIAL_NAME {  Material[MAX_UBO_MATERIALS] materials; }
#define USING_UBO_DIRLIGHT_MACRO layout ( std140, binding = BIND_UBO_DIRLIGHT ) uniform UBO_DIRLIGHT_NAME {  DirLight dirLight; }
#define USING_UBO_FRUSTUM_MACRO layout ( std140, binding = BIND_UBO_FRUSTUM ) uniform UBO_FRUSTUM_NAME {  Frustum frustum; }
#define USING_UBO_VIEWRAY_MACRO layout ( std140, binding = BIND_UBO_VIEWRAY ) uniform UBO_VIEWRAY_NAME {  ViewRays viewRays; }
#define USING_UBO_POINTLIGHT_MACRO layout ( std140, binding = BIND_UBO_POINTLIGHT ) uniform UBO_POINTLIGHT_NAME {  PointLight[MAX_UBO_POINTLIGHTS] pointLights; }
#define USING_UBO_SPOTLIGHT_MACRO layout ( std140, binding = BIND_UBO_SPOTLIGHT ) uniform UBO_SPOTLIGHT_NAME {  SpotLight[MAX_UBO_SPOTLIGHTS] spotLights; }
#define USING_UBO_JOINT_MACRO layout ( std140, binding = BIND_UBO_JOINT ) uniform UBO_JOINT_NAME {  Joint[MAX_UBO_JOINTS] joints; }
#define USING_UBO_SKY_MACRO layout ( std140, binding = BIND_UBO_SKY ) uniform UBO_SKY_NAME {  Sky sky; }
#define USING_UBO_STATIC_SHADOW_MACRO layout ( std140, binding = BIND_UBO_STATIC_SHADOW ) uniform UBO_STATIC_SHADOW_NAME {  StaticShadow[MAX_UBO_STATIC_SHADOW] staticShadows; }
#define USING_UBO_TRANSFORM_EXTRA_MACRO layout ( std140, binding = BIND_UBO_TRANSFORM_EXTRA ) uniform UBO_TRANSFORM_EXTRA_NAME {  TransformExtra[MAX_UBO_TRANSFORM_EXTRA] transformExtras; }

#define READ_ATTR_POSITION_MACRO inPosition
#define READ_ATTR_NORMAL_MACRO inNormal
#define READ_ATTR_COLOR_MACRO inColor
#define READ_ATTR_TEXCOORD_MACRO inTexCoord
#define READ_ATTR_JOINT_WEIGHT_MACRO inJointWeight
#define READ_ATTR_INSTANCE_ID_MACRO inInstanceId
// Input attribute macro expansions.
#define USING_ATTR_POSITION_MACRO layout ( location = ATTRIB_LOC_POSITION ) in vec3 inPosition
#define USING_ATTR_NORMAL_MACRO layout ( location = ATTRIB_LOC_NORMAL ) in vec3 inNormal
#define USING_ATTR_COLOR_MACRO layout ( location = ATTRIB_LOC_COLOR ) in vec3 inColor
#define USING_ATTR_TEXCOORD_MACRO layout ( location = ATTRIB_LOC_TEXCOORD ) in vec2 inTexCoord
#define USING_ATTR_JOINT_WEIGHT_MACRO layout ( location = ATTRIB_LOC_JOINT_WEIGHT ) in uvec3 inJointWeight
#define USING_ATTR_INSTANCE_ID_MACRO layout ( location = ATTRIB_LOC_INSTANCE_ID ) in int inInstanceId

#define USING(TYPE, NAME) USING_##TYPE##_##NAME##_MACRO

#define SAMPLING(NAME, SRC, TYPE, INDEX) layout ( binding = SRC##_##INDEX ) uniform sampler##TYPE NAME

#define WRITING(NAME, TYPE, INDEX) layout ( location = RT_##INDEX ) out TYPE NAME

#define READ(TYPE,NAME) READ_##TYPE##_##NAME##_MACRO

// No separate sampler-images in GL so the macro expansion is basic
#define SAMPLER_FOR(TYPE, NAME, ID) NAME

/* 
* These macros are unused in the OpenGL pipeline, 
* define them here to avoid compiler errors.
*/
// No push constant block required in GL
#define INPUTS_BEGIN 
#define INPUTS_END
// Attribs in GL are matched by name
#define ATTR_LOC(POS)
// No push constant padding necesary in GL
#define UNUSED_INPUT(IDX)
// No descriptor sets in GL
#define DESCRIPTOR(TYPE, NAME, BIND)

#endif // SHARED_OPENGL_GLSL