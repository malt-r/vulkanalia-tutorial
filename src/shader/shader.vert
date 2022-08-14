#version 450

vec2 positions[3] = vec2[] (
	vec2(0.0, -0.5),
	vec2(0.5, 0.5),
	vec2(-0.5, 0.5)
);

// invoked on every vertex
// builtin gl_VertexIndex variable contains index of current vertex
void main() {

	// add dummy z and w coordinates
	// gl_Position is the builtin output of this vertex shader
	gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
}
