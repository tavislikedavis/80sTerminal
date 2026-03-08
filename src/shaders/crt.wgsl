// CRT Post-Processing Shader
// Per-CRT style bezel rendering with configurable colors

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    curvature: f32,
    scanline_intensity: f32,
    scanline_count: f32,
    bloom_intensity: f32,
    bloom_radius: f32,
    chromatic_aberration: f32,
    vignette_intensity: f32,
    flicker_intensity: f32,
    noise: f32,
    phosphor_persistence: f32,
    bezel_size: f32,
    screen_brightness: f32,
    tab_bar_offset: f32,
    bezel_color: vec3<f32>,
    opacity: f32,
    bezel_corner_radius: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var input_texture: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn barrel_distort(uv: vec2<f32>, curvature: f32) -> vec2<f32> {
    let centered = uv * 2.0 - 1.0;
    let offset = centered.yx * centered.yx * centered.xy * curvature;
    let distorted = centered + offset;
    return distorted * 0.5 + 0.5;
}

fn is_valid_uv(uv: vec2<f32>) -> bool {
    return uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
}

fn scanline(uv: vec2<f32>, intensity: f32, count: f32) -> f32 {
    let scanline_pos = uv.y * count;
    let line = pow(sin(scanline_pos * 3.14159265), 2.0);
    let thin_line = pow(sin(scanline_pos * 3.14159265 * 2.0), 8.0) * 0.15;
    return 1.0 - intensity * (1.0 - line) + thin_line * intensity;
}

fn vignette(uv: vec2<f32>, intensity: f32) -> f32 {
    let centered = uv * 2.0 - 1.0;
    let dist = length(centered);
    return 1.0 - intensity * dist * dist;
}

fn bloom(uv: vec2<f32>, radius: f32) -> vec3<f32> {
    let pixel_size = 1.0 / uniforms.resolution;
    var bloom_color = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let offset = vec2<f32>(f32(i), f32(j)) * pixel_size * radius;
            let sample_uv = uv + offset;
            if is_valid_uv(sample_uv) {
                let weight = 1.0 / (1.0 + length(vec2<f32>(f32(i), f32(j))));
                bloom_color += textureSample(input_texture, tex_sampler, sample_uv).rgb * weight;
                weight_sum += weight;
            }
        }
    }
    return bloom_color / weight_sum;
}

