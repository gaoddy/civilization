//! GPU BC7 执行层（wgpu / Metal）。
//!
//! 一次 submit 处理一批图：每张图一个大纹理 + 一个输出 storage buffer，
//! 批量内多张图只是同一个 command encoder 里多几个 dispatch，最后统一回读。
//! 批次按输入字节数切块（`CHUNK_BYTES`），避免全量素材同时驻留显存。

use super::{EncodeError, Encoder};
use crate::Format;

const SHADER: &str = include_str!("../shaders/bc7-standard.wgsl");
/// 单批上传上限：超过就切下一批。
const CHUNK_BYTES: usize = 96 << 20;
/// `queue.write_texture` 的 `bytes_per_row` 对齐要求。
const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

#[derive(Debug)]
pub enum GpuError {
    NoAdapter,
    Device(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::NoAdapter => write!(f, "未找到可用的 GPU adapter（需要 Metal）"),
            GpuError::Device(e) => write!(f, "创建设备失败：{e}"),
        }
    }
}

impl std::error::Error for GpuError {}

pub struct GpuEncoder {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
}

impl GpuEncoder {
    pub fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|_| GpuError::NoAdapter)?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("asset-compress"),
            ..Default::default()
        }))
        .map_err(|e| GpuError::Device(e.to_string()))?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bc7-standard"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("bc7-encode"),
            layout: None,
            module: &module,
            entry_point: Some("encode"),
            compilation_options: Default::default(),
            cache: None,
        });
        Ok(Self {
            device,
            queue,
            pipeline,
        })
    }

    /// 批量编码；返回顺序与输入一致。非 4 倍数尺寸先按边缘复制补齐。
    pub fn encode_batch(&self, items: &[(&[u8], u32, u32)]) -> Vec<Vec<u8>> {
        let mut out = Vec::with_capacity(items.len());
        let mut start = 0;
        while start < items.len() {
            let mut end = start;
            let mut bytes = 0;
            while end < items.len() && (end == start || bytes + items[end].0.len() <= CHUNK_BYTES) {
                bytes += items[end].0.len();
                end += 1;
            }
            out.extend(self.encode_chunk(&items[start..end]));
            start = end;
        }
        out
    }

    fn encode_chunk(&self, items: &[(&[u8], u32, u32)]) -> Vec<Vec<u8>> {
        struct Job {
            texture: wgpu::Texture,
            dst: wgpu::Buffer,
            staging: wgpu::Buffer,
            ubo: wgpu::Buffer,
            out_len: usize,
            blocks_x: u32,
            blocks_y: u32,
        }
        let mut jobs = Vec::with_capacity(items.len());
        for &(rgba, w, h) in items {
            assert!(w > 0 && h > 0, "GPU 编码要求非零尺寸：{w}x{h}");
            assert_eq!(rgba.len(), w as usize * h as usize * 4, "{w}x{h} 像素长度不符");
            // 补齐后的尺寸才是上传与分块的依据；补齐像素不进入纹理。
            let padded = super::pad_to_blocks(rgba, w, h);
            let (rgba, w, h) = match &padded {
                Some((d, pw, ph)) => (d.as_slice(), *pw, *ph),
                None => (rgba, w, h),
            };
            let blocks_x = w.div_ceil(4);
            let blocks_y = h.div_ceil(4);
            let out_len = (blocks_x * blocks_y * 16) as usize;

            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bc7-src"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let row = w * 4;
            let padded = row.div_ceil(ROW_ALIGN) * ROW_ALIGN;
            if padded != row {
                let mut buf = vec![0u8; padded as usize * h as usize];
                for y in 0..h as usize {
                    let src = &rgba[y * row as usize..(y + 1) * row as usize];
                    buf[y * padded as usize..y * padded as usize + row as usize].copy_from_slice(src);
                }
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &buf,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(h),
                    },
                    wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                );
            } else {
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    rgba,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(row),
                        rows_per_image: Some(h),
                    },
                    wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                );
            }

            let dst = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bc7-dst"),
                size: out_len as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bc7-staging"),
                size: out_len as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut params = [0u8; 32];
            for (i, v) in [blocks_x, blocks_y, w, h, 0u32].iter().enumerate() {
                params[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            let ubo = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bc7-params"),
                size: 32,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&ubo, 0, &params);

            jobs.push(Job {
                texture,
                dst,
                staging,
                ubo,
                out_len,
                blocks_x,
                blocks_y,
            });
        }

        let bgl = self.pipeline.get_bind_group_layout(0);
        let mut enc = self.device.create_command_encoder(&Default::default());
        for job in &jobs {
            let view = job.texture.create_view(&Default::default());
            let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bc7"),
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: job.dst.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: job.ubo.as_entire_binding(),
                    },
                ],
            });
            {
                let mut pass = enc.begin_compute_pass(&Default::default());
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(job.blocks_x.div_ceil(8), job.blocks_y.div_ceil(8), 1);
            }
            enc.copy_buffer_to_buffer(&job.dst, 0, &job.staging, 0, job.out_len as u64);
        }
        self.queue.submit(Some(enc.finish()));

        let mut mapped = Vec::with_capacity(jobs.len());
        for job in &jobs {
            let slice = job.staging.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            mapped.push(slice);
        }
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();

        jobs.iter()
            .zip(mapped.iter())
            .map(|(job, slice)| {
                let data = slice.get_mapped_range().expect("mapped").to_vec();
                assert_eq!(data.len(), job.out_len);
                data
            })
            .collect()
    }
}

impl Encoder for GpuEncoder {
    fn encode_batch(
        &self,
        items: &[(&[u8], u32, u32)],
        fmt: Format,
    ) -> Vec<Result<Vec<u8>, EncodeError>> {
        match fmt {
            Format::Bc7 => {
                let fits = |&(rgba, w, h): &(&[u8], u32, u32)| {
                    w > 0 && h > 0 && rgba.len() == w as usize * h as usize * 4
                };
                let valid: Vec<(&[u8], u32, u32)> =
                    items.iter().copied().filter(fits).collect();
                let mut encoded = GpuEncoder::encode_batch(self, &valid).into_iter();
                items
                    .iter()
                    .map(|&(rgba, w, h)| {
                        let ok = fits(&(rgba, w, h));
                        if ok {
                            Ok(encoded.next().expect("批量输出与输入一一对应"))
                        } else {
                            Err(EncodeError::BadSize { w, h })
                        }
                    })
                    .collect()
            }
            Format::Astc4x4 => items
                .iter()
                .map(|_| Err(EncodeError::Astc("GPU 层只负责 BC7".into())))
                .collect(),
        }
    }

    fn encode(&self, rgba: &[u8], w: u32, h: u32, fmt: Format) -> Result<Vec<u8>, EncodeError> {
        match fmt {
            Format::Bc7 => {
                if w == 0 || h == 0 || rgba.len() != w as usize * h as usize * 4 {
                    return Err(EncodeError::BadSize { w, h });
                }
                Ok(self.encode_batch(&[(rgba, w, h)]).pop().unwrap())
            }
            Format::Astc4x4 => Err(EncodeError::Astc("GPU 层只负责 BC7".into())),
        }
    }
}
