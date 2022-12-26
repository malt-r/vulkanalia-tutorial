#version 450

// will be updated every frame to make things spin
// similar to the location directive for attributes
layout(binding = 0) uniform UniformBufferObject {
	mat4 model;
	mat4 view;
	mat4 proj;
} ubo;

// these are vertex attributes, they are defined for each vertex
// the location = x notation assigns indices to the inputs, so we can 
// reference them
// 
// some types (dvec3) use multiple slots, this needs to be accounted for 
// in this indexing
layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 fragColor;


// invoked on every vertex
// builtin gl_VertexIndex variable contains index of current vertex
void main() {

	// add dummy z and w coordinates
	// gl_Position is the builtin output of this vertex shader
	gl_Position = ubo.proj * ubo.view * ubo.model * vec4(inPosition, 0.0, 1.0);
	fragColor = inColor;
}
