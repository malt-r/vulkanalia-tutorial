#version 450

// there is no builtin output variable for fragment shaders
// layout(location = 0) specifies the location in the attached framebuffer,
// to which this variable will be linked
layout(location = 0) out vec4 outColor;

// called for every fragment
void main() {
	outColor = vec4(1.0, 0.0, 0.0, 1.0);
}
