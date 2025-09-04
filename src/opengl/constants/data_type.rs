#[derive(Debug, Clone, Copy)]
pub enum DataType {
    UByte332,
    UByte233Rev,
    UShort565,
    UShort4444,
    UShort5551,
    UShort565Rev,
    UShort4444Rev,
    UShort1555Rev,
    UInt8888,
    UInt1010102,
    UInt8888Rev,
    UInt2101010Rev,
    Double,
    Float,
    Short,
    Byte,
    Int,
    UShort,
    UByte,
    UInt,
}

impl DataType {
    pub fn value(&self) -> u32 {
        use gl::*;
        match self {
            DataType::UByte332 => UNSIGNED_BYTE_3_3_2,
            DataType::UByte233Rev => UNSIGNED_BYTE_2_3_3_REV,
            DataType::UShort565 => UNSIGNED_SHORT_5_6_5,
            DataType::UShort4444 => UNSIGNED_SHORT_4_4_4_4,
            DataType::UShort5551 => UNSIGNED_SHORT_5_5_5_1,
            DataType::UShort565Rev => UNSIGNED_SHORT_5_6_5_REV,
            DataType::UShort4444Rev => UNSIGNED_SHORT_4_4_4_4_REV,
            DataType::UShort1555Rev => UNSIGNED_SHORT_1_5_5_5_REV,
            DataType::UInt8888 => UNSIGNED_INT_8_8_8_8,
            DataType::UInt1010102 => UNSIGNED_INT_10_10_10_2,
            DataType::UInt8888Rev => UNSIGNED_INT_8_8_8_8_REV,
            DataType::UInt2101010Rev => UNSIGNED_INT_2_10_10_10_REV,
            DataType::Double => DOUBLE,
            DataType::Float => FLOAT,
            DataType::Short => SHORT,
            DataType::Byte => BYTE,
            DataType::Int => INT,
            DataType::UShort => UNSIGNED_SHORT,
            DataType::UByte => UNSIGNED_BYTE,
            DataType::UInt => UNSIGNED_INT,
        }
    }

    pub fn bytes(&self) -> usize {
        // Changed from u32 to usize
        match self {
            DataType::UByte332 | DataType::UByte233Rev | DataType::UByte | DataType::Byte => 1,
            DataType::UShort565
            | DataType::UShort4444
            | DataType::UShort5551
            | DataType::UShort565Rev
            | DataType::UShort4444Rev
            | DataType::UShort1555Rev
            | DataType::UShort
            | DataType::Short => 2,
            DataType::UInt8888
            | DataType::UInt1010102
            | DataType::UInt8888Rev
            | DataType::UInt2101010Rev
            | DataType::Int
            | DataType::UInt
            | DataType::Float => 4,
            DataType::Double => 8,
        }
    }
}
