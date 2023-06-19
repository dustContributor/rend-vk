#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_buffer_reference : require
#extension GL_EXT_scalar_block_layout : require

layout(scalar, std430, buffer_reference, buffer_reference_align = 8) readonly buffer Vertices
{
    vec3 items[];
};

layout(scalar, std430, buffer_reference, buffer_reference_align = 8) readonly buffer Normals
{
    vec3 items[];
};

layout(push_constant) uniform Registers
{
    Vertices vertices;
    Normals normals;
} registers;

layout (location = 0) out vec3 passColor;

void main() {
    restrict vec3 inPos = registers.vertices.items[gl_VertexIndex];
    // restrict vec3 normals = registers.normals.items[gl_VertexIndex];
    passColor = inPos.xyz;
    gl_Position = vec4(inPos, 1.0);
}