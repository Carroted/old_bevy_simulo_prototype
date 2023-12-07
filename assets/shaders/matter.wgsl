struct MatterMaterial {
    color: vec4<f32>,
    strokeColor: vec4<f32>,
    strokeWidth : f32
};

@group(1) @binding(0) var<uniform> material: MatterMaterial;
@group(1) @binding(1) var color_texture: texture_2d<f32>;
@group(1) @binding(2) var color_sampler: sampler;

fn get_sample(probe: vec2<f32>) -> f32 {
    return textureSample(color_texture, color_sampler, probe).a;
}

[[stage(fragment)]]
fn fragment(in_uv : [[location(0)]] vec2<f32>) -> [[location(0)]] vec4<f32> {
    var uv = in_uv;
    var stroke : f32 = get_sample(uv + vec2<f32>(material.strokeWidth,0.0));
    stroke += get_sample(uv + vec2<f32>(-material.strokeWidth,0.0));
    stroke += get_sample(uv + vec2<f32>(0.0,material.strokeWidth));
    stroke += get_sample(uv + vec2<f32>(0.0,-material.strokeWidth));
    stroke += get_sample(uv + vec2<f32>(material.strokeWidth,-material.strokeWidth));
    stroke += get_sample(uv + vec2<f32>(-material.strokeWidth,material.strokeWidth));
    stroke += get_sample(uv + vec2<f32>(material.strokeWidth,material.strokeWidth));
    stroke += get_sample(uv + vec2<f32>(-material.strokeWidth,-material.strokeWidth));
    stroke = min(stroke, 1.0);
    if (textureSample(color_texture, color_sampler, uv).a > stroke) {
        return material.color;  // return the fill color inside the object
    }
    return material.strokeColor;  // return the stroke color at the boundary
}