fn rounded_box_sdf(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Bezel rendering with configurable colors per CRT style
fn render_bezel(uv: vec2<f32>, bezel_size: f32) -> vec4<f32> {
    let centered = uv * 2.0 - 1.0;
    let scale = 0.4;

    // Screen area (inside the inner bezel)
    let screen_size = vec2<f32>(1.0 - bezel_size * scale * 2.0, 1.0 - bezel_size * scale * 2.0);
    let screen_radius = 0.008;
    let screen_dist = rounded_box_sdf(centered, screen_size, screen_radius);

    // Inner bezel area - recessed frame
    let inner_bezel_size = vec2<f32>(1.0 - bezel_size * scale * 1.2, 1.0 - bezel_size * scale * 1.2);
    let inner_bezel_radius = uniforms.bezel_corner_radius * 0.8;
    let inner_bezel_dist = rounded_box_sdf(centered, inner_bezel_size, inner_bezel_radius);

    // Outer monitor case
    let outer_size = vec2<f32>(1.0, 1.0);
    let outer_radius = uniforms.bezel_corner_radius;
    let outer_dist = rounded_box_sdf(centered, outer_size, outer_radius);

    // Configurable base color from the CRT style
    let base_color = uniforms.bezel_color;
    // Derive dark and light variants from the base
    let dark_color = base_color * 0.75;
    let light_color = min(base_color * 1.15 + vec3<f32>(0.05), vec3<f32>(1.0));

    var bezel_color = base_color;

    // Lighting from top-left
    let light_dir = normalize(vec2<f32>(-0.5, 0.7));
    let surface_normal = normalize(centered);
    let light_intensity = dot(surface_normal, light_dir) * 0.15 + 0.85;

    // Inner bezel area (recessed screen surround)
    if inner_bezel_dist < 0.0 && screen_dist > 0.0 {
        bezel_color = dark_color;

        let depth = smoothstep(0.0, 0.025, -inner_bezel_dist);

        // Top edge highlight
        let top_highlight = smoothstep(-0.3, 0.8, centered.y) * depth;
        bezel_color = mix(bezel_color, light_color, top_highlight * 0.4);

        // Left edge highlight
        let left_highlight = smoothstep(0.3, -0.5, centered.x) * depth * 0.5;
        bezel_color = mix(bezel_color, light_color, left_highlight * 0.3);

        // Bottom/right shadow
        let shadow = smoothstep(0.5, -0.3, centered.y) * smoothstep(-0.3, 0.5, centered.x);
        bezel_color *= 1.0 - shadow * 0.12 * depth;

        // Inner shadow where screen meets bezel
        let recess_shadow = smoothstep(0.02, 0.0, screen_dist);
        bezel_color *= 1.0 - recess_shadow * 0.35;

        // Subtle plastic texture
        let plastic_noise = rand(uv * 800.0) * 0.015;
        bezel_color += vec3<f32>(plastic_noise - 0.0075);
    }
    // Outer bezel area
    else if screen_dist > 0.0 {
        bezel_color = base_color;

        // Vertical gradient - lighter at top
        let grad = smoothstep(-1.0, 1.0, centered.y) * 0.08;
        bezel_color = mix(bezel_color, light_color, grad);

        // Top edge light catch
        let top_reflection = smoothstep(0.7, 0.95, centered.y) * smoothstep(0.0, 0.015, -outer_dist);
        bezel_color = mix(bezel_color, light_color, top_reflection * 0.4);

        // Subtle surface texture
        let surface_var = rand(uv * 600.0) * 0.015;
        bezel_color += vec3<f32>(surface_var - 0.0075);

        // Ambient occlusion near inner bezel
        let ao = smoothstep(0.03, 0.0, inner_bezel_dist);
        bezel_color *= 1.0 - ao * 0.15;

        // Edge definition between outer and inner bezel
        let edge = smoothstep(0.0, 0.006, inner_bezel_dist) * smoothstep(0.015, 0.006, inner_bezel_dist);
        bezel_color -= vec3<f32>(0.04) * edge;
    }

    // Apply overall lighting
    bezel_color *= light_intensity;

    // Slight vignette on bezel for depth
    let bezel_vignette = 1.0 - length(centered) * 0.08;
    bezel_color *= bezel_vignette;

    if screen_dist > 0.0 {
        return vec4<f32>(bezel_color, 1.0);
    }

    // Negative alpha = screen area
    return vec4<f32>(0.0, 0.0, 0.0, -1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;
    let alpha = uniforms.opacity;

    // Reserve space for the tab bar at the top
    let tab_offset = uniforms.tab_bar_offset;
    if uv.y < tab_offset {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Remap UV so the CRT/bezel occupies the area below the tab bar
    uv.y = (uv.y - tab_offset) / (1.0 - tab_offset);

    // Check if we're in the bezel area
    if uniforms.bezel_size > 0.0 {
        let bezel_result = render_bezel(uv, uniforms.bezel_size);

        if bezel_result.a > 0.0 {
            // Bezel always fully opaque
            return vec4<f32>(bezel_result.rgb, 1.0);
        }

        // Transform UV to screen-space
        let scale = 0.4;
        let screen_min = uniforms.bezel_size * scale;
        let screen_max = 1.0 - uniforms.bezel_size * scale;
        uv = (uv - vec2<f32>(screen_min)) / (screen_max - screen_min);
    }

    // Apply barrel distortion
    if uniforms.curvature > 0.0 {
        uv = barrel_distort(uv, uniforms.curvature);
    }

    // Check bounds after distortion
    if !is_valid_uv(uv) {
        // Dark screen edge inside the bezel stays opaque; outside respects transparency
        if uniforms.bezel_size > 0.0 {
            return vec4<f32>(0.01, 0.01, 0.01, 1.0);
        }
        return vec4<f32>(0.0, 0.0, 0.0, alpha);
    }

    // Chromatic aberration
    var color: vec3<f32>;
    if uniforms.chromatic_aberration > 0.0 {
        let ca = uniforms.chromatic_aberration;
        let center = vec2<f32>(0.5);
        let dir = normalize(uv - center);
        let r_uv = uv + dir * ca;
        let g_uv = uv;
        let b_uv = uv - dir * ca;
        color.r = textureSample(input_texture, tex_sampler, r_uv).r;
        color.g = textureSample(input_texture, tex_sampler, g_uv).g;
        color.b = textureSample(input_texture, tex_sampler, b_uv).b;
    } else {
        color = textureSample(input_texture, tex_sampler, uv).rgb;
    }

    // Screen brightness
    color *= uniforms.screen_brightness;

    // Bloom
    if uniforms.bloom_intensity > 0.0 {
        let bloom_color = bloom(uv, uniforms.bloom_radius);
        color = mix(color, bloom_color, uniforms.bloom_intensity * 0.3);
        let brightness = dot(color, vec3<f32>(0.299, 0.587, 0.114));
        if brightness > 0.5 {
            color += bloom_color * uniforms.bloom_intensity * (brightness - 0.5) * 2.0;
        }
    }

    // Scanlines
    if uniforms.scanline_intensity > 0.0 {
        let sl = scanline(uv, uniforms.scanline_intensity, uniforms.scanline_count);
        color *= sl;
    }

    // Vignette
    if uniforms.vignette_intensity > 0.0 {
        let vig = vignette(uv, uniforms.vignette_intensity);
        color *= vig;
    }

    // Flicker
    if uniforms.flicker_intensity > 0.0 {
        let flicker = 1.0 - uniforms.flicker_intensity * 0.5 * (sin(uniforms.time * 60.0) * 0.5 + 0.5);
        color *= flicker;
    }

    // Noise
    if uniforms.noise > 0.0 {
        let noise_val = rand(uv * uniforms.resolution + vec2<f32>(uniforms.time * 100.0));
        color += vec3<f32>((noise_val - 0.5) * uniforms.noise);
    }

    // RGB phosphor pattern
    let phosphor_x = fract(uv.x * uniforms.resolution.x / 3.0);
    var phosphor_mask: vec3<f32>;
    if phosphor_x < 0.333 {
        phosphor_mask = vec3<f32>(1.0, 0.85, 0.85);
    } else if phosphor_x < 0.666 {
        phosphor_mask = vec3<f32>(0.85, 1.0, 0.85);
    } else {
        phosphor_mask = vec3<f32>(0.85, 0.85, 1.0);
    }
    color *= mix(vec3<f32>(1.0), phosphor_mask, 0.12);

    // Subtle screen glare
    let glare_pos = uv - vec2<f32>(0.15, 0.1);
    let glare = max(0.0, 1.0 - length(glare_pos) * 4.0) * 0.02;
    color += vec3<f32>(glare);

    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    // Apply transparency to screen content only (straight alpha for PostMultiplied)
    return vec4<f32>(color, alpha);
}
