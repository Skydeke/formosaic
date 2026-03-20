use crate::{
    engine::{
        architecture::models::material::Material,
        rendering::abstracted::renderable::Renderable,
    },
    opengl::{
        constants::{data_type::DataType, render_mode::RenderMode, vbo_usage::VboUsage},
        objects::{
            attribute::Attribute,
            data_buffer::DataBuffer,
            vao::Vao,
        },
    },
};
use cgmath::Vector3;
use std::rc::Rc;

pub struct Mesh {
    /// De-indexed vertex positions: 3 floats per vertex, 3 vertices per triangle,
    /// laid out as [t0v0x, t0v0y, t0v0z, t0v1x, ..., t1v0x, ...].
    /// Every triangle owns its own 3 vertices — no sharing.
    positions: Vec<f32>,

    /// Per-vertex scramble offsets, same layout as `positions`.
    /// Added on top of `positions` during upload; zero = unscrambled.
    scramble_offsets: Vec<f32>,

    /// Shared handle to the position VBO for re-uploads (scramble + animation).
    pos_buffer: Option<Rc<DataBuffer>>,

    // Legacy field for `lowest()` on the old `from_vao()` path.
    vert: Vec<f32>,
    attributes: Vec<Attribute>,

    vao: Vao,
    material: Option<Material>,
}

impl Mesh {
    /// Build a `Mesh` from raw indexed geometry.
    ///
    /// The geometry is **de-indexed** on construction: every triangle gets its
    /// own three private vertices (positions, normals, texcoords).  This is
    /// required so that `scramble_along_axis` can displace each triangle as a
    /// rigid independent unit without vertex-sharing causing tearing or
    /// accumulated offsets.
    pub fn from_raw(
        positions: Vec<f32>,
        normals: Vec<f32>,
        texcoords: Vec<f32>,
        indices: Vec<u32>,
    ) -> Self {
        let tri_count = indices.len() / 3;
        let vert_count = tri_count * 3; // one unique vertex per corner

        // ── De-index: expand every attribute so no vertex is shared ──────────
        let mut flat_pos  = Vec::with_capacity(vert_count * 3);
        let mut flat_norm = Vec::with_capacity(if normals.is_empty()   { 0 } else { vert_count * 3 });
        let mut flat_tex  = Vec::with_capacity(if texcoords.is_empty() { 0 } else { vert_count * 2 });

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
            }
        }

        // ── Build VAO — no index buffer needed ───────────────────────────────
        let mut pos_buf = DataBuffer::new(VboUsage::DynamicDraw);
        pos_buf.allocate_float(flat_pos.len());
        pos_buf.store_float(0, &flat_pos);
        let pos_buffer = Rc::new(pos_buf);

        let mut vao = Vao::create();
        let pos_attr = Attribute::of(0, 3, DataType::Float, false);
        vao.load_data_buffer(pos_buffer.clone(), &[pos_attr]);

        if !flat_tex.is_empty() {
            let mut tex_buf = DataBuffer::new(VboUsage::StaticDraw);
            tex_buf.allocate_float(flat_tex.len());
            tex_buf.store_float(0, &flat_tex);
            let tex_attr = Attribute::of(1, 2, DataType::Float, false);
            vao.load_data_buffer(Rc::new(tex_buf), &[tex_attr]);
        }

        if !flat_norm.is_empty() {
            let mut norm_buf = DataBuffer::new(VboUsage::StaticDraw);
            norm_buf.allocate_float(flat_norm.len());
            norm_buf.store_float(0, &flat_norm);
            let norm_attr = Attribute::of(2, 3, DataType::Float, false);
            vao.load_data_buffer(Rc::new(norm_buf), &[norm_attr]);
        }
        // No index buffer — draw with gl::DrawArrays via vertex count.

        let n = flat_pos.len();
        let vert = flat_pos.clone();
        Self {
            scramble_offsets: vec![0.0; n],
            positions: flat_pos,
            pos_buffer: Some(pos_buffer),
            vert,
            attributes: vec![],
            vao,
            material: None,
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
        }
    }

    /// Displace each triangle by an independent random amount along `axis`.
    ///
    /// Because the mesh is de-indexed, each triangle's 3 vertices are
    /// contiguous in `positions` (9 floats).  We write the same displacement
    /// to all 3 corners — the triangle moves as a rigid unit with no tearing.
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

        if self.positions.is_empty() {
            return;
        }

        let axis = axis.normalize();
        let mut rng = rand::rng();
        let n = self.positions.len();
        let mut offsets = vec![0.0f32; n];

        // Stride 9 floats = 3 vertices × 3 floats = one triangle.
        let tri_count = n / 9;
        for tri in 0..tri_count {
            let amount: f32 = rng.random_range(min_disp..max_disp);
            let sign: f32 = if rng.random_range(0.0f32..1.0) < 0.5 { 1.0 } else { -1.0 };
            let disp = axis * amount * sign;

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

    /// Upload positions lerped between scrambled (t=1) and original (t=0).
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

    /// Read-only access to the de-indexed vertex positions (3 f32 per vertex).
    pub fn positions(&self) -> &[f32] {
        &self.positions
    }

    pub fn set_material(&mut self, mat: Material) {
        self.material = Some(mat);
    }

    pub fn material(&self) -> Option<&Material> {
        self.material.as_ref()
    }

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
        // De-indexed mesh: use DrawArrays, not DrawElements.
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
