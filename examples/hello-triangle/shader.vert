#version 450

layout(location = 0) in vec2 a_TexCoord;
layout(location = 0) out vec2 v_TexCoord;

void main() {
    float x = float(gl_VertexIndex - 1);
    float y = float(((gl_VertexIndex & 1) * 2) - 1);
    gl_Position = vec4(x, y, 0.0, 1.0);
    v_TexCoord = a_TexCoord;
}
