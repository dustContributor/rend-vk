#ifndef SHARED_OPENGL_GLSL
#define SHARED_OPENGL_GLSL

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

//#pragma optionNV(strict on)
#extension GL_ARB_shading_language_420pack : require
#extension GL_ARB_shading_language_packing : require
// Got renamed in Vulkan
#define gl_VertexIndex gl_VertexID

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
#define BIND_UBO_GLOBAL 0
#define BIND_UBO_PER_PASS 1
#define BIND_UBO_TRANSFORM 2
#define BIND_UBO_MATERIAL 3
#define BIND_UBO_DIR_LIGHT 4
#define BIND_UBO_FRUSTUM 5
#define BIND_UBO_VIEW_RAY 6
#define BIND_UBO_POINT_LIGHT 7
#define BIND_UBO_SPOT_LIGHT 8
//#define BIND_UBO_JOINT 9
//#define BIND_UBO_SKY 10
#define BIND_UBO_STATIC_SHADOW 11
#define BIND_UBO_TRANSFORM_EXTRA 12
#define BIND_UBO_VIEW 13
#define BIND_UBO_TIMING 14
// UBO names
#define UBO_TRANSFORMS_NAME	TransformBlock
#define UBO_MATERIAL_NAME MaterialBlock
#define	UBO_DIRLIGHT_NAME DirLightBlock
#define UBO_FRUSTUM_NAME FrustumBlock
#define	UBO_VIEWRAY_NAME ViewRayBlock
#define	UBO_POINTLIGHT_NAME PointLightBlock
#define UBO_SPOTLIGHT_NAME SpotLightBlock
#define UBO_JOINT_NAME JointBlock
#define UBO_SKY_NAME SkyBlock
#define	UBO_STATIC_SHADOW_NAME StaticShadowBlock
#define UBO_TRANSFORM_EXTRA_NAME TransformExtraBlock
#define UBO_VIEW_NAME ViewBlock
#define UBO_TIMING_NAME TimingBlock
#define UBO_PER_PASS_NAME PerPassBlock
/*** UBO size limits. ****/
// 64kB size limit for UBO
#define UBO_MAX_SIZE 65536
#define UBO_TRANSFORM_SIZE 128
#define UBO_MATERIAL_SIZE 16
#define UBO_DIRLIGHT_SIZE 352
#define UBO_FRUSTUM_SIZE 48
#define UBO_VIEWRAY_SIZE 64
#define UBO_POINTLIGHT_SIZE 16
#define UBO_SPOTLIGHT_SIZE 48
#define UBO_JOINT_SIZE 48
#define UBO_SKY_SIZE 96
#define UBO_STATIC_SHADOW_SIZE 16
#define UBO_TRANSFORM_EXTRA_SIZE 64
#define UBO_VIEW_SIZE 512
#define UBO_TIMING_SIZE 16

#define MAX_UBO_TRANSFORMS	UBO_MAX_SIZE / UBO_TRANSFORM_SIZE
#define MAX_UBO_MATERIALS	UBO_MAX_SIZE / UBO_MATERIAL_SIZE
#define MAX_UBO_DIRLIGHTS UBO_MAX_SIZE / UBO_DIRLIGHT_SIZE
#define MAX_UBO_POINTLIGHTS UBO_MAX_SIZE / UBO_POINTLIGHT_SIZE
#define MAX_UBO_SPOTLIGHTS UBO_MAX_SIZE / UBO_SPOTLIGHT_SIZE
#define MAX_UBO_JOINTS UBO_MAX_SIZE / UBO_JOINT_SIZE
#define MAX_UBO_STATIC_SHADOW UBO_MAX_SIZE / UBO_STATIC_SHADOW_SIZE
#define MAX_UBO_TRANSFORM_EXTRA UBO_MAX_SIZE / UBO_TRANSFORM_EXTRA_SIZE

// Per instance data
#define READ_INST_TRANSFORM_MACRO transforms[passInstanceId]
#define READ_INST_MATERIAL_MACRO materials[passInstanceId]
#define READ_INST_DIRLIGHT_MACRO dirLights[passInstanceId]
#define READ_INST_POINTLIGHT_MACRO pointLights[passInstanceId]
#define READ_INST_SPOTLIGHT_MACRO spotLights[passInstanceId]
#define READ_INST_JOINT_MACRO joints[passInstanceId]
#define READ_INST_STATIC_SHADOW_MACRO staticShadows[passInstanceId]
#define READ_INST_TRANSFORM_EXTRA_MACRO transformExtras[passInstanceId]


// Per pass constant data
#define READ_CONST(NAME) NAME

// Per pass data
#define READ_PASS_DIRLIGHT_MACRO dirLight
#define READ_PASS_FRUSTUM_MACRO frustum
#define READ_PASS_VIEWRAY_MACRO viewRay
#define READ_PASS_VIEW_MACRO view
#define READ_PASS_TIMING_MACRO timing
// Per-attribute data
#define READ_ATTR_POSITION_MACRO inPosition
#define READ_ATTR_NORMAL_MACRO inNormal
#define READ_ATTR_COLOR_MACRO unpackUnorm4x8(inColor)
#define READ_ATTR_TEXCOORD_MACRO inTexCoord
#define READ_ATTR_JOINT_WEIGHT_MACRO inJointWeight
// Base attribute/instance read macro expansion
#define READ(TYPE,NAME) READ_##TYPE##_##NAME##_MACRO

