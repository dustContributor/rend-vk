#ifndef SHARED_VULKAN_GLSL
#define SHARED_VULKAN_GLSL

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#extension GL_ARB_separate_shader_objects : require
#extension GL_ARB_shading_language_420pack : require
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_EXT_buffer_reference : require
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_shader_explicit_arithmetic_types : require
// #extension GL_EXT_debug_printf : enable

#include "shared.glsl.frag"

#define BUFFER_REF(NAME, TYPE) \
layout(scalar, buffer_reference, buffer_reference_align = 8) readonly buffer NAME { TYPE items[]; };
// Per vertex data
BUFFER_REF(Positions, vec3)
BUFFER_REF(Normals, vec3)
BUFFER_REF(TexCoords, vec2)
BUFFER_REF(Colors, uint)
// Per instance data
BUFFER_REF(Transforms, Transform)
BUFFER_REF(Materials, Material)
BUFFER_REF(DirLights, DirLight)
BUFFER_REF(PointLights, PointLight)
BUFFER_REF(SpotLights, SpotLight)
BUFFER_REF(StaticShadows, StaticShadow)
BUFFER_REF(TransformExtras, TransformExtra)
// Per pass data

#define DESC_SET_SAMPLER 0
#define DESC_SET_TEXTURE 1
#define DESC_SET_ATTACHMENT 2

// Per vertex attributes
#define READ_ATTR_POSITION_MACRO registers.positions.items[gl_VertexIndex]
#define READ_ATTR_NORMAL_MACRO registers.normals.items[gl_VertexIndex]
#define READ_ATTR_COLOR_MACRO unpackUnorm4x8(registers.colors.items[gl_VertexIndex])
#define READ_ATTR_TEXCOORD_MACRO registers.texCoords.items[gl_VertexIndex]
#define READ_ATTR_JOINT_WEIGHT_MACRO registers.joints.items[gl_VertexIndex]
// Per instance data
#define READ_INST_INSTANCE_ID_MACRO gl_InstanceIndex
#define READ_INST_TRANSFORM_MACRO registers.transforms.items[passInstanceId]
#define READ_INST_MATERIAL_MACRO registers.materials.items[passInstanceId]
#define READ_INST_DIRLIGHT_MACRO registers.dirLights.items[passInstanceId]
#define READ_INST_FRUSTUM_MACRO registers.frustums.items[passInstanceId]
#define READ_INST_VIEWRAY_MACRO registers.viewRays.items[passInstanceId]
#define READ_INST_POINTLIGHT_MACRO registers.pointLights.items[passInstanceId]
#define READ_INST_SPOTLIGHT_MACRO registers.spotLights.items[passInstanceId]
#define READ_INST_JOINT_MACRO registers.joints.items[passInstanceId]
#define READ_INST_SKY_MACRO registers.skies.items[passInstanceId]
#define READ_INST_STATIC_SHADOW_MACRO registers.staticShadows.items[passInstanceId]
#define READ_INST_TRANSFORM_EXTRA_MACRO registers.transformExtras.items[passInstanceId]
// Per pass constant data
#define READ_CONST(name) registers.pass. name
// Per pass data
#define READ_PASS_TRANSFORM_MACRO registers.pass.transform
#define READ_PASS_MATERIAL_MACRO registers.pass.material
#define READ_PASS_DIRLIGHT_MACRO registers.pass.dirLight
#define READ_PASS_FRUSTUM_MACRO registers.pass.frustum
#define READ_PASS_VIEWRAY_MACRO registers.pass.viewRay
#define READ_PASS_JOINT_MACRO registers.pass.joint
#define READ_PASS_SKY_MACRO registers.pass.sky
#define READ_PASS_STATIC_SHADOW_MACRO registers.pass.staticShadow
#define READ_PASS_TRANSFORM_EXTRA_MACRO registers.pass.transformExtra
#define READ_PASS_VIEW_MACRO registers.pass.view
#define READ_PASS_TIMING_MACRO registers.pass.timing
// Base attribute/instance read macro expansion
#define READ(TYPE,NAME) READ_##TYPE##_##NAME##_MACRO

