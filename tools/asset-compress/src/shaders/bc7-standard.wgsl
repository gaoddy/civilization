// BC7 (BPTC) compute shader encoder — modes 1 and 6 (mode 4 code is retained
// but disabled; see the `use4 = false` note below).
//
// Ported from the phase-0 prototype `output/gpu-texture-phase0/bc7/bc7_mode1.wgsl`
// (see src/shaders/THIRD_PARTY.md). MIT.
//
// One invocation per 4×4 block. Emits 16 bytes = 4 u32s into the storage
// buffer at `dst[block_index * 4 .. + 3]`. Works in the 8-bit integer domain
// (texels rounded to 0..255). Two candidates are fitted independently and the
// one with the smaller reconstruction error wins: mode 6 (one RGBA line, 4-bit
// indices) and mode 1 (two subsets, 3-bit indices, opaque blocks only).
//
// ALGORITHM: per block, the covariance picks mode 6 (one RGBA line, 16
// levels) or mode 4 (one channel split off into its own scalar plane, the
// other three on a line; 2-bit + 3-bit index sets) by comparing the
// variance each explains net of quantisation; the principal axis comes
// from a power iteration seeded with the largest-variance column. Both
// modes then share ONE extents pass whose stored projections become the
// indices, one endpoint quantiser (7-bit + p / 5-bit) and an anchor rule
// applied before indexing. Gray + opaque blocks take an integer 1-D tail:
// lossless for spans ≤ 15 with odd endpoints (alpha exactly 255),
// alpha-aware scalar LSQ refit above.
//
// MODE 6 LAYOUT (LSB-first, bit 0 = byte 0's bit 0)
//   bits 0..6    mode field      (0b0000001 — only bit 6 is 1)
//   bits 7..13   R0 (7-bit)   bits 14..20 R1   bits 21..27 G0   bits 28..34 G1
//   bits 35..41  B0   bits 42..48 B1   bits 49..55 A0   bits 56..62 A1
//   bit  63      P0   bit 64 P1
//   bits 65..67  pixel 0 index (3 bits; anchor, MSB implicit 0)
//   bits 68..71  pixel 1 index (4 bits) ... bits 124..127 pixel 15 index
//
// Effective 8-bit endpoint channel = (7_bit_value << 1) | p_bit.
// Palette[i] = ((64 − W4[i]) × e0_8 + W4[i] × e1_8 + 32) >> 6, integer.
//
// MODE 4 LAYOUT: see bc7_fast_f16.wgsl (decode reference in bc7_ref.ts).
//
// The block is assembled with straight-line constant shifts — a generic
// write_bits() helper's dynamic word indexing keeps the output array out of
// registers.

