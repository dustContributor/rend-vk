#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

INPUTS_BEGIN
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
    Transform trns = READ(INST, TRANSFORM);
    mat4 prevMvp = READ(INST, TRANSFORM_EXTRA).prevMvp;
    mat4 mvp = trns.mvp;
    mat3 mv = mat3(trns.mv);
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