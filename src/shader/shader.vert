#version 450

// will be updated every frame to make things spin
// similar to the location directive for attributes
// 
// it is possible to bind multiple descriptor sets simultaneously;
// for each one a descriptor layout needs to be defined
// in a shader, we could reference different descriptor sets like this
// `layout(set = 0, binding = 0)...``
// -> useful to put descriptors, which vary per object and descriptors, which 
// stay the same for every object in different sets (more efficient, than 
// rebinding all descriptor sets across all draw calls)
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
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inTexCoord;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 fragTexCoord;


// invoked on every vertex
// builtin gl_VertexIndex variable contains index of current vertex
void main() {

	// add dummy z and w coordinates
	// gl_Position is the builtin output of this vertex shader
	gl_Position = ubo.proj * ubo.view * ubo.model * vec4(inPosition, 1.0);
	fragColor = inColor;
	fragTexCoord = inTexCoord;
}