struct Params {
  blocks_x: u32,
  blocks_y: u32,
  width:    u32,
  height:   u32,
  y0:       u32, // first block row of this dispatch (row-band encodes)
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

// Float -> u32 for exact integers in [0, 2^23): x + 2^23 holds x in its
// mantissa, so bitcast(x + MAGIC) ^ MAGIC_BITS == x (or masked, for small
// fields). WGSL's u32(float) is a SATURATING conversion (compares + selects
// around the convert): the gray path's per-texel ones cost ~5% on gray maps.
// (The colour path's endpoint conversions measured faster left as u32().)
const MAGIC = 8388608.0;
const MAGIC_BITS = 0x4B000000u;
fn fbits(x: f32, mask: u32) -> u32 {
  return bitcast<u32>(x + MAGIC) & mask;
}
fn fu(x: f32) -> u32 {
  return bitcast<u32>(x + MAGIC) ^ MAGIC_BITS;
}

// Mode-4 endpoint-precision charge (8-bit² covariance units; see the f16
// module's header — 0.15 there).
const N4: f32 = 38.1;

// Nibble-slot compaction for the mode-4 index fields (see bc7_fast_f16.wgsl).
fn compact2(x: u32) -> u32 {
  var y = (x | (x >> 2u)) & 0x0F0F0F0Fu;
  y = (y | (y >> 4u)) & 0x00FF00FFu;
  return (y | (y >> 8u)) & 0x0000FFFFu;
}
fn compact3(x: u32) -> u32 {
  var y = (x & 0x07070707u) | ((x >> 1u) & 0x38383838u);
  y = (y & 0x003F003Fu) | ((y >> 2u) & 0x0FC00FC0u);
  return (y & 0x00000FFFu) | ((y >> 4u) & 0x00FFF000u);
}


// ------------------------- 模式 1（原型：2 子集） ------------------------- //
const W3F: array<f32, 8> = array<f32, 8>(0.0, 9.0, 18.0, 27.0, 37.0, 46.0, 55.0, 64.0);
const PART2: array<u32, 64> = array<u32, 64>(
  0xCCCCu, 0x8888u, 0xEEEEu, 0xECC8u,
  0xC880u, 0xFEECu, 0xFEC8u, 0xEC80u,
  0xC800u, 0xFFECu, 0xFE80u, 0xE800u,
  0xFFE8u, 0xFF00u, 0xFFF0u, 0xF000u,
  0xF710u, 0x008Eu, 0x7100u, 0x08CEu,
  0x008Cu, 0x7310u, 0x3100u, 0x8CCEu,
  0x088Cu, 0x3110u, 0x6666u, 0x366Cu,
  0x17E8u, 0x0FF0u, 0x718Eu, 0x399Cu,
  0xAAAAu, 0xF0F0u, 0x5A5Au, 0x33CCu,
  0x3C3Cu, 0x55AAu, 0x9696u, 0xA55Au,
  0x73CEu, 0x13C8u, 0x324Cu, 0x3BDCu,
  0x6996u, 0xC33Cu, 0x9966u, 0x0660u,
  0x0272u, 0x04E4u, 0x4E40u, 0x2720u,
  0xC936u, 0x936Cu, 0x39C6u, 0x639Cu,
  0x9336u, 0x9CC6u, 0x817Eu, 0xE718u,
  0xCCF0u, 0x0FCCu, 0x7744u, 0xEE22u,
);
const ANCH1: array<u32, 64> = array<u32, 64>(
  15u, 15u, 15u, 15u, 15u, 15u, 15u, 15u,
  15u, 15u, 15u, 15u, 15u, 15u, 15u, 15u,
  15u, 2u, 8u, 2u, 2u, 8u, 8u, 15u,
  2u, 8u, 2u, 2u, 8u, 8u, 2u, 2u,
  15u, 15u, 6u, 8u, 2u, 8u, 15u, 15u,
  2u, 8u, 2u, 2u, 2u, 15u, 15u, 6u,
  6u, 2u, 6u, 8u, 15u, 15u, 2u, 2u,
  15u, 15u, 15u, 15u, 15u, 2u, 2u, 15u,
);

// mode 1 端点：6-bit 码 + 共享 p-bit，解码时 7-bit 值 v 展开为 8-bit (v<<1)|(v>>6)
fn expand7(v: vec3<f32>) -> vec3<f32> {
  return floor(v * 2.0) + floor(v / 64.0);
}

// 把 n 位字段 v 写进 4×u32 的 pos 处（小端）
fn put(buf: ptr<function, array<u32, 4>>, pos: u32, n: u32, v: u32) {
  var p = pos;
  var rem = n;
  var val = v;
  loop {
    if (rem == 0u) { break; }
    let w = p >> 5u;
    let off = p & 31u;
    let take = min(rem, 32u - off);
    let m = (1u << take) - 1u;
    (*buf)[w] = (*buf)[w] | ((val & m) << off);
    val = val >> take;
    p = p + take;
    rem = rem - take;
  }
}

// 单子集拟合：协方差主轴（幂迭代 2 次）→ 端点 → 6-bit 码 + 共享 p-bit → 3-bit 索引
// 返回 RGB SSE；q0o/q1o 为 6-bit 码，po 为 p-bit，e0o/e1o 为展开后的 8-bit 端点。
fn fit_subset(px: ptr<function, array<vec3<f32>, 16>>, mask: u32, sel: u32,
              q0o: ptr<function, vec3<u32>>, q1o: ptr<function, vec3<u32>>, po: ptr<function, u32>,
              e0o: ptr<function, vec3<f32>>, e1o: ptr<function, vec3<f32>>,
              idxo: ptr<function, array<u32, 16>>) -> f32 {
  var n = 0.0;
  var mean = vec3<f32>(0.0);
  for (var k: u32 = 0u; k < 16u; k = k + 1u) {
    if (((mask >> k) & 1u) == sel) { mean = mean + (*px)[k]; n = n + 1.0; }
  }
  mean = mean / max(n, 1.0);
  var cov = mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
  for (var k: u32 = 0u; k < 16u; k = k + 1u) {
    if (((mask >> k) & 1u) == sel) {
      let d = (*px)[k] - mean;
      cov = cov + mat3x3<f32>(d * d.x, d * d.y, d * d.z);
    }
  }
  var axis = vec3<f32>(cov[0][0], cov[1][1], cov[2][2]);
  if (dot(axis, axis) < 1e-6) { axis = vec3<f32>(1.0); }
  for (var it: u32 = 0u; it < 2u; it = it + 1u) {
    let nx = cov * axis;
    let nn = dot(nx, nx);
    axis = select(axis, nx * inverseSqrt(max(nn, 1e-12)), nn > 1e-12);
  }
  var tmin = 1e30;
  var tmax = -1e30;
  for (var k: u32 = 0u; k < 16u; k = k + 1u) {
    if (((mask >> k) & 1u) == sel) {
      let t = dot((*px)[k] - mean, axis);
      tmin = min(tmin, t);
      tmax = max(tmax, t);
    }
  }
  let f0 = clamp(mean + axis * tmin, vec3<f32>(0.0), vec3<f32>(255.0));
  let f1 = clamp(mean + axis * tmax, vec3<f32>(0.0), vec3<f32>(255.0));

  var best = 1e30;
  var bq0 = vec3<u32>(0u);
  var bq1 = vec3<u32>(0u);
  var bp = 0u;
  var bv0 = vec3<f32>(0.0);
  var bv1 = vec3<f32>(0.0);
  for (var pi2: u32 = 0u; pi2 < 2u; pi2 = pi2 + 1u) {
    let pf = f32(pi2);
    // 6-bit 码：v7 = 2q + p，8-bit 值 ≈ 2·v7 ⇒ q ≈ 值 / 4
    let g0 = clamp(floor((f0 - vec3<f32>(pf)) * 0.25 + 0.5), vec3<f32>(0.0), vec3<f32>(63.0));
    let g1 = clamp(floor((f1 - vec3<f32>(pf)) * 0.25 + 0.5), vec3<f32>(0.0), vec3<f32>(63.0));
    let v0 = expand7(g0 * 2.0 + vec3<f32>(pf));
    let v1 = expand7(g1 * 2.0 + vec3<f32>(pf));
    let c = dot(v0 - f0, v0 - f0) + dot(v1 - f1, v1 - f1);
    if (c < best) {
      best = c;
      bq0 = vec3<u32>(g0);
      bq1 = vec3<u32>(g1);
      bp = pi2;
      bv0 = v0;
      bv1 = v1;
    }
  }

  let dv = bv1 - bv0;
  let dd = max(dot(dv, dv), 1e-6);
  var sse = 0.0;
  for (var k: u32 = 0u; k < 16u; k = k + 1u) {
    if (((mask >> k) & 1u) == sel) {
      let t = clamp(dot((*px)[k] - bv0, dv) / dd, 0.0, 1.0) * 64.0;
      var bi = 0u;
      var bd = 1e9;
      for (var j: u32 = 0u; j < 8u; j = j + 1u) {
        let e = abs(W3F[j] - t);
        if (e < bd) { bd = e; bi = j; }
      }
      let rec = (bv0 * (64.0 - W3F[bi]) + bv1 * W3F[bi]) / 64.0;
      let e = rec - (*px)[k];
      sse = sse + dot(e, e);
      (*idxo)[k] = bi;
    }
  }
  *q0o = bq0;
  *q1o = bq1;
  *po = bp;
  *e0o = bv0;
  *e1o = bv1;
  return sse;
}

// ------------------------------- Entry --------------------------------- //

@compute @workgroup_size(8, 8, 1)
fn encode(@builtin(global_invocation_id) gid_raw: vec3<u32>) {
  // Row-band encodes dispatch a slice of the block grid starting at row y0.
  let gid = vec3<u32>(gid_raw.x, gid_raw.y + params.y0, gid_raw.z);
  if (gid.x >= params.blocks_x || gid.y >= params.blocks_y) {
    return;
  }
  let block_index = gid.y * params.blocks_x + gid.x;
  let base = vec2<i32>(i32(gid.x) * 4, i32(gid.y) * 4);
  let mx = vec2<i32>(i32(params.width) - 1, i32(params.height) - 1);

  // Load 16 RGBA pixels (8-bit integer domain), the per-channel bbox and
  // the gray test.
  var pixels: array<vec4<f32>, 16>;
  var lo = vec4<f32>(255.0);
  var hi = vec4<f32>(0.0);
  var gd = 0.0;
  let xs = min(vec4<i32>(base.x) + vec4<i32>(0, 1, 2, 3), vec4<i32>(mx.x));
  let ys = min(vec4<i32>(base.y) + vec4<i32>(0, 1, 2, 3), vec4<i32>(mx.y));
  for (var i: u32 = 0u; i < 16u; i = i + 1u) {
    let px = clamp(floor(textureLoad(src_tex, vec2<i32>(xs[i & 3u], ys[i >> 2u]), 0) * 255.0 + 0.5), vec4<f32>(0.0), vec4<f32>(255.0));
    pixels[i] = px;
    lo = min(lo, px);
    hi = max(hi, px);
    gd = max(gd, max(abs(px.x - px.y), abs(px.x - px.z)));
  }

  // Gray + opaque blocks: 1-D, own tail (see bc7_fast_f16.wgsl) — lossless
  // for spans ≤ 15 with odd endpoints (alpha exactly 255), closed-form
  // scalar LSQ refit with alpha-aware p-bit pricing above that.
  if (lo.w == 255.0 && gd == 0.0) {
    let vmin = lo.x;
    let vmax = hi.x;
    var e0 = vmin;
    var e1 = vmax;
    if (vmax - vmin <= 15.0) {
      if (fract(e0 * 0.5) == 0.0 && e0 > 0.0 && e1 - e0 < 15.0) { e0 = e0 - 1.0; }
      if (fract(e1 * 0.5) == 0.0 && e1 < 255.0 && e1 - e0 < 15.0) { e1 = e1 + 1.0; }
    } else {
      let k1 = 15.0 / (vmax - vmin);
      let k0 = 0.5 - vmin * k1;
      var sL = 0.0;
      var sLL = 0.0;
      var sv = 0.0;
      var sLv = 0.0;
      for (var k: u32 = 0u; k < 16u; k = k + 1u) {
        let v = pixels[k].x;
        let L = floor(v * k1 + k0);
        sL = sL + L;
        sLL = sLL + L * L;
        sv = sv + v;
        sLv = sLv + L * v;
      }
      let C = sLL * (1.0 / 225.0);
      let B = sL * (1.0 / 15.0) - C;
      let A = 16.0 - sL * (2.0 / 15.0) + C;
      let Y = sLv * (1.0 / 15.0);
      let X = sv - Y;
      let det = A * C - B * B;
      if (det > 1e-3) {
        let s0 = clamp((C * X - B * Y) / det, 0.0, 255.0);
        let s1 = clamp((A * Y - B * X) / det, 0.0, 255.0);
        // price(e0, e1, p0, p1) up to the block constant: RGB ×3 + alpha.
        let ps0 = vmin - 2.0 * floor(vmin * 0.5);
        let ps1 = vmax - 2.0 * floor(vmax * 0.5);
        var best = 3.0 * (A * vmin * vmin + 2.0 * B * vmin * vmax + C * vmax * vmax - 2.0 * (X * vmin + Y * vmax))
          + A * (1.0 - ps0) + 2.0 * B * (1.0 - ps0) * (1.0 - ps1) + C * (1.0 - ps1);
        for (var pc: u32 = 0u; pc < 4u; pc = pc + 1u) {
          let p0 = f32(pc & 1u);
          let p1 = f32(pc >> 1u);
          let c0 = 2.0 * clamp(floor((s0 - p0) * 0.5 + 0.5), 0.0, 127.0) + p0;
          let c1 = 2.0 * clamp(floor((s1 - p1) * 0.5 + 0.5), 0.0, 127.0) + p1;
          let pr = 3.0 * (A * c0 * c0 + 2.0 * B * c0 * c1 + C * c1 * c1 - 2.0 * (X * c0 + Y * c1))
            + A * (1.0 - p0) + 2.0 * B * (1.0 - p0) * (1.0 - p1) + C * (1.0 - p1);
          if (pr < best) {
            best = pr;
            e0 = c0;
            e1 = c1;
          }
        }
      }
    }
    var glo = 0u;
    var ghi = 0u;
    if (e1 != e0) {
      let k1 = 15.0 / (e1 - e0);
      let k0 = 0.5 - e0 * k1;
      for (var k: u32 = 0u; k < 8u; k = k + 1u) {
        let sg = clamp(floor(pixels[k].x * k1 + k0), 0.0, 15.0);
        glo = glo | (fbits(sg, 15u) << (k * 4u));
      }
      for (var k: u32 = 8u; k < 16u; k = k + 1u) {
        let sg = clamp(floor(pixels[k].x * k1 + k0), 0.0, 15.0);
        ghi = ghi | (fbits(sg, 15u) << ((k - 8u) * 4u));
      }
    }
    var u0 = fu(e0);
    var u1 = fu(e1);
    if ((glo & 0x8u) != 0u) {
      let t = u0; u0 = u1; u1 = t;
      glo = ~glo; ghi = ~ghi;
    }
    let q0 = u0 >> 1u;
    let q1 = u1 >> 1u;
    let og = block_index * 4u;
    dst[og] = 0x40u | (q0 << 7u) | (q1 << 14u) | (q0 << 21u) | (q1 << 28u);
    dst[og + 1u] = (q1 >> 4u) | (q0 << 3u) | (q1 << 10u) | (127u << 17u) | (127u << 24u) | ((u0 & 1u) << 31u);
    dst[og + 2u] = (u1 & 1u) | ((glo & 0x7u) << 1u) | (glo & 0xFFFFFFF0u);
    dst[og + 3u] = ghi;
    return;
  }

  // Covariance (10 symmetric products) over d = px − p0, stored back into
  // pixels: every later pass works block-relative.
  let p0v = pixels[0];
  pixels[0] = vec4<f32>(0.0);
  var sd = vec4<f32>(0.0);
  var cx = vec4<f32>(0.0);
  var cy = vec3<f32>(0.0);
  var cz = vec2<f32>(0.0);
  var cw = 0.0;
  for (var i: u32 = 1u; i < 16u; i = i + 1u) {
    let d = pixels[i] - p0v;
    pixels[i] = d;
    sd = sd + d;
    cx = cx + d.x * d;
    cy = cy + d.y * d.yzw;
    cz = cz + d.z * d.zw;
    cw = cw + d.w * d.w;
  }
  let md = sd * (1.0 / 16.0);
  let sd4 = sd * 0.25;
  cx = cx - sd4.x * sd4;
  cy = cy - sd4.y * sd4.yzw;
  cz = cz - sd4.z * sd4.zw;
  cw = cw - sd4.w * sd4.w;
  let diag = vec4<f32>(cx.x, cy.x, cz.x, cw);
  let trace = diag.x + diag.y + diag.z + diag.w;

  // Mode decision + fit axis (see bc7_fast_f16.wgsl). Flat blocks keep
  // axis 0: every index 0, both endpoints at the mean.
  var use4 = false;
  var idx1 = false;
  var ch = 0u;
  var cmask = vec4<f32>(1.0);
  var axisF = vec4<f32>(0.0);
  if (trace > 0.254) {
    let s = 1.0 / trace;
    let m0 = cx * s;
    let m1 = vec4<f32>(cx.y, cy) * s;
    let m2 = vec4<f32>(cx.z, cy.y, cz) * s;
    let m3 = vec4<f32>(cx.w, cy.z, cz.y, cw) * s;
    var axis = m0;
    var dm = diag.x;
    if (diag.y > dm) { axis = m1; dm = diag.y; }
    if (diag.z > dm) { axis = m2; dm = diag.z; }
    if (diag.w > dm) { axis = m3; }
    axis = axis * inverseSqrt(dot(axis, axis));
    axis = vec4<f32>(dot(m0, axis), dot(m1, axis), dot(m2, axis), dot(m3, axis));
    axis = vec4<f32>(dot(m0, axis), dot(m1, axis), dot(m2, axis), dot(m3, axis));
    let a4 = max(dot(axis, axis), 1e-12);
    axis = axis * inverseSqrt(a4);
    let lamn = sqrt(sqrt(a4));
    let lam = lamn * trace;

    // λ3 for all four scalar-channel candidates at once (lane c).
    let a2 = axis * axis;
    var nn = vec4<f32>(0.0);
    { let wr = lamn * axis.x - axis * m0; nn = nn + wr * wr * vec4<f32>(0.0, 1.0, 1.0, 1.0); }
    { let wr = lamn * axis.y - axis * m1; nn = nn + wr * wr * vec4<f32>(1.0, 0.0, 1.0, 1.0); }
    { let wr = lamn * axis.z - axis * m2; nn = nn + wr * wr * vec4<f32>(1.0, 1.0, 0.0, 1.0); }
    { let wr = lamn * axis.w - axis * m3; nn = nn + wr * wr * vec4<f32>(1.0, 1.0, 1.0, 0.0); }
    var l3v = sqrt(nn / max(vec4<f32>(1.0) - a2, vec4<f32>(1e-3))) * trace;
    let d1 = max(max(diag.x, diag.y), max(diag.z, diag.w));
    let oh1 = diag == vec4<f32>(d1);
    let dr = select(diag, vec4<f32>(-1.0), oh1);
    let d2 = max(max(dr.x, dr.y), max(dr.z, dr.w));
    l3v = clamp(l3v, select(vec4<f32>(d1), vec4<f32>(d2), oh1), vec4<f32>(trace) - diag);

    let sb = max(l3v, diag) * (48.0 / 49.0) + min(l3v, diag) * (8.0 / 9.0);
    let smax = max(max(sb.x, sb.y), max(sb.z, sb.w));
    use4 = smax - N4 > lam * (224.0 / 225.0);
    if (sb.y == smax) { ch = 1u; }
    if (sb.z == smax) { ch = 2u; }
    if (sb.w == smax) { ch = 3u; }
    let ohc = select(vec4<f32>(0.0), vec4<f32>(1.0), vec4<u32>(ch) == vec4<u32>(0u, 1u, 2u, 3u));
    idx1 = dot(l3v - diag, ohc) > 0.0;

    let col = select(select(select(m0, m1, ch == 1u), m2, ch == 2u), m3, ch == 3u);
    cmask = select(vec4<f32>(1.0), vec4<f32>(1.0) - ohc, use4);
    var v = select(axis, (lamn * axis - dot(axis, ohc) * col) * cmask + (hi - lo) * cmask * (1e-3 / 255.0), use4);
    v = v * inverseSqrt(max(dot(v, v), 1e-12));
    v = vec4<f32>(dot(m0, v), dot(m1, v), dot(m2, v), dot(m3, v)) * cmask;
    let vv = dot(v, v);
    axisF = select(vec4<f32>(0.0), v * inverseSqrt(max(vv, 1e-12)), vv > 1e-6);
  }

  // mode 4 关闭：原型里的 mode-4 打包路径没有做过解码对拍，且与 mode 6 共用
  // 端点量化器；直接启用会让 mode 6 的端点按 5-bit 量化（实测 8×8 块 7.96 dB）。
  // v0.1 素材全部不透明（实测 0 张带 alpha），质量门禁由 mode 1/6 达成，
  // 因此这里保留原型行为；启用前需要为 mode 4 单独做量化与解码验证。
  use4 = false;

  // Mode-4 scalar plane: 6-bit codes at the channel's exact extremes, index
  // map v = ⌊d·ks + os⌋. chs = 0 leaves ks = 0 for mode 6.
  let chs = vec4<f32>(1.0) - cmask;
  let Ls = select(7.0, 3.0, idx1);
  var A0 = u32(floor(dot(lo, chs) * (63.0 / 255.0) + 0.5));
  var A1 = u32(floor(dot(hi, chs) * (63.0 / 255.0) + 0.5));
  var ks = vec4<f32>(0.0);
  var os = 0.0;
  {
    let d0a = f32((A0 << 2u) | (A0 >> 4u));
    let d1a = f32((A1 << 2u) | (A1 >> 4u));
    let aspan = d1a - d0a;
    if (aspan > 0.0) {
      let sca = Ls / aspan;
      ks = chs * sca;
      os = (dot(p0v, chs) - d0a) * sca + 0.5;
    }
    // Anchor rule up front: pixel 0 (the d-space origin) indexes at ⌊os⌋.
    if (floor(os) >= (Ls + 1.0) * 0.5) {
      let t = A0; A0 = A1; A1 = t;
      ks = -ks;
      os = Ls + 1.0 - os;
    }
  }

  // ONE extents pass along the fit axis: projections kept for the colour
  // indices, scalar-plane indices ride along as float nibble fields.
  var tv: array<f32, 16>;
  var t_min = 1e30;
  var t_max = -1e30;
  var ga = 0.0;
  var gb = 0.0;
  var gc = 0.0;
  var w3 = 1.0;
  for (var k: u32 = 0u; k < 16u; k = k + 1u) {
    let t = dot(pixels[k], axisF);
    tv[k] = t;
    t_min = min(t_min, t);
    t_max = max(t_max, t);
    var v = clamp(floor(dot(pixels[k], ks) + os), 0.0, Ls);
    if (k == 0u) { v = min(v, floor(Ls * 0.5)); }
    if (k < 6u) { ga = ga + v * w3; } else if (k < 12u) { gb = gb + v * w3; } else { gc = gc + v * w3; }
    w3 = select(w3 * 16.0, 1.0, k == 5u || k == 11u);
  }
  let tm = dot(md, axisF);
  let mean = p0v + md;
  let seed_lo = clamp(mean + (t_min - tm) * axisF, vec4<f32>(0.0), vec4<f32>(255.0));
  let seed_hi = clamp(mean + (t_max - tm) * axisF, vec4<f32>(0.0), vec4<f32>(255.0));

  // Endpoint codes, one quantiser for both modes: mode 6 = 7-bit + p-bit
  // (per endpoint, by quantisation error), mode 4 colour = 5-bit.
  let sc = select(0.5, 31.0 / 255.0, use4);
  let cmax = select(127.0, 31.0, use4);
  let y0 = seed_lo * sc;
  let y1 = seed_hi * sc;
  let r0 = min(floor(y0 + 0.5), vec4<f32>(cmax));
  let r1 = min(floor(y1 + 0.5), vec4<f32>(cmax));
  let f0 = min(floor(y0), vec4<f32>(cmax));
  let f1 = min(floor(y1), vec4<f32>(cmax));
  let e0r = r0 - y0;
  let e0f = f0 + 0.5 - y0;
  let e1r = r1 - y1;
  let e1f = f1 + 0.5 - y1;
  let pp0 = !use4 && dot(e0f, e0f) < dot(e0r, e0r);
  let pp1 = !use4 && dot(e1f, e1f) < dot(e1r, e1r);
  let g0 = select(r0, f0, pp0);
  let g1 = select(r1, f1, pp1);
  var q0c = vec4<u32>(g0);
  var q1c = vec4<u32>(g1);
  var P0 = u32(pp0);
  var P1 = u32(pp1);
  // Decoded 8-bit endpoints: mode 6 2q + p, mode 4 q << 3 | q >> 2.
  let d0 = select(g0 * 2.0 + f32(P0), g0 * 8.0 + floor(g0 * 0.25), use4);
  let d1 = select(g1 * 2.0 + f32(P1), g1 * 8.0 + floor(g1 * 0.25), use4);
  let Lc = select(15.0, select(3.0, 7.0, idx1), use4);

  let tau0 = dot(d0 - p0v, axisF);
  let tau1 = dot(d1 - p0v, axisF);
  let span = tau1 - tau0;
  var kc = 0.0;
  var oc = 0.0;
  if (abs(span) > 0.25) {
    kc = Lc / span;
    oc = 0.5 - tau0 * kc;
  }
  // Anchor rule up front (pixel 0 projects to t = 0, index ⌊oc⌋).
  if (min(floor(oc), Lc) >= (Lc + 1.0) * 0.5) {
    let tq = q0c; q0c = q1c; q1c = tq;
    let tp = P0; P0 = P1; P1 = tp;
    oc = 0.5 + tau1 * kc;
    kc = -kc;
  }
  var sse6 = 0.0;
  var fa = 0.0;
  var fb = 0.0;
  var fc = 0.0;
  var w = 1.0;
  for (var k: u32 = 0u; k < 16u; k = k + 1u) {
    var sg = clamp(floor(tv[k] * kc + oc), 0.0, Lc);
    if (k == 0u) { sg = min(sg, floor(Lc * 0.5)); }
    let r6 = (d0 * (64.0 - sg) + d1 * sg) / 64.0;
    let e6 = r6.xyz - (pixels[k].rgb + p0v.rgb);
    sse6 = sse6 + dot(e6, e6);
    if (k < 6u) { fa = fa + sg * w; } else if (k < 12u) { fb = fb + sg * w; } else { fc = fc + sg * w; }
    w = select(w * 16.0, 1.0, k == 5u || k == 11u);
  }
  let ub = u32(fb);
  let ilo = u32(fa) | (ub << 24u);
  let ihi = (ub >> 8u) | (u32(fc) << 16u);

  // ---------- 模式 1 候选：2 子集 + 3-bit 索引（仅不透明块） ----------
  if (lo.w == 255.0) {
    var absx: array<vec3<f32>, 16>;
    for (var k: u32 = 0u; k < 16u; k = k + 1u) { absx[k] = pixels[k].rgb + p0v.rgb; }

    var keep: array<u32, 8>;
    var ksc: array<f32, 8>;
    for (var j: u32 = 0u; j < 8u; j = j + 1u) { ksc[j] = 1e30; }
    for (var pi: u32 = 0u; pi < 64u; pi = pi + 1u) {
      let mask = PART2[pi];
      // 一阶 + 平方和：SSE = Σ|p|² − |Σp|²/n，单趟即可
      var s0 = vec3<f32>(0.0);
      var s1 = vec3<f32>(0.0);
      var q0 = 0.0;
      var q1 = 0.0;
      var c0 = 0.0;
      var c1 = 0.0;
      for (var k: u32 = 0u; k < 16u; k = k + 1u) {
        let p = absx[k];
        let pp = dot(p, p);
        if (((mask >> k) & 1u) == 0u) { s0 = s0 + p; q0 = q0 + pp; c0 = c0 + 1.0; }
        else { s1 = s1 + p; q1 = q1 + pp; c1 = c1 + 1.0; }
      }
      if (c0 == 0.0 || c1 == 0.0) { continue; }
      let sc = (q0 - dot(s0, s0) / c0) + (q1 - dot(s1, s1) / c1);
      if (sc < ksc[7]) {
        var v = sc;
        var pp = pi;
        for (var j: u32 = 0u; j < 8u; j = j + 1u) {
          if (v < ksc[j]) {
            let tk = ksc[j]; ksc[j] = v; v = tk;
            let tp = keep[j]; keep[j] = pp; pp = tp;
          }
        }
      }
    }

    var bsse = sse6;
    var bpi = 0u;
    var q0a = vec3<u32>(0u); var q1a = vec3<u32>(0u); var pa = 0u;
    var q0b = vec3<u32>(0u); var q1b = vec3<u32>(0u); var pb = 0u;
    var ia: array<u32, 16>;
    var ib: array<u32, 16>;
    for (var j: u32 = 0u; j < 8u; j = j + 1u) {
      let pi = keep[j];
      let mask = PART2[pi];
      var e0a = vec3<f32>(0.0); var e1a = vec3<f32>(0.0);
      var e0b = vec3<f32>(0.0); var e1b = vec3<f32>(0.0);
      var t0 = vec3<u32>(0u); var t1 = vec3<u32>(0u); var tp = 0u;
      var idxa: array<u32, 16>;
      var idxb: array<u32, 16>;
      let sa = fit_subset(&absx, mask, 0u, &t0, &t1, &tp, &e0a, &e1a, &idxa);
      var u0 = vec3<u32>(0u); var u1 = vec3<u32>(0u); var up = 0u;
      let sb = fit_subset(&absx, mask, 1u, &u0, &u1, &up, &e0b, &e1b, &idxb);
      if (sa + sb < bsse) {
        bsse = sa + sb;
        bpi = pi;
        q0a = t0; q1a = t1; pa = tp;
        q0b = u0; q1b = u1; pb = up;
        for (var k: u32 = 0u; k < 16u; k = k + 1u) { ia[k] = idxa[k]; ib[k] = idxb[k]; }
      }
    }

    if (bsse < sse6) {
      let mask = PART2[bpi];
      let a1 = ANCH1[bpi];
      var ca0 = vec3<u32>(q0a);
      var ca1 = vec3<u32>(q1a);
      var cb0 = vec3<u32>(q0b);
      var cb1 = vec3<u32>(q1b);
      // 锚点索引高位必须为 0：反相该子集索引并交换端点
      if (ia[0] >= 4u) {
        for (var k: u32 = 0u; k < 16u; k = k + 1u) {
          if (((mask >> k) & 1u) == 0u) { ia[k] = 7u - ia[k]; }
        }
        let t = ca0; ca0 = ca1; ca1 = t;
      }
      if (ib[a1] >= 4u) {
        for (var k: u32 = 0u; k < 16u; k = k + 1u) {
          if (((mask >> k) & 1u) == 1u) { ib[k] = 7u - ib[k]; }
        }
        let t = cb0; cb0 = cb1; cb1 = t;
      }
      var bf: array<u32, 4>;
      put(&bf, 0u, 2u, 2u);
      put(&bf, 2u, 6u, bpi);
      // 端点字段按「通道在外、端点在里」排列：R(s0lo,s0hi,s1lo,s1hi) G(...) B(...)
      put(&bf, 8u, 6u, ca0.x); put(&bf, 14u, 6u, ca1.x);
      put(&bf, 20u, 6u, cb0.x); put(&bf, 26u, 6u, cb1.x);
      put(&bf, 32u, 6u, ca0.y); put(&bf, 38u, 6u, ca1.y);
      put(&bf, 44u, 6u, cb0.y); put(&bf, 50u, 6u, cb1.y);
      put(&bf, 56u, 6u, ca0.z); put(&bf, 62u, 6u, ca1.z);
      put(&bf, 68u, 6u, cb0.z); put(&bf, 74u, 6u, cb1.z);
      put(&bf, 80u, 1u, pa);
      put(&bf, 81u, 1u, pb);
      var pos = 82u;
      for (var k: u32 = 0u; k < 16u; k = k + 1u) {
        let sub = (mask >> k) & 1u;
        let v = select(ia[k], ib[k], sub == 1u);
        let n = select(3u, 2u, k == 0u || k == a1);
        put(&bf, pos, n, v);
        pos = pos + n;
      }
      let om = block_index * 4u;
      dst[om] = bf[0];
      dst[om + 1u] = bf[1];
      dst[om + 2u] = bf[2];
      dst[om + 3u] = bf[3];
      return;
    }
  }

  let o = block_index * 4u;
  if (!use4) {
    dst[o] = 0x40u | (q0c.x << 7u) | (q1c.x << 14u) | (q0c.y << 21u) | (q1c.y << 28u);
    dst[o + 1u] = (q1c.y >> 4u) | (q0c.z << 3u) | (q1c.z << 10u) | (q0c.w << 17u) | (q1c.w << 24u) | (P0 << 31u);
    dst[o + 2u] = P1 | ((ilo & 0x7u) << 1u) | (ilo & 0xFFFFFFF0u);
    dst[o + 3u] = ihi;
  } else {
    let vb = u32(gb);
    let slo = u32(ga) | (vb << 24u);
    let shi = (vb >> 8u) | (u32(gc) << 16u);
    let c2 = compact2(select(ilo, slo, idx1)) | (compact2(select(ihi, shi, idx1)) << 16u);
    let iA = compact3(select(slo, ilo, idx1));
    let iB = compact3(select(shi, ihi, idx1));
    let R0 = select(q0c.x, q0c.w, ch == 0u);
    let G0 = select(q0c.y, q0c.w, ch == 1u);
    let B0 = select(q0c.z, q0c.w, ch == 2u);
    let R1 = select(q1c.x, q1c.w, ch == 0u);
    let G1 = select(q1c.y, q1c.w, ch == 1u);
    let B1 = select(q1c.z, q1c.w, ch == 2u);
    let rot = (ch + 1u) & 3u;
    let field2 = (c2 & 1u) | ((c2 >> 2u) << 1u);
    let f_lo = (iA & 3u) | ((iA >> 3u) << 2u) | (iB << 23u);
    let f_hi = iB >> 9u;
    dst[o] = 0x10u | (rot << 5u) | (u32(idx1) << 7u) | (R0 << 8u) | (R1 << 13u) | (G0 << 18u) | (G1 << 23u) | (B0 << 28u);
    dst[o + 1u] = (B0 >> 4u) | (B1 << 1u) | (A0 << 6u) | (A1 << 12u) | ((field2 & 0x3FFFu) << 18u);
    dst[o + 2u] = (field2 >> 14u) | (f_lo << 17u);
    dst[o + 3u] = (f_lo >> 15u) | (f_hi << 17u);
  }
}
