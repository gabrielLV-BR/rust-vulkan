#!/usr/bin/sh

cd src/resources/shaders/
glslc basic.frag -o frag.spv
glslc basic.vert -o vert.spv