// UBO macro expansions for per-instance data
#define USING_INST_TRANSFORM_MACRO layout ( std140, binding = BIND_UBO_TRANSFORM ) uniform UBO_TRANSFORMS_NAME {  Transform[MAX_UBO_TRANSFORMS] transforms; };
#define USING_INST_MATERIAL_MACRO layout ( std140, binding = BIND_UBO_MATERIAL ) uniform UBO_MATERIAL_NAME {  Material[MAX_UBO_MATERIALS] materials; };
#define USING_INST_DIRLIGHT_MACRO layout ( std140, binding = BIND_UBO_DIR_LIGHT ) uniform UBO_DIRLIGHT_NAME {  DirLight[MAX_UBO_DIRLIGHTS] dirLights; };
#define USING_INST_FRUSTUM_MACRO layout ( std140, binding = BIND_UBO_FRUSTUM ) uniform UBO_FRUSTUM_NAME {  Frustum frustum; };
#define USING_INST_VIEWRAY_MACRO layout ( std140, binding = BIND_UBO_VIEW_RAY ) uniform UBO_VIEWRAY_NAME {  ViewRay viewRay; };
#define USING_INST_POINTLIGHT_MACRO layout ( std140, binding = BIND_UBO_POINT_LIGHT ) uniform UBO_POINTLIGHT_NAME {  PointLight[MAX_UBO_POINTLIGHTS] pointLights; };
#define USING_INST_SPOTLIGHT_MACRO layout ( std140, binding = BIND_UBO_SPOT_LIGHT ) uniform UBO_SPOTLIGHT_NAME {  SpotLight[MAX_UBO_SPOTLIGHTS] spotLights; };
#define USING_INST_JOINT_MACRO layout ( std140, binding = BIND_UBO_JOINT ) uniform UBO_JOINT_NAME {  Joint[MAX_UBO_JOINTS] joints; };
#define USING_INST_SKY_MACRO layout ( std140, binding = BIND_UBO_SKY ) uniform UBO_SKY_NAME {  Sky sky; };
#define USING_INST_STATIC_SHADOW_MACRO layout ( std140, binding = BIND_UBO_STATIC_SHADOW ) uniform UBO_STATIC_SHADOW_NAME {  StaticShadow[MAX_UBO_STATIC_SHADOW] staticShadows; };
#define USING_INST_TRANSFORM_EXTRA_MACRO layout ( std140, binding = BIND_UBO_TRANSFORM_EXTRA ) uniform UBO_TRANSFORM_EXTRA_NAME {  TransformExtra[MAX_UBO_TRANSFORM_EXTRA] transformExtras; };
// Per-pass data definitions
#define USING_PASS_DIRLIGHT_MACRO DirLight dirLight;
#define USING_PASS_FRUSTUM_MACRO Frustum frustum;
#define USING_PASS_VIEWRAY_MACRO ViewRay viewRay;
#define USING_PASS_VIEW_MACRO View view;
#define USING_PASS_TIMING_MACRO Timing timing;

// Input attribute macro expansions.
#define USING_ATTR_POSITION_MACRO layout ( location = ATTRIB_LOC_POSITION ) in vec3 inPosition;
#define USING_ATTR_NORMAL_MACRO layout ( location = ATTRIB_LOC_NORMAL ) in vec3 inNormal;
#define USING_ATTR_COLOR_MACRO layout ( location = ATTRIB_LOC_COLOR ) in uint inColor;
#define USING_ATTR_TEXCOORD_MACRO layout ( location = ATTRIB_LOC_TEXCOORD ) in vec2 inTexCoord;
#define USING_ATTR_JOINT_WEIGHT_MACRO layout ( location = ATTRIB_LOC_JOINT_WEIGHT ) in uvec3 inJointWeight;

// On GL this is an attribute, but use it as conceptually "per instance" data
#define READ_INST_INSTANCE_ID_MACRO inInstanceId
#define USING_INST_INSTANCE_ID_MACRO layout ( location = ATTRIB_LOC_INSTANCE_ID ) in int inInstanceId;

#define USING(TYPE, NAME) USING_##TYPE##_##NAME##_MACRO

#define SAMPLING(NAME, SRC, TYPE, INDEX) layout ( binding = SRC##_##INDEX ) uniform sampler##TYPE NAME;

#define WRITING(NAME, TYPE, INDEX) layout ( location = RT_##INDEX ) out TYPE NAME

// No separate sampler-images in GL so the macro expansion is basic
#define SAMPLER_FOR(NAME, TYPE, TIDX, SIDX)  NAME
// Per pass data gets concatenated in a single UBO
#define PASS_DATA_BEGIN \
layout ( std140, binding = BIND_UBO_PER_PASS ) uniform UBO_PER_PASS_NAME { 
#define PASS_DATA_END \
};

/* 
* These macros are unused in the OpenGL pipeline, 
* define them here to avoid compiler errors.
*/
// No push constant block required in GL
#define INPUTS_BEGIN 
#define INPUTS_END
// No push constant block required in GL
#define USING_PASS_DATA_MACRO
// Attribs in GL are matched by name
#define ATTR_LOC(POS)
// No push constant padding necesary in GL
#define UNUSED_INPUT(IDX)
// No descriptor sets in GL
#define DESCRIPTOR(TYPE, NAME, BIND)

#endif // SHARED_OPENGL_GLSL