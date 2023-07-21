#ifndef SHARED_VULKAN_GLSL
#define SHARED_VULKAN_GLSL

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_buffer_reference : require
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_debug_printf : enable

// Add the ifdef even if this file is api specific to avoid complaints from glslang
#ifdef IS_VULKAN
#extension GL_GOOGLE_include_directive : require 
#else
#extension GL_ARB_shading_language_include : require
#endif

#include "shared.glsl.frag"

layout(scalar, buffer_reference, buffer_reference_align = 8) readonly buffer Positions
{
    vec3 items[];
};
layout(scalar, buffer_reference, buffer_reference_align = 8) readonly buffer Normals
{
    vec3 items[];
};
layout(scalar, buffer_reference, buffer_reference_align = 8) readonly buffer TexCoords
{
    vec2 items[];
};
layout(scalar, buffer_reference, buffer_reference_align = 8) readonly buffer Transforms
{
    Transform items[];
};
layout(scalar, buffer_reference, buffer_reference_align = 8) readonly buffer Materials
{
    Material items[];
};
layout(scalar, buffer_reference, buffer_reference_align = 8) readonly buffer TransformExtras
{
    TransformExtra items[];
};
// Per vertex attributes
#define READ_ATTR_POSITION_MACRO registers.positions.items[gl_VertexIndex]
#define READ_ATTR_NORMAL_MACRO registers.normals.items[gl_VertexIndex]
#define READ_ATTR_COLOR_MACRO registers.colors.items[gl_VertexIndex]
#define READ_ATTR_TEXCOORD_MACRO registers.texCoords.items[gl_VertexIndex]
#define READ_ATTR_JOINT_WEIGHT_MACRO registers.joints.items[gl_VertexIndex]
// Per instance data
#define READ_INST_INSTANCE_ID_MACRO gl_InstanceIndex
#define READ_INST_TRANSFORM_MACRO registers.transforms.items[gl_InstanceIndex]
#define READ_INST_MATERIAL_MACRO registers.materials.items[gl_InstanceIndex]
#define READ_INST_DIRLIGHT_MACRO registers.dirLight
#define READ_INST_FRUSTUM_MACRO registers.frustum
#define READ_INST_VIEWRAY_MACRO registers.viewRays
#define READ_INST_POINTLIGHT_MACRO registers.pointLights.items[gl_InstanceIndex]
#define READ_INST_SPOTLIGHT_MACRO sregisters.potLights.items[gl_InstanceIndex]
#define READ_INST_JOINT_MACRO registers.joints.items[gl_InstanceIndex]
#define READ_INST_SKY_MACRO registers.sky
#define READ_INST_STATIC_SHADOW_MACRO registers.staticShadows.items[gl_InstanceIndex]
#define READ_INST_TRANSFORM_EXTRA_MACRO registers.transformExtras.items[gl_InstanceIndex]

#define READ(TYPE,NAME) READ_##TYPE##_##NAME##_MACRO

// Padding to share BDA blocks between shaders without having to declare unused addresses
#define USING_PAD_0_MACRO int padding00;int padding01;
#define USING_PAD_1_MACRO int padding10;int padding11;
#define USING_PAD_2_MACRO int padding20;int padding21;
#define USING_PAD_3_MACRO int padding30;int padding31;
#define USING_PAD_4_MACRO int padding40;int padding41;
#define USING_PAD_5_MACRO int padding50;int padding51;
#define USING_PAD_6_MACRO int padding60;int padding61;
#define USING_PAD_7_MACRO int padding70;int padding71;

#define USING_ATTR_POSITION_MACRO Positions positions;
#define USING_ATTR_NORMAL_MACRO Normals normals;
#define USING_ATTR_TEXCOORD_MACRO TexCoords texCoords;

#define USING_INST_TRANSFORM_MACRO Transforms transforms;
#define USING_INST_MATERIAL_MACRO Materials materials;
#define USING_INST_TRANSFORM_EXTRA_MACRO TransformExtras transformExtras;
// Implicit in vulkan
#define USING_INST_INSTANCE_ID_MACRO


#define USING(TYPE,NAME) USING_##TYPE##_##NAME##_MACRO

#define INPUTS_BEGIN \
layout(push_constant) uniform Registers {
#define INPUTS_END \
}\
registers;

#define WRITING(NAME, TYPE, INDEX) layout ( location = INDEX ) out TYPE NAME

#define ATTR_LOC(POS) layout (location = POS)


#define SAMPLER_FOR(TYPE,NAME) sampler##TYPE## ( NAME, smp )

#endif // SHARED_VULKAN_GLSL