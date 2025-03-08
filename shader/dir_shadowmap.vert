#version 330 core

#extension GL_GOOGLE_include_directive : enable 
#extension GL_ARB_shading_language_include : enable 

#include "shared_wrapper.glsl.frag"

PASS_DATA_BEGIN
	USING(PASS, DIRLIGHT)
	USING(PASS, TIMING)
PASS_DATA_END

INPUTS_BEGIN
	USING(PASS, DATA)
    USING(ATTR, POSITION)
    USING(INST, TRANSFORM)
    USING(INST, STATIC_SHADOW)
    // Always last
    USING(INST, INSTANCE_ID)
INPUTS_END

// No output parameters

void main() {
    // Instance index. Mandatory first line of main.
    int passInstanceId = READ(INST, INSTANCE_ID);
    Timing tm = READ(PASS, TIMING);
    vec3 inPosition = READ(ATTR, POSITION);
    Transform trn = READ(INST, TRANSFORM);
    StaticShadow ss = READ(INST, STATIC_SHADOW);
    DirLight dl = READ(PASS, DIRLIGHT);
    
    // Interpolate model translation to get the current position
    vec3 prevTrans = trn.prevModel[3].xyz;
    vec3 currTrans = trn.model[3].xyz;
    // TODO: Interpolate rotation too
    vec3 translation = mix(prevTrans, currTrans, tm.interpolation);
    mat4 model = trn.model;
    model[3] = vec4(translation.xyz, 1.0);
    mat4 mvp = dl.cascadeViewProjs[ss.cascadeId] * model;
    // Projected position.
    gl_Position = mvp * vec4(inPosition, 1.0);
}