#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

PASS_DATA_BEGIN
	USING(PASS, VIEWRAY)
	USING(PASS, FRUSTUM)
	USING(PASS, VIEW)
PASS_DATA_END

INPUTS_BEGIN
	USING(PASS, DATA)
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
ATTR_LOC(1) flat out vec3 passViewPosCenter;
ATTR_LOC(2) flat out float passInvRadius;
ATTR_LOC(3) flat out vec3 passLightColor;

void main() {
  // Instance index. Mandatory first line of main.
  passInstanceId = READ(INST, INSTANCE_ID);
	PointLight pointLight = READ(INST, POINTLIGHT);
  vec3 inPosition = READ(ATTR, POSITION);
  Transform trn = READ(INST, TRANSFORM);
  View vw = READ(PASS, VIEW);
  mat4 mvp = vw.viewProj * trn.model;
  mat4 mv = vw.view * trn.model;
  // Inverse radius used for each fragment
  passInvRadius = 1.0 / pointLight.radius;
  // Pass color so fragment shader doesn't has toread the point light data
  passLightColor = pointLight.color;
	// Last column is the translation.
  passViewPosCenter = mv[3].xyz;
		// Matrix contains scaling and positioning.
  gl_Position = mvp * vec4(inPosition * pointLight.radius * 2.0, 1.0);
}