use crate::{
    architecture::models::material::Material,
    opengl::{
        constants::{data_type::DataType, render_mode::RenderMode, vbo_usage::VboUsage},
        objects::{attribute::Attribute, data_buffer::DataBuffer, vao::Vao},
    },
    rendering::abstracted::renderable::Renderable,
};
use cgmath::{InnerSpace, Vector3};
use std::rc::Rc;

pub struct Mesh {
    positions: Vec<f32>,
    pos_buffer: Option<Rc<DataBuffer>>,
    vert: Vec<f32>,
    attributes: Vec<Attribute>,
    vao: Vao,
    material: Option<Material>,
    has_vertex_colors: bool,
    bone_indices: Vec<[i32; 4]>,
    bone_weights: Vec<[f32; 4]>,
    is_skinned: bool,
}

impl Mesh {
    /// Build a `Mesh` from raw indexed geometry.
    ///
    /// `colors` is an optional flat RGBA array (4 f32 per vertex, indexed same
    /// as `positions`).  Pass an empty `Vec` when the mesh has no vertex colors.
    pub fn from_raw(
        positions: Vec<f32>,
        normals: Vec<f32>,
        texcoords: Vec<f32>,
        indices: Vec<u32>,
        colors: Vec<f32>,
        bone_indices: Vec<[i32; 4]>,
        bone_weights: Vec<[f32; 4]>,
    ) -> Self {
        let tri_count = indices.len() / 3;
        let vert_count = tri_count * 3;

        let mut flat_pos = Vec::with_capacity(vert_count * 3);
        let mut flat_norm = Vec::with_capacity(vert_count * 3);
        let mut flat_tex = Vec::with_capacity(if texcoords.is_empty() {
            0
        } else {
            vert_count * 2
        });
        let mut flat_colors =
            Vec::with_capacity(if colors.is_empty() { 0 } else { vert_count * 4 });
        let has_bones = bone_indices
            .iter()
            .any(|indices| indices.iter().any(|&idx| idx >= 0));
        let mut flat_bone_indices = Vec::with_capacity(if has_bones { vert_count } else { 0 });
        let mut flat_bone_weights = Vec::with_capacity(if has_bones { vert_count } else { 0 });
        let use_face_normals = normals.is_empty();

        for tri in 0..tri_count {
            let i0 = indices[tri * 3] as usize;
            let i1 = indices[tri * 3 + 1] as usize;
            let i2 = indices[tri * 3 + 2] as usize;

            let face_normal = if use_face_normals {
                let p0 = Vector3::new(
                    positions[i0 * 3],
                    positions[i0 * 3 + 1],
                    positions[i0 * 3 + 2],
                );
                let p1 = Vector3::new(
                    positions[i1 * 3],
                    positions[i1 * 3 + 1],
                    positions[i1 * 3 + 2],
                );
                let p2 = Vector3::new(
                    positions[i2 * 3],
                    positions[i2 * 3 + 1],
                    positions[i2 * 3 + 2],
                );

                let n = (p1 - p0).cross(p2 - p0);
                if n.magnitude2() > 0.0 {
                    n.normalize()
                } else {
                    Vector3::new(0.0, 1.0, 0.0)
                }
            } else {
                Vector3::new(0.0, 0.0, 0.0)
            };

            for corner in 0..3 {
                let vi = indices[tri * 3 + corner] as usize;

                flat_pos.push(positions[vi * 3]);
                flat_pos.push(positions[vi * 3 + 1]);
                flat_pos.push(positions[vi * 3 + 2]);

                if use_face_normals {
                    flat_norm.push(face_normal.x);
                    flat_norm.push(face_normal.y);
                    flat_norm.push(face_normal.z);
                } else {
                    flat_norm.push(normals[vi * 3]);
                    flat_norm.push(normals[vi * 3 + 1]);
                    flat_norm.push(normals[vi * 3 + 2]);
                }
                if !texcoords.is_empty() {
                    flat_tex.push(texcoords[vi * 2]);
                    flat_tex.push(texcoords[vi * 2 + 1]);
                }
                if !colors.is_empty() {
                    flat_colors.push(colors[vi * 4]);
                    flat_colors.push(colors[vi * 4 + 1]);
                    flat_colors.push(colors[vi * 4 + 2]);
                    flat_colors.push(colors[vi * 4 + 3]);
                }
                if has_bones {
                    flat_bone_indices.push(bone_indices[vi]);
                    flat_bone_weights.push(bone_weights[vi]);
                }
            }
        }

        // ── Build VAO ────────────────────────────────────────────────────────

        // location 0: positions (dynamic — displacement re-uploads here)
        let mut pos_buf = DataBuffer::new(VboUsage::DynamicDraw);
        pos_buf.allocate_float(flat_pos.len());
        pos_buf.store_float(0, &flat_pos);
        let pos_buffer = Rc::new(pos_buf);

        let mut vao = Vao::create();
        vao.load_data_buffer(
            pos_buffer.clone(),
            &[Attribute::of(0, 3, DataType::Float, false)],
        );

        // location 1: UVs
        if !flat_tex.is_empty() {
            let mut buf = DataBuffer::new(VboUsage::StaticDraw);
            buf.allocate_float(flat_tex.len());
            buf.store_float(0, &flat_tex);
            vao.load_data_buffer(Rc::new(buf), &[Attribute::of(1, 2, DataType::Float, false)]);
        }

        // location 2: normals
        if !flat_norm.is_empty() {
            let mut buf = DataBuffer::new(VboUsage::StaticDraw);
            buf.allocate_float(flat_norm.len());
            buf.store_float(0, &flat_norm);
            vao.load_data_buffer(Rc::new(buf), &[Attribute::of(2, 3, DataType::Float, false)]);
        }

        // location 5: vertex colors (rgba f32).
        // Always upload a buffer — zeros if no colors so the attribute pointer
        // is valid (avoids reading garbage when the shader samples location 5).
        let has_vertex_colors = flat_colors.len() == vert_count * 4;
        {
            let color_data = if has_vertex_colors {
                flat_colors.clone()
            } else {
                vec![0.0f32; vert_count * 4]
            };
            let mut buf = DataBuffer::new(VboUsage::StaticDraw);
            buf.allocate_float(color_data.len());
            buf.store_float(0, &color_data);
            vao.load_data_buffer(Rc::new(buf), &[Attribute::of(5, 4, DataType::Float, false)]);
        }

        // location 3: bone indices (ivec4)
        if has_bones {
            let mut buf = DataBuffer::new(VboUsage::StaticDraw);
            buf.allocate_int(flat_bone_indices.len() * 4);
            let flat: Vec<i32> = flat_bone_indices
                .iter()
                .flat_map(|arr| arr.iter())
                .copied()
                .collect();
            buf.store_int(0, &flat);
            vao.load_data_buffer(Rc::new(buf), &[Attribute::of(3, 4, DataType::Int, false)]);
        } else {
            let mut buf = DataBuffer::new(VboUsage::StaticDraw);
            let zeros: Vec<i32> = vec![0; vert_count * 4];
            buf.allocate_int(zeros.len());
            buf.store_int(0, &zeros);
            vao.load_data_buffer(Rc::new(buf), &[Attribute::of(3, 4, DataType::Int, false)]);
        }

        // location 4: bone weights (vec4)
        if has_bones {
            let mut buf = DataBuffer::new(VboUsage::StaticDraw);
            buf.allocate_float(flat_bone_weights.len() * 4);
            let flat: Vec<f32> = flat_bone_weights
                .iter()
                .flat_map(|arr| arr.iter())
                .copied()
                .collect();
            buf.store_float(0, &flat);
            vao.load_data_buffer(Rc::new(buf), &[Attribute::of(4, 4, DataType::Float, false)]);
        } else {
            let mut buf = DataBuffer::new(VboUsage::StaticDraw);
            let weights: Vec<f32> = (0..vert_count)
                .flat_map(|_| [1.0f32, 0.0, 0.0, 0.0])
                .collect();
            buf.allocate_float(weights.len());
            buf.store_float(0, &weights);
            vao.load_data_buffer(Rc::new(buf), &[Attribute::of(4, 4, DataType::Float, false)]);
        }

        Self {
            positions: flat_pos,
            pos_buffer: Some(pos_buffer),
            vert: Vec::new(),
            attributes: vec![],
            vao,
            material: None,
            has_vertex_colors,
            bone_indices: flat_bone_indices,
            bone_weights: flat_bone_weights,
            is_skinned: has_bones,
        }
    }

