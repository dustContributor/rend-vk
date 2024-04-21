#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

PASS_DATA_BEGIN
	USING(PASS, FRUSTUM)
PASS_DATA_END

INPUTS_BEGIN
	  USING(PASS, DATA)
    USING(ATTR, POSITION)
    USING(ATTR, COLOR)
    USING(ATTR, TEXCOORD)
    UNUSED_INPUT(4) // material
    // Always last
    USING(INST, INSTANCE_ID)
INPUTS_END

// Output parameters.
ATTR_LOC(0) out vec2 passTexCoord;
ATTR_LOC(1) out vec4 passColor;
ATTR_LOC(2) flat out int passInstanceId;

void main() {
  // Instance index. Mandatory first line of main.
  passInstanceId = READ(INST, INSTANCE_ID);
  passTexCoord = READ(ATTR, TEXCOORD);
  passColor = READ(ATTR, COLOR);
  vec3 inPosition = READ(ATTR, POSITION);
	Frustum frustum = READ(PASS, FRUSTUM);
  mat4 trn = mat4(
  	vec4(2.0 / frustum.width, 0.0, 0.0, 0.0),
  	vec4(0.0, -2.0 / frustum.height, 0.0, 0.0),
  	vec4(0.0, 0.0, -1.0, 0.0),
  	vec4(-1.0, 1.0, 0.0, 1.0)
  );
  // Projected position.
  gl_Position = trn * vec4(inPosition.xy, 0.0, 1.0);
}
