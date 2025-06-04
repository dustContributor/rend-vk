#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

PASS_DATA_BEGIN
	USING(PASS, VIEW)
	USING(PASS, VIEWRAY)
	USING(PASS, FRUSTUM)
PASS_DATA_END

INPUTS_BEGIN
	USING(PASS, DATA)
	USING(ATTR, POSITION)
	UNUSED_INPUT(1)
	UNUSED_INPUT(2)
	USING(INST, SPOTLIGHT)
	// Always last
	USING(INST, INSTANCE_ID)
INPUTS_END

// Output parameters.
ATTR_LOC(0) flat out int passInstanceId;
ATTR_LOC(1) flat out vec3 passViewOrigin;
ATTR_LOC(2) flat out vec3 passViewDir;
ATTR_LOC(3) flat out float passInvRange;
ATTR_LOC(4) flat out vec3 passLightColor;
ATTR_LOC(5) flat out float passCutoffCos;

void main() {
	// Instance index. Mandatory first line of main.
	passInstanceId = READ(INST, INSTANCE_ID);
	View vw = READ(PASS, VIEW);
	SpotLight spotLight = READ(INST, SPOTLIGHT);
	vec3 inPosition = READ(ATTR, POSITION);
	vec3 up = spotLight.direction;
	vec3 side = vec3(1, 0, 0);
	// Ensure the side vector is orthogonal to the up vector
	side = normalize(side - (dot(side, up) * up));
	vec3 forward = normalize(cross(up, side));
	vec4 position = vec4(spotLight.position, 1.0);

	// Basis matrix
	mat4 model = mat4(mat3(side, -up, forward) * mat3(spotLight.range));
	model[3] = position;

	vec3 origin = spotLight.position - (spotLight.direction * spotLight.range / 2.0);

	mat4 mvp = vw.viewProj * model;
	passInvRange = 1.0 / spotLight.range;
	passCutoffCos = cos(spotLight.cutoffRad);
	// Pass color so fragment shader doesn't has to read from buffer data again
	passLightColor = spotLight.color;
	passViewOrigin = (vw.view * vec4(origin, 1.0)).xyz;
	passViewDir = (vw.view * vec4(spotLight.direction, 0.0)).xyz;
	gl_Position = mvp * vec4(inPosition, 1.0);
}