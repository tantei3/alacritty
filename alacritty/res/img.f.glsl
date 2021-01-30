#version 330 core
in vec2 TexCoords;

out vec4 color;

uniform sampler2D imageTex;

void main() {
    color = vec4(texture(imageTex, TexCoords).rgb, 1.0);
}