// Default and pre-defined descriptor sets
#define DESCRIPTOR_SAMPLER_DEFAULT_MACRO(BIND) layout (set = BIND, binding = 0) uniform sampler[] samplers;
#define DESCRIPTOR_TEXTURE_DEFAULT_MACRO(BIND) layout (set = BIND, binding = 0) uniform texture2D[] textures;
#define DESCRIPTOR_TEXTURE_CUBE_MACRO(BIND) layout (set = BIND, binding = 0) uniform textureCube[] textures;
#define DESCRIPTOR_SAMPLER_MACRO(NAME, BIND) DESCRIPTOR_SAMPLER_##NAME##_MACRO(BIND)
#define DESCRIPTOR_TEXTURE_MACRO(NAME, BIND) DESCRIPTOR_TEXTURE_##NAME##_MACRO(BIND)
#define DESCRIPTOR_TARGET_IMAGE_MACRO(NAME, BIND) layout (set = DESC_SET_TARGET_IMAGE, binding = BIND) uniform texture2D NAME;
// Base descriptor set macro expansion
#define DESCRIPTOR(TYPE, NAME, BIND) DESCRIPTOR_##TYPE##_MACRO(NAME,BIND)

/* 
* Padding to share BDA blocks between shaders without 
* having to declare unused addresses
*/
#define UNUSED_INPUT(IDX) int padding##IDX##0;int padding##IDX##1;

// Vertex attribute definitions
#define USING_ATTR_POSITION_MACRO Positions positions;
#define USING_ATTR_NORMAL_MACRO Normals normals;
#define USING_ATTR_TEXCOORD_MACRO TexCoords texCoords;
#define USING_ATTR_COLOR_MACRO Colors colors;
// Per-instance data definitions
#define USING_INST_TRANSFORM_MACRO Transforms transforms;
#define USING_INST_MATERIAL_MACRO Materials materials;
#define USING_INST_DIRLIGHT_MACRO DirLights dirLights;
#define USING_INST_POINTLIGHT_MACRO PointLights pointLights;
#define USING_INST_SPOTLIGHT_MACRO SpotLights spotLights;
#define USING_INST_STATIC_SHADOW_MACRO StaticShadows staticShadows;
#define USING_INST_TRANSFORM_EXTRA_MACRO TransformExtras transformExtras;
// Per-pass data definitions
#define USING_PASS_TRANSFORM_MACRO Transform transform;
#define USING_PASS_MATERIAL_MACRO Material material;
#define USING_PASS_DIRLIGHT_MACRO DirLight dirLight;
#define USING_PASS_FRUSTUM_MACRO Frustum frustum;
#define USING_PASS_VIEWRAY_MACRO ViewRay viewRay;
#define USING_PASS_TRANSFORM_EXTRA_MACRO TransformExtra transformExtra;
#define USING_PASS_VIEW_MACRO View view;
#define USING_PASS_TIMING_MACRO Timing timing;
// This struct will hold all the per-pass data together
#define USING_PASS_DATA_MACRO PassData pass;
// Using pre-defined gl_InstanceIndex in vulkan
#define USING_INST_INSTANCE_ID_MACRO

#define USING(TYPE,NAME) USING_##TYPE##_##NAME##_MACRO

#define INPUTS_BEGIN \
layout(scalar, push_constant) uniform Registers {
#define INPUTS_END \
}\
registers;

#define PASS_DATA_BEGIN \
layout(scalar, buffer_reference, buffer_reference_align = 8) readonly buffer PassData {
#define PASS_DATA_END \
};
// Render target writing
#define WRITING(NAME, TYPE, INDEX) layout ( location = INDEX ) out TYPE NAME
// Output attribute location
#define ATTR_LOC(POS) layout (location = POS)
// Separate image-sampler usage
#define SAMPLER_FOR(NAME, TYPE, TIDX, SIDX) sampler##TYPE## \( textures[nonuniformEXT(TIDX)], samplers[nonuniformEXT(SIDX)] )
/* 
* These macros are unused in the Vulkan pipeline, 
* define them here to avoid compiler errors.
*/
#define SAMPLING_SHW_TEX(NAME, TYPE, INDEX) 
#define SAMPLING_SMP_TEX(NAME, TYPE, INDEX) 
// With render targets instead we directly map them to descriptors at the specified binding points
#define SAMPLING_SMP_RT(NAME, TYPE, INDEX) \
layout (set = DESC_SET_ATTACHMENT, binding = INDEX) uniform sampler##TYPE NAME;

#define SAMPLING(NAME, SRC, TYPE, INDEX) SAMPLING_##SRC(NAME, TYPE, INDEX)

#endif // SHARED_VULKAN_GLSL