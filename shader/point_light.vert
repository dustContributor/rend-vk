#version 330 core

#ifdef IS_EXTERNAL_COMPILER
#extension GL_GOOGLE_include_directive : require 
#else
#extension GL_ARB_shading_language_include : require
#endif

#include "shared_wrapper.glsl.frag"

INPUTS_BEGIN
	UNUSED_INPUT(0) // pass data
  USING(ATTR, POSITION)
  USING(ATTR, NORMAL)
  USING(ATTR, TEXCOORD)
  USING(INST, TRANSFORM)
	USING(INST, POINTLIGHT)
  // Always last
  USING(INST, INSTANCE_ID)
INPUTS_END

// Output parameters.
ATTR_LOC(0) flat out int passInstanceId;
ATTR_LOC(1) out vec3 passViewPosCenter;
ATTR_LOC(2) flat out float passInvRadius;
ATTR_LOC(3) flat out vec3 passColor;

void main() {
  // Instance index. Mandatory first line of main.
  passInstanceId = READ(INST, INSTANCE_ID);
	PointLight pointLight = READ(INST, POINTLIGHT);
  vec3 inPosition = READ(ATTR, POSITION);
  Transform trns = READ(INST, TRANSFORM);
  mat4 mvp = trns.mvp;
  mat3 mv = mat3(trns.mv); 
  // Inverse radius used for each fragment
  passInvRadius = 1.0 / pointLight.radius;
  // Pass color so fragment shader doesn't has toread the point light data
  passColor = pointLight.color;
	// Last column is the translation.
  passViewPosCenter = trns.mv[3].xyz;
		// Matrix contains scaling and positioning.
  gl_Position = mvp * vec4(inPosition * pointLight.radius * 2.0, 1.0);
}