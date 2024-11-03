#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

PASS_DATA_BEGIN
	USING(PASS, VIEW)
PASS_DATA_END

INPUTS_BEGIN
	USING(PASS, DATA)
    USING(ATTR, POSITION)
    USING(ATTR, NORMAL)
    USING(ATTR, TEXCOORD)
    USING(INST, TRANSFORM)
    USING(INST, MATERIAL)
    USING(INST, TRANSFORM_EXTRA)
    // Always last
    USING(INST, INSTANCE_ID)
INPUTS_END

// Output parameters.
ATTR_LOC(0) out vec2 passTexCoord;
ATTR_LOC(1) out vec3 passNormal;
ATTR_LOC(2) out vec3 passViewPos;
ATTR_LOC(3) out vec3 passProjPos;
ATTR_LOC(4) out vec3 passPrevProjPos;
ATTR_LOC(5) flat out int passInstanceId;

void main() {
    // Instance index. Mandatory first line of main.
    passInstanceId = READ(INST, INSTANCE_ID);
    vec3 inPosition = READ(ATTR, POSITION);
    Transform trn = READ(INST, TRANSFORM);
    View vw = READ(PASS, VIEW);
    mat4 prevModel = READ(INST, TRANSFORM_EXTRA).prevModel;
    mat4 prevMvp = vw.prevViewProj * prevModel;
    mat4 mvp = vw.viewProj * trn.model;
    mat3 mv = mat3(vw.view * trn.model);
    // Texcoords.
    passTexCoord = READ(ATTR, TEXCOORD);
    // Normal in view space.
    passNormal = normalize(mv * READ(ATTR, NORMAL));
    // Position in view space.
    passViewPos = mv * inPosition;
    // Projected position.
    gl_Position = mvp * vec4(inPosition, 1.0);

    passProjPos = gl_Position.xyw;
    passPrevProjPos = (prevMvp * vec4(inPosition, 1.0)).xyw;
}