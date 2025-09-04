pub enum VboAccess {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

impl VboAccess {
    pub fn value(&self) -> u32 {
        use gl::*;
        match self {
            VboAccess::ReadOnly => READ_ONLY,
            VboAccess::WriteOnly => WRITE_ONLY,
            VboAccess::ReadWrite => READ_WRITE,
        }
    }
}

