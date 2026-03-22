use crate::{
    architecture::models::material::Material,
    rendering::abstracted::renderable::Renderable,
    opengl::{
        constants::{data_type::DataType, render_mode::RenderMode, vbo_usage::VboUsage},
        objects::{attribute::Attribute, data_buffer::DataBuffer, vao::Vao},
    },
};
use cgmath::Vector3;
use std::rc::Rc;

pub struct Mesh {
    positions:        Vec<f32>,
    scramble_offsets: Vec<f32>,
    pos_buffer:       Option<Rc<DataBuffer>>,
    vert:             Vec<f32>,
    attributes:       Vec<Attribute>,
    vao:              Vao,
    material:         Option<Material>,
    has_vertex_colors: bool,
}

impl Mesh {
    /// Build a `Mesh` from raw indexed geometry.
    ///
    /// `colors` is an optional flat RGBA array (4 f32 per vertex, indexed same
    /// as `positions`).  Pass an empty `Vec` when the mesh has no vertex colors.
    pub fn from_raw(
        positions: Vec<f32>,
        normals:   Vec<f32>,
        texcoords: Vec<f32>,
        indices:   Vec<u32>,
        colors:    Vec<f32>,
    ) -> Self {
        let tri_count  = indices.len() / 3;
        let vert_count = tri_count * 3;

        let mut flat_pos    = Vec::with_capacity(vert_count * 3);
        let mut flat_norm   = Vec::with_capacity(if normals.is_empty()   { 0 } else { vert_count * 3 });
        let mut flat_tex    = Vec::with_capacity(if texcoords.is_empty() { 0 } else { vert_count * 2 });
        let mut flat_colors = Vec::with_capacity(if colors.is_empty()    { 0 } else { vert_count * 4 });

        for tri in 0..tri_count {
            for corner in 0..3 {
                let vi = indices[tri * 3 + corner] as usize;

                flat_pos.push(positions[vi * 3]);
                flat_pos.push(positions[vi * 3 + 1]);
                flat_pos.push(positions[vi * 3 + 2]);

                if !normals.is_empty() {
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
            }
        }

        // ── Build VAO ────────────────────────────────────────────────────────

        // location 0: positions (dynamic — scramble re-uploads here)
        let mut pos_buf = DataBuffer::new(VboUsage::DynamicDraw);
        pos_buf.allocate_float(flat_pos.len());
        pos_buf.store_float(0, &flat_pos);
        let pos_buffer = Rc::new(pos_buf);

        let mut vao = Vao::create();
        vao.load_data_buffer(pos_buffer.clone(), &[Attribute::of(0, 3, DataType::Float, false)]);

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

        let n    = flat_pos.len();
        let vert = flat_pos.clone();
        Self {
            scramble_offsets: vec![0.0; n],
            positions: flat_pos,
            pos_buffer: Some(pos_buffer),
            vert,
            attributes: vec![],
            vao,
            material: None,
            has_vertex_colors,
        }
    }

    /// Legacy constructor kept for compatibility.
    pub fn from_vao(vao: Vao) -> Self {
        Self {
            positions: vec![],
            scramble_offsets: vec![],
            pos_buffer: None,
            vert: vec![],
            attributes: vec![],
            vao,
            material: None,
            has_vertex_colors: false,
        }
    }

    pub fn scramble_along_axis(&mut self, axis: Vector3<f32>, min_disp: f32, max_disp: f32) {
        use cgmath::InnerSpace;
        use rand::Rng;

        let pos_buffer = match &self.pos_buffer {
            Some(b) => b.clone(),
            None => {
                log::warn!("[Mesh] scramble_along_axis on legacy mesh — skipping.");
                return;
            }
        };

        if self.positions.is_empty() { return; }

        let axis = axis.normalize();
        let mut rng = rand::rng();
        let n = self.positions.len();
        let mut offsets = vec![0.0f32; n];

        let tri_count = n / 9;
        for tri in 0..tri_count {
            let amount: f32 = rng.random_range(min_disp..max_disp);
            let disp = axis * amount;
            let base = tri * 9;
            for corner in 0..3 {
                let v = base + corner * 3;
                offsets[v]     = disp.x;
                offsets[v + 1] = disp.y;
                offsets[v + 2] = disp.z;
            }
        }

        self.scramble_offsets = offsets;
        self.upload_at_t(1.0, &pos_buffer);
    }

    pub fn upload_lerp(&self, t: f32) {
        if let Some(buf) = &self.pos_buffer {
            self.upload_at_t(t, buf);
        }
    }

    fn upload_at_t(&self, t: f32, buf: &Rc<DataBuffer>) {
        let n = self.positions.len();
        let mut data = vec![0.0f32; n];
        for i in 0..n {
            data[i] = self.positions[i] + self.scramble_offsets[i] * t;
        }
        buf.store_float_shared(0, &data);
    }

    pub fn lowest(&self) -> f32 {
        let mut ret = f32::INFINITY;
        let stride = self.attributes.first().map_or(3, |a| a.bytes_per_vertex() / 4);
        let data = if !self.positions.is_empty() { &self.positions } else { &self.vert };
        if stride == 0 || data.is_empty() { return ret; }
        for i in (1..data.len()).step_by(stride) {
            if data[i] < ret { ret = data[i]; }
        }
        ret
    }

    pub fn positions(&self)          -> &[f32]         { &self.positions }
    pub fn has_vertex_colors(&self)  -> bool           { self.has_vertex_colors }
    pub fn set_material(&mut self, mat: Material)      { self.material = Some(mat); }
    pub fn material(&self)           -> Option<&Material> { self.material.as_ref() }

    pub fn delete(&mut self, delete_vbos: bool) {
        self.vao.delete(delete_vbos);
    }
}

impl Renderable for Mesh {
    fn bind(&mut self) {
        self.vao.bind();
        self.vao.enable_attributes();
    }

    fn render(&self, render_mode: RenderMode) {
        let vert_count = (self.positions.len() / 3) as i32;
        if vert_count > 0 {
            unsafe { gl::DrawArrays(render_mode.value(), 0, vert_count); }
        }
    }
}

impl Mesh {
    pub fn unbind(&self) { self.vao.unbind(); }
}