    /// Legacy constructor kept for compatibility.
    pub fn from_vao(vao: Vao) -> Self {
        Self {
            positions: vec![],
            pos_buffer: None,
            vert: vec![],
            attributes: vec![],
            vao,
            material: None,
            has_vertex_colors: false,
            bone_indices: vec![],
            bone_weights: vec![],
            is_skinned: false,
        }
    }

    pub fn upload_positions(&mut self, positions: Vec<f32>) {
        self.positions = positions;
        if let Some(buf) = &self.pos_buffer {
            buf.store_float_shared(0, &self.positions);
        }
    }

    pub fn lowest(&self) -> f32 {
        let mut ret = f32::INFINITY;
        let stride = self
            .attributes
            .first()
            .map_or(3, |a| a.bytes_per_vertex() / 4);
        let data = if !self.positions.is_empty() {
            &self.positions
        } else {
            &self.vert
        };
        if stride == 0 || data.is_empty() {
            return ret;
        }
        for i in (1..data.len()).step_by(stride) {
            if data[i] < ret {
                ret = data[i];
            }
        }
        ret
    }

    pub fn positions(&self) -> &[f32] {
        &self.positions
    }
    pub fn has_vertex_colors(&self) -> bool {
        self.has_vertex_colors
    }
    pub fn is_skinned(&self) -> bool {
        self.is_skinned
    }
    pub fn set_material(&mut self, mat: Material) {
        self.material = Some(mat);
    }
    pub fn material(&self) -> Option<&Material> {
        self.material.as_ref()
    }
    pub fn bone_indices(&self) -> &[[i32; 4]] {
        &self.bone_indices
    }
    pub fn bone_weights(&self) -> &[[f32; 4]] {
        &self.bone_weights
    }

    pub fn delete(&mut self, delete_vbos: bool) {
        self.vao.delete(delete_vbos);
    }
}

impl Renderable for Mesh {
    fn bind(&self) {
        self.vao.bind();
        self.vao.enable_attributes();
    }

    fn render(&self, render_mode: RenderMode) {
        let vert_count = (self.positions.len() / 3) as i32;
        if vert_count > 0 {
            unsafe {
                gl::DrawArrays(render_mode.value(), 0, vert_count);
            }
        }
    }
}

impl Mesh {
    pub fn unbind(&self) {
        self.vao.unbind();
    }
}
