#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
// Output parameters.
layout (location = 0) out vec2 passTexCoord;

// layout (location = 0) in vec3 inPos;
// layout (location = 2) in vec3 inColor;

vec2 texCoordFromVID(int vertexId)
{
  // 0
  // 0, 0
  // -1, -1, 0
  // 1
  // 2, 0
  // 3, -1, 0
  // 2
  // 0, 2
  // -1, 3, 0
  return  vertexId == 0 ? vec2(0, 0) :
          vertexId == 1 ? vec2(2, 0) : vec2(0, 2);
  // return  vertexId == 0 ? vec2(-1, 1) :
  //         vertexId == 1 ? vec2(1,1) : vec2(0, -1);  
  // return  vertexId == 0 ? vec2(-2, 1) :
  //         vertexId == 1 ? vec2(2,1) : vec2(0, -3);
}

void main()
{
  passTexCoord = texCoordFromVID(gl_VertexIndex);
  gl_Position = vec4((passTexCoord * 2.0 - 1.0), 0.0, 1.0);
  // gl_Position = vec4(passTexCoord, 0.0, 1.0);
}