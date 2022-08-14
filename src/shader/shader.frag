#version 450

// there is no builtin output variable for fragment shaders
// layout(location = 0) specifies the location in the attached framebuffer,
// to which this variable will be linked
layout(location = 0) out vec4 outColor;

// does not need to have the same name as output variable in
// vertex shader, will be linked together exclusively by the location parameter
layout(location = 0) in vec3 fragColor;

// called for every fragment
void main() {
	outColor = vec4(fragColor, 1.0);
}
