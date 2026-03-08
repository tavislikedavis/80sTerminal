// Text Rendering Shader
// Renders terminal text using glyph atlas

struct Uniforms {
    resolution: vec2<f32>,
    cell_size: vec2<f32>,
    atlas_size: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var glyph_atlas: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) bg_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) bg_color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Convert pixel coordinates to clip space
    let x = (in.position.x / uniforms.resolution.x) * 2.0 - 1.0;
    let y = 1.0 - (in.position.y / uniforms.resolution.y) * 2.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    out.bg_color = in.bg_color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // If this is a background-only vertex (no UV), draw background
    if in.uv.x == 0.0 && in.uv.y == 0.0 && in.color.a == 0.0 {
        return in.bg_color;
    }

    // Sample glyph from atlas
    let glyph_alpha = textureSample(glyph_atlas, tex_sampler, in.uv).r;

    // Return colored glyph
    return vec4<f32>(in.color.rgb, in.color.a * glyph_alpha);
}
