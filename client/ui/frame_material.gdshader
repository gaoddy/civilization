shader_type canvas_item;
render_mode blend_mix;

// 框体材质（方向 A：黄铜浮雕）。只画边框一圈与内侧阴影，中间透出原有内容。
// tint   时代合金色：每个时代换一种铜/钢/陶的基色，形状与光照保持一致。
// golden 三连金框：更亮的金、内圈再压一道金线、外发光更强。
// guide  引导流光：只有需要玩家操作的元素才开启，其余保持静态。
// surface 数字底板：默认只画边框、中间透出内容；打开后内部铺满深铜面，
//         供段位、倒计时与数字这类没有整块底色的信息表面使用。

uniform vec2 box_size = vec2(200.0, 120.0);
uniform float corner = 8.0;
uniform float band = 2.8;
uniform int style = 0;
uniform float hover = 0.0;
uniform vec3 tint = vec3(0.88, 0.70, 0.34);
uniform vec3 golden_tint = vec3(0.94, 0.78, 0.42);
uniform float golden = 0.0;
uniform float guide = 0.0;
uniform float surface = 0.0;

float sd_rbox(vec2 p, vec2 b, float r) {
	vec2 q = abs(p) - b + vec2(r);
	return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r;
}

float hash21(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float vnoise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	float a = hash21(i);
	float b = hash21(i + vec2(1.0, 0.0));
	float c = hash21(i + vec2(0.0, 1.0));
	float d = hash21(i + vec2(1.0, 1.0));
	return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

void fragment() {
	vec2 p = (UV - 0.5) * box_size;
	float d = sd_rbox(p, box_size * 0.5, corner);
	float aa = max(fwidth(d), 0.0001) * 1.2;

	// t: 0 = 外沿，1 = 内沿
	float t = clamp(-d / band, 0.0, 1.0);
	float slope = cos(t * 3.14159265);
	vec2 dir = normalize(p + vec2(0.0001));
	float light = dot(dir, normalize(vec2(-0.42, -0.90)));

	float ring_a = (1.0 - smoothstep(-aa, aa, d)) - (1.0 - smoothstep(-band - aa, -band + aa, d));
	float inner_a = 1.0 - smoothstep(-band - aa, -band + aa, d);

	vec3 alloy = tint;
	vec3 dark = tint * 0.16;
	vec3 ring_col = alloy;
	float glow_a = 0.0;
	float shadow_a = 0.0;
	float grain_a = 0.0;
	vec3 grain_col = vec3(0.0);
	vec3 glow_col = golden_tint;

	if (style == 0) {
		// A 黄铜浮雕：金属色随倒角法线明暗，细颗粒压住塑料感。
		float metal = 0.5 + 0.5 * light * slope;
		float grain = vnoise(UV * box_size * 0.30);
		ring_col = mix(dark, alloy, metal) * (0.90 + 0.20 * grain);

		// 只有引导态才走流光：可购买、可操作、待选择的目标。
		if (guide > 0.5) {
			float sweep = smoothstep(0.76, 1.0, sin((UV.x - UV.y) * 4.2 - TIME * 1.6));
			ring_col += sweep * alloy * 0.55;
		}
		ring_col *= 1.0 + hover * 0.20;

		if (golden > 0.5) {
			// 三连金框：只提亮足金并加强外发光，不加内圈线，保持单层边框。
			ring_col = mix(ring_col, golden_tint * 1.08, 0.55 + 0.25 * metal);
			glow_col = golden_tint;
			glow_a = (1.0 - smoothstep(0.0, 11.0, d)) * step(0.0, d) * 0.46;
		} else {
			glow_a = (1.0 - smoothstep(0.0, 8.0, d)) * step(0.0, d) * 0.30;
		}

		shadow_a = inner_a * (1.0 - smoothstep(0.0, 13.0, -d - band)) * 0.42;
		// 内表面顶部受光，底部压暗，让面板读成有厚度的板。
		float top_light = clamp(0.5 - p.y / box_size.y, 0.0, 1.0);
		grain_a = inner_a * 0.05;
		grain_col = vec3(1.0, 0.86, 0.62) * top_light;
	} else {
		// B 蚀刻铜锈：暗铜底、强颗粒、锈斑，内侧第二道凹槽加重年代感。
		float metal = 0.34 + 0.66 * abs(light) * abs(slope);
		float grain = vnoise(UV * box_size * 0.55);
		float patina = smoothstep(0.60, 0.86, vnoise(UV * box_size * 0.085 + 3.7));
		vec3 brass = mix(dark, alloy * 0.78, metal) * (0.70 + 0.44 * grain);
		brass = mix(brass, vec3(0.22, 0.40, 0.34), patina * 0.55);
		ring_col = brass * (1.0 + hover * 0.12);
		float groove = abs(-d - band * 2.6);
		float groove_a = (1.0 - smoothstep(0.5, 1.7, groove)) * inner_a;
		if (groove_a > 0.01) {
			ring_a = max(ring_a, groove_a * 0.85);
			ring_col = mix(ring_col, dark * 1.6, groove_a);
		}
		if (golden > 0.5) {
			ring_col = mix(ring_col, golden_tint * 1.08, 0.45);
			glow_a = (1.0 - smoothstep(0.0, 10.0, d)) * step(0.0, d) * 0.40;
		} else {
			glow_a = (1.0 - smoothstep(0.0, 7.0, d)) * step(0.0, d) * 0.16;
		}
		shadow_a = inner_a * (1.0 - smoothstep(0.0, 10.0, -d - band)) * 0.54;
		grain_a = inner_a * 0.12;
		grain_col = vec3(0.30, 0.25, 0.16) * (0.55 + 0.9 * grain);
	}

	float total = max(max(max(ring_a, shadow_a), glow_a), grain_a);
	vec3 col = (ring_col * ring_a + glow_col * glow_a + grain_col * grain_a) / max(total, 0.0001);
	if (surface > 0.5) {
		// 数字底板：整块深铜面，左上受光、右下压暗，颗粒只作质感。
		// 边圈、流光与颗粒仍按各自权重叠在底板之上，形状语言与边框材质一致。
		float top_light = clamp(0.55 - p.y / box_size.y * 1.05, 0.0, 1.0);
		float sheen = 0.5 + 0.5 * light;
		float grain = vnoise(UV * box_size * 0.30);
		vec3 base = mix(vec3(0.070, 0.056, 0.038), tint * 0.34, 0.62);
		vec3 plate = base * (0.82 + 0.22 * grain);
		plate += tint * (0.030 + 0.085 * top_light * sheen);
		float edge = clamp(max(max(ring_a, glow_a), grain_a), 0.0, 1.0) * inner_a;
		col = mix(plate, col, edge);
		total = max(total, inner_a * 0.96);
	}
	COLOR = vec4(col, total);
}
