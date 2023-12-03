#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

INPUTS_BEGIN
    USING(ATTR, POSITION)
    USING(ATTR, NORMAL)
    USING(ATTR, TEXCOORD)
    USING(INST, TRANSFORM)
    USING(INST, INSTANCE_ID)
INPUTS_END

// Output parameters.
ATTR_LOC(0) out vec3 outColor;

void main() {
    // Instance index. Mandatory first line of main.
    int passInstanceId = READ(INST, INSTANCE_ID);
    vec3 inPosition = READ(ATTR, POSITION);
    Transform trns = READ(INST, TRANSFORM);
    // Projected position.
    gl_Position = trns.mvp * vec4(inPosition, 1.0);
    outColor = normalize(abs(gl_Position.xyz));